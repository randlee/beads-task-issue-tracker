//! A lost final health snapshot is retained as a terminal waiter result.
#![cfg(feature = "test_hooks")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration fixture owns one process-global bridge"
)]

use std::time::Duration;

use sc_observability_log::{
    ActionName, BridgeOptions, LevelFilter, LoggerConfig, ServiceName, WaitError,
    fail_next_health_snapshot,
};

#[test]
fn health_snapshot_failure_notifies_and_retains_unavailable_for_all_controls() {
    let root = tempfile::tempdir().unwrap();
    let mut config = LoggerConfig::default_for(
        ServiceName::new("shutdown-snapshot-unavailable").unwrap(),
        root.path().to_path_buf(),
    );
    config.level = LevelFilter::Info;
    config.enable_console_sink = false;
    let guard = sc_observability_log::init(
        config,
        BridgeOptions {
            default_action: ActionName::new("log.record").unwrap(),
            parse_bracket_action: false,
        },
    )
    .unwrap();
    let first_control = guard.control();
    let second_control = first_control.clone();

    fail_next_health_snapshot();
    guard.shutdown(Duration::from_secs(5)).unwrap();

    let first = first_control.wait_stopped(Duration::from_secs(1));
    let second = second_control.wait_stopped(Duration::ZERO);
    for result in [first, second] {
        assert!(matches!(
            result,
            Err(WaitError::Unavailable { diagnostic })
                if diagnostic.code.as_str() == "SC_OBSERVABILITY_LOG_STATUS_UNAVAILABLE"
        ));
    }
}
