//! Cross-producer direct-admission evidence for the single guarded writer.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration fixture owns one process-global bridge"
)]

use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Duration;

use sc_observability::{RedactionPolicy, Redactor};
use sc_observability_log::{
    ActionName, AdmissionOutcome, BridgeEvent, BridgeOptions, DropCause, EmitError, EventLevel,
    FieldKeyError, LevelFilter, LogControl, LoggerConfig, ServiceName, TargetCategory,
};

static CONTROL: OnceLock<LogControl> = OnceLock::new();
static NESTED: Mutex<Vec<Result<AdmissionOutcome, EmitError>>> = Mutex::new(Vec::new());

fn event(message: &str) -> BridgeEvent {
    BridgeEvent {
        level: EventLevel::Info,
        target: TargetCategory::new("one_writer").unwrap(),
        action: None,
        message: Some(message.to_owned()),
        outcome: None,
        fields: serde_json::Map::new(),
        request_id: None,
        correlation_id: None,
        trace: None,
    }
}

struct ReenteringRedactor;

impl Redactor for ReenteringRedactor {
    fn redact(&self, key: &str, _value: &mut serde_json::Value) {
        match key {
            "nest_facade" => log::info!(target: "one_writer", "nested facade record"),
            "nest_direct" => NESTED.lock().unwrap_or_else(PoisonError::into_inner).push(
                CONTROL
                    .get()
                    .unwrap()
                    .try_log(event("nested direct record")),
            ),
            "panic" => panic!("expected fixture redactor panic"),
            _ => {}
        }
    }
}

fn counted_once(control: &LogControl, cause: DropCause, action: impl FnOnce()) {
    let before = control.health().unwrap().dropped;
    action();
    let after = control.health().unwrap().dropped;
    for other in DropCause::ALL {
        assert_eq!(
            after.get(other),
            before.get(other) + u64::from(other == cause)
        );
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one process-global fixture keeps cross-producer accounting deterministic"
)]
fn direct_facade_and_macro_share_guard_accounting_and_envelope() {
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
    let guard = sc_observability_log::init(
        config,
        BridgeOptions {
            default_action: ActionName::new("log.record").unwrap(),
            parse_bracket_action: false,
        },
    )
    .unwrap();
    let control = guard.control();
    CONTROL.set(control.clone()).unwrap();
    let path = control.active_log_path().unwrap().unwrap();

    assert_eq!(
        control.try_log(event("direct record")).unwrap(),
        AdmissionOutcome::Accepted
    );
    log::info!(target: "one_writer", "facade record");
    sc_observability_log::info!(target: "one_writer", "macro record");
    let filtered = BridgeEvent {
        level: EventLevel::Debug,
        ..event("filtered direct record")
    };
    assert_eq!(
        control.try_log(filtered).unwrap(),
        AdmissionOutcome::Filtered
    );

    let invalid = BridgeEvent {
        fields: serde_json::Map::from_iter([(
            "sc_observability_log.private".to_owned(),
            serde_json::json!(true),
        )]),
        ..event("invalid direct record")
    };
    counted_once(&control, DropCause::InvalidEvent, || {
        assert!(matches!(
            control.try_log(invalid),
            Err(EmitError::InvalidField {
                reason: FieldKeyError::ReservedPrefix,
                ..
            })
        ));
    });

    counted_once(&control, DropCause::ReentrantEmit, || {
        sc_observability_log::info!(target: "one_writer", nest_direct = true, "outer macro");
    });
    assert!(matches!(
        NESTED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_slice(),
        [Err(EmitError::Reentrant)]
    ));
    counted_once(&control, DropCause::ReentrantEmit, || {
        assert_eq!(
            control
                .try_log(BridgeEvent {
                    fields: serde_json::Map::from_iter([(
                        "nest_facade".to_owned(),
                        serde_json::json!(true),
                    )]),
                    ..event("outer direct")
                })
                .unwrap(),
            AdmissionOutcome::Accepted
        );
    });
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    counted_once(&control, DropCause::LoggerPanicked, || {
        assert!(matches!(
            control.try_log(BridgeEvent {
                fields: serde_json::Map::from_iter([
                    ("panic".to_owned(), serde_json::json!(true),)
                ]),
                ..event("panicking direct")
            }),
            Err(EmitError::Panicked)
        ));
    });
    std::panic::set_hook(old_hook);

    control.flush(Duration::from_secs(5)).unwrap();
    guard.shutdown(Duration::from_secs(5)).unwrap();
    counted_once(&control, DropCause::NotInstalled, || {
        assert!(matches!(
            control.try_log(event("post-stop direct")),
            Err(EmitError::NotRunning { .. })
        ));
    });
    let contents = std::fs::read_to_string(path).unwrap();
    for message in [
        "direct record",
        "facade record",
        "macro record",
        "outer macro",
        "outer direct",
    ] {
        assert!(contents.contains(message), "missing {message:?}");
    }
    for message in [
        "filtered direct record",
        "invalid direct record",
        "nested direct record",
        "nested facade record",
        "panicking direct",
        "post-stop direct",
    ] {
        assert!(!contents.contains(message), "unexpected {message:?}");
    }
}
