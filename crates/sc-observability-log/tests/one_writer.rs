//! One `init` per test binary: facade, macro and structured submission share one core and one writer.
//!
//! R-A4-005 design evidence 2. The `log` facade, the event macros and
//! `LogControl::submit` all reach the same `sc_observability::Logger` (one JSONL
//! file, one bridge-owned envelope), and every rejection on any of the three
//! paths is counted exactly once. A custom redactor, which runs inside the guarded
//! core, re-enters through a *different* producer and panics on demand, proving
//! that all three share a single guard. Every sub-case runs inside the single
//! test fn.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration test: helper fns are not covered by clippy.toml allow-*-in-tests"
)]

use std::path::Path;
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Duration;

use sc_observability::{RedactionPolicy, Redactor};
use sc_observability_log::{
    ActionName, BridgeOptions, DropCause, DroppedEvents, InvalidInputReason, JsonValue, Level,
    LevelFilter, LogControl, LoggerConfig, ServiceName, StructuredRecord, SubmitError,
    SubmitOutcome,
};
use serde_json::Value;

static CONTROL: OnceLock<LogControl> = OnceLock::new();
static NESTED_SUBMITS: Mutex<Vec<Result<SubmitOutcome, SubmitError>>> = Mutex::new(Vec::new());

/// Runs inside the guarded core for every field of every admitted event.
struct ReenteringRedactor;

impl Redactor for ReenteringRedactor {
    fn redact(&self, key: &str, _value: &mut JsonValue) {
        match key {
            "nest_facade" => log::info!(target: "one_writer", "nested facade record"),
            "nest_macro" => {
                sc_observability_log::info!(target: "one_writer", "nested macro record");
            }
            "nest_submit" => {
                let result = CONTROL.get().unwrap().submit(
                    StructuredRecord::new(Level::INFO, "one_writer")
                        .with_message("nested submit record"),
                );
                NESTED_SUBMITS
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(result);
            }
            "panic" => panic!("ReenteringRedactor (expected by this test)"),
            _ => {}
        }
    }
}

