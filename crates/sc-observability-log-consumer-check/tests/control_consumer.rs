//! One `init` per test binary: a consumer uses health and bounded flush through `LogControl` only.
//!
//! R-A4-005 design evidence 3. The lifecycle owner keeps the `LogGuard`; the
//! consumer code under test (`status::read_status`) sees only a `LogControl`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration test: helper fns are not covered by clippy.toml allow-*-in-tests"
)]

use std::time::Duration;

use sc_observability_log::{
    ActionName, BridgeHealthState, BridgeLifecycle, BridgeOptions, Failure, FlushFailure,
    LevelFilter, LoggerConfig, ServiceName, SinkStatus,
};
use sc_observability_log_consumer_check::status::read_status;

/// Scratch log root under the system temp dir, removed on drop (no dev-dependency needed).
struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn consumer_reads_health_and_flushes_through_control_only() {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "sc-observability-log-consumer-{}",
        std::process::id()
    )));
    let mut config = LoggerConfig::default_for(
        ServiceName::new("consumer-check").unwrap(),
        scratch.0.clone(),
    );
    config.level = LevelFilter::Info;
    config.enable_console_sink = false;
    let options = BridgeOptions {
        default_action: ActionName::new("log.record").unwrap(),
        parse_bracket_action: false,
    };
    let guard = sc_observability_log::init(config, options).unwrap();
    let control = guard.control();

    sc_observability_log::info!(target: "consumer", "record seen by the consumer");
    let running = read_status(&control, Duration::from_secs(5));
    assert_eq!(running.flush, Ok(()));
    assert_eq!(running.health.lifecycle, BridgeLifecycle::Running);
    assert_eq!(running.health.state, BridgeHealthState::Healthy);
    assert_eq!(running.health.file_sink.status, SinkStatus::Healthy);
    let path = running.health.file_sink.active_log_path.clone().unwrap();
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("record seen by the consumer")
    );

    guard.shutdown(Duration::from_secs(5)).unwrap();

    let stopped = read_status(&control, Duration::from_secs(1));
    let report = stopped.flush.unwrap_err();
    assert_eq!(report.failure, Failure::Flush(FlushFailure::ShutDown));
    assert_eq!(
        report.code,
        sc_observability_log::error_codes::SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN
    );
    assert_eq!(stopped.health.lifecycle, BridgeLifecycle::Stopped);
}
