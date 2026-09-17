//! One `init` per test binary: a timed-out shutdown completes late, observably.
//!
//! R-A4-005 design evidence 4 (timeout versus final stop). A submission blocks
//! inside a custom redactor while it holds the installed logger, so the owner's
//! `shutdown` cannot gain sole ownership within its timeout and returns
//! `ShutdownError::TimedOut`. The lifecycle remains `ShuttingDown` (not the
//! final `Stopped`), and new submissions are rejected. Releasing the redactor lets
//! the detached shutdown helper finish: health reports `Stopped` with the stopped
//! writer's final report, and the record admitted before the late completion is
//! flushed to the file.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration test: helper fns are not covered by clippy.toml allow-*-in-tests"
)]

use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use sc_observability::{RedactionPolicy, Redactor};
use sc_observability_log::{
    ActionName, BridgeHealthState, BridgeLifecycle, BridgeOptions, JsonValue, Level, LevelFilter,
    LoggerConfig, ServiceName, ShutdownError, StructuredRecord, SubmitError, SubmitOutcome,
    WaitError, WriterStatus,
};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(150);
const LATE_COMPLETION_DEADLINE: Duration = Duration::from_secs(20);

/// `entered` fires when the redactor blocks; `release` unblocks it.
struct Gate {
    entered: SyncSender<()>,
    release: Mutex<Receiver<()>>,
}

static GATE: OnceLock<Gate> = OnceLock::new();

struct BlockingRedactor;

impl Redactor for BlockingRedactor {
    fn redact(&self, key: &str, _value: &mut JsonValue) {
        if key == "block" {
            let gate = GATE.get().unwrap();
            gate.entered.send(()).unwrap();
            gate.release.lock().unwrap().recv().unwrap();
        }
    }
}

#[test]
fn timed_out_shutdown_completes_late_and_is_observable() {
    let (entered_tx, entered_rx) = sync_channel(1);
    let (release_tx, release_rx) = sync_channel(1);
    assert!(
        GATE.set(Gate {
            entered: entered_tx,
            release: Mutex::new(release_rx),
        })
        .is_ok()
    );

    let root = tempfile::tempdir().unwrap();
    let mut config = LoggerConfig::default_for(
        ServiceName::new("shutdown-timeout").unwrap(),
        root.path().to_path_buf(),
    );
    config.level = LevelFilter::Info;
    config.enable_console_sink = false;
    config.redaction = RedactionPolicy {
        custom_redactors: vec![Box::new(BlockingRedactor)],
        ..RedactionPolicy::default()
    };
    let options = BridgeOptions {
        default_action: ActionName::new("log.record").unwrap(),
        parse_bracket_action: false,
    };
    let guard = sc_observability_log::init(config, options).unwrap();
    let control = guard.control();
    let path = control.active_log_path().unwrap();

    // A submission that holds the installed logger until released.
    let blocked_control = control.clone();
    let blocked = std::thread::spawn(move || {
        blocked_control.submit(
            StructuredRecord::new(Level::INFO, "shutdown_timeout")
                .with_message("admitted before the late completion")
                .with_field("block", true),
        )
    });
    entered_rx.recv().unwrap();

    // 1. The owner's shutdown times out.
    let started = Instant::now();
    let result = guard.shutdown(SHUTDOWN_TIMEOUT);
    assert!(
        matches!(result, Err(ShutdownError::TimedOut { timeout }) if timeout == SHUTDOWN_TIMEOUT),
        "{result:?}"
    );
    assert!(
        started.elapsed() < SHUTDOWN_TIMEOUT * 10,
        "shutdown stays bounded"
    );

    // 2. Timed out is not final: lifecycle says so, nothing is accepted, no final report yet.
    let pending = control.health();
    assert_eq!(pending.lifecycle, BridgeLifecycle::ShuttingDown);
    assert_eq!(pending.state, BridgeHealthState::Unavailable);
    assert!(
        pending.logger.is_none(),
        "no final report before completion"
    );
    assert_eq!(
        serde_json::to_value(&pending).unwrap()["lifecycle"],
        "shutting_down"
    );
    assert_eq!(
        control.submit(StructuredRecord::new(Level::ERROR, "shutdown_timeout")),
        Err(SubmitError::Stopped {
            lifecycle: BridgeLifecycle::ShuttingDown
        })
    );
    assert!(matches!(
        control.wait_stopped(Duration::from_millis(10)),
        Err(WaitError::TimedOut { .. })
    ));

    // 3. Release the blocked submission: the detached helper completes the shutdown late.
    release_tx.send(()).unwrap();
    assert_eq!(blocked.join().unwrap(), Ok(SubmitOutcome::Accepted));
    let deadline = Instant::now() + LATE_COMPLETION_DEADLINE;
    let stopped = loop {
        let health = control.health();
        if health.lifecycle == BridgeLifecycle::Stopped {
            break health;
        }
        assert!(
            Instant::now() < deadline,
            "late completion never observed: {health:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    let logger = stopped
        .logger
        .as_ref()
        .expect("final report after late completion");
    assert_eq!(logger.writer_state, WriterStatus::Stopped);
    assert_eq!(logger.queue.depth, 0);
    let first_report = control.wait_stopped(Duration::ZERO).unwrap();
    let second_report = control.wait_stopped(Duration::ZERO).unwrap();
    assert_eq!(
        serde_json::to_value(&first_report).unwrap(),
        serde_json::to_value(&second_report).unwrap(),
        "waiters observe the one retained late result rather than rebuilding it"
    );

    // The late shutdown flushed the record admitted while it was pending.
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("admitted before the late completion"));
}