fn read_events(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// Runs `body` and asserts it changed exactly one counter, `cause`, by exactly one.
fn assert_counted_once(control: &LogControl, cause: DropCause, label: &str, body: impl FnOnce()) {
    let before: DroppedEvents = control.health().unwrap().dropped;
    body();
    let after = control.health().unwrap().dropped;
    for other in DropCause::ALL {
        let expected = before.get(other) + u64::from(other == cause);
        assert_eq!(after.get(other), expected, "{label}: counter {other:?}");
    }
    assert_eq!(after.total(), before.total() + 1, "{label}: total");
}

fn assert_counts_nothing(control: &LogControl, label: &str, body: impl FnOnce()) {
    let before = control.health().unwrap().dropped;
    body();
    assert_eq!(control.health().unwrap().dropped, before, "{label}");
}

fn submit(control: &LogControl, record: StructuredRecord) -> Result<SubmitOutcome, SubmitError> {
    control.submit(record)
}

fn admitted_records(control: &LogControl) {
    assert_counts_nothing(control, "three producers, no rejection", || {
        log::info!(target: "one_writer", "facade record");
        sc_observability_log::info!(target: "one_writer", "macro record");
        let accepted = submit(
            control,
            StructuredRecord::new(Level::INFO, "one_writer::ui")
                .with_action("ui.click")
                .with_message("submit record")
                .with_field("button", "save"),
        );
        assert_eq!(accepted, Ok(SubmitOutcome::Accepted));
    });
    assert_counts_nothing(control, "filtered submit", || {
        let filtered = submit(
            control,
            StructuredRecord::new(Level::DEBUG, "one_writer").with_message("filtered record"),
        );
        assert_eq!(filtered, Ok(SubmitOutcome::Filtered));
    });
}

fn rejected_input(control: &LogControl) {
    assert_counted_once(control, DropCause::InvalidEvent, "reserved key", || {
        let rejected = submit(
            control,
            StructuredRecord::new(Level::INFO, "one_writer")
                .with_message("invalid record")
                .with_field("sc_observability_log.shadowed_fields", 1),
        );
        assert_eq!(
            rejected,
            Err(SubmitError::InvalidInput(
                InvalidInputReason::ReservedFieldKey {
                    key: "sc_observability_log.shadowed_fields".to_owned()
                }
            ))
        );
    });
}

/// Each producer re-enters through another one; each nested record is counted once.
fn cross_producer_reentrancy(control: &LogControl) {
    assert_counted_once(
        control,
        DropCause::ReentrantEmit,
        "facade inside submit",
        || {
            let outer = submit(
                control,
                StructuredRecord::new(Level::INFO, "one_writer")
                    .with_message("outer submit record")
                    .with_field("nest_facade", true),
            );
            assert_eq!(outer, Ok(SubmitOutcome::Accepted));
        },
    );
    assert_counted_once(
        control,
        DropCause::ReentrantEmit,
        "submit inside macro",
        || {
            sc_observability_log::info!(target: "one_writer", nest_submit = true, "outer macro record");
        },
    );
    assert_eq!(
        NESTED_SUBMITS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_slice(),
        [Err(SubmitError::Reentrant)],
        "the nested submit reports its own rejection"
    );
    assert_counted_once(
        control,
        DropCause::ReentrantEmit,
        "macro inside facade",
        || {
            log::info!(target: "one_writer", nest_macro = true; "outer facade record");
        },
    );
}

/// A panic in the shared core (here: a redactor) is contained once on every path.
fn contained_panics(control: &LogControl) {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    assert_counted_once(control, DropCause::LoggerPanicked, "facade panic", || {
        log::info!(target: "one_writer", panic = true; "panicking facade record");
    });
    assert_counted_once(control, DropCause::LoggerPanicked, "macro panic", || {
        sc_observability_log::info!(target: "one_writer", panic = true, "panicking macro record");
    });
    assert_counted_once(control, DropCause::LoggerPanicked, "submit panic", || {
        let panicked = submit(
            control,
            StructuredRecord::new(Level::INFO, "one_writer")
                .with_message("panicking submit record")
                .with_field("panic", true),
        );
        assert_eq!(panicked, Err(SubmitError::ContainedPanic));
    });
    std::panic::set_hook(previous_hook);
}

fn assert_one_writer(path: &Path) {
    let events = read_events(path);
    let count = |message: &str| {
        events
            .iter()
            .filter(|event| event["message"] == message)
            .count()
    };
    for written in [
        "facade record",
        "macro record",
        "submit record",
        "outer submit record",
        "outer macro record",
        "outer facade record",
    ] {
        assert_eq!(count(written), 1, "{written} is written exactly once");
    }
    for dropped in [
        "filtered record",
        "invalid record",
        "nested facade record",
        "nested macro record",
        "nested submit record",
        "panicking facade record",
        "panicking macro record",
        "panicking submit record",
        "after shutdown",
    ] {
        assert_eq!(count(dropped), 0, "{dropped} is not written");
    }
    // One bridge-owned envelope for every producer.
    let pid = u64::from(std::process::id());
    let envelope = |event: &Value| {
        (
            event["version"].clone(),
            event["service"].clone(),
            event["identity"].clone(),
        )
    };
    let first = envelope(&events[0]);
    assert_eq!(first.1, "one-writer");
    assert_eq!(first.2["pid"], pid);
    assert!(events.iter().all(|event| envelope(event) == first));
    let submitted = events
        .iter()
        .find(|event| event["message"] == "submit record")
        .unwrap();
    assert_eq!(submitted["target"], "one_writer.ui");
    assert_eq!(submitted["action"], "ui.click");
    assert_eq!(submitted["fields"]["button"], "save");
    assert!(submitted["timestamp"].is_string());
    // One writer: a single JSONL file in the log directory.
    let jsonl_files = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".jsonl"))
        .count();
    assert_eq!(jsonl_files, 1);
}

#[test]
fn facade_macro_and_submit_share_one_core_and_one_writer() {
    let root = tempfile::tempdir().unwrap();
    let mut config = LoggerConfig::default_for(
        ServiceName::new("one-writer").unwrap(),
        root.path().to_path_buf(),
    );
    config.level = LevelFilter::Info;
    config.enable_console_sink = false;
    config.redaction = RedactionPolicy {
        custom_redactors: vec![Box::new(ReenteringRedactor)],
        ..RedactionPolicy::default()
    };
    let options = BridgeOptions {
        default_action: ActionName::new("log.record").unwrap(),
        parse_bracket_action: false,
    };
    let guard = sc_observability_log::init(config, options).unwrap();
    let control = guard.control();
    CONTROL.set(control.clone()).unwrap();
    let path = control.active_log_path().unwrap();

    admitted_records(&control);
    rejected_input(&control);
    cross_producer_reentrancy(&control);
    contained_panics(&control);
    // Fresh process: the counters hold exactly the rejections above.
    let dropped = control.health().unwrap().dropped;
    assert_eq!(dropped.total(), 7);
    assert_eq!(dropped.get(DropCause::ReentrantEmit), 3);
    assert_eq!(dropped.get(DropCause::LoggerPanicked), 3);
    assert_eq!(dropped.get(DropCause::InvalidEvent), 1);

    control.flush(Duration::from_secs(5)).unwrap();
    guard.shutdown(Duration::from_secs(5)).unwrap();

    assert_counted_once(
        &control,
        DropCause::NotInstalled,
        "submit after shutdown",
        || {
            let stopped = submit(
                &control,
                StructuredRecord::new(Level::ERROR, "one_writer").with_message("after shutdown"),
            );
            assert_eq!(
                stopped,
                Err(SubmitError::Stopped {
                    lifecycle: sc_observability_log::LifecyclePhase::Stopped
                })
            );
        },
    );
    assert_one_writer(&path);
}
