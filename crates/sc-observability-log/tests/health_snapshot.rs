//! One `init` per test binary: `LogGuard::health` / `LogControl::health` (R-A4-004).
//!
//! Covers the running snapshot, the serialized shape and the defined result after
//! shutdown, all inside the single test fn.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration test: helper fns are not covered by clippy.toml allow-*-in-tests"
)]

use std::time::Duration;

use sc_observability_log::{
    ActionName, BRIDGE_HEALTH_SCHEMA_VERSION, BridgeHealthReport, BridgeHealthState,
    BridgeLifecycle, DropCause, LevelFilter, LoggerConfig, ServiceName, SinkStatus, WriterStatus,
};

const QUEUE_CAPACITY: usize = 64;

fn assert_running(health: &BridgeHealthReport, expected_path: &std::path::Path) {
    assert_eq!(health.schema_version, BRIDGE_HEALTH_SCHEMA_VERSION);
    assert_eq!(health.lifecycle, BridgeLifecycle::Running);
    assert_eq!(health.state, BridgeHealthState::Healthy);
    let logger = health.logger.as_ref().expect("running logger health");
    assert_eq!(logger.writer_state, WriterStatus::Running);
    assert_eq!(logger.queue.capacity, QUEUE_CAPACITY as u64);
    assert!(logger.queue.high_water_mark >= logger.queue.depth);
    assert!(logger.last_writer_error.is_none());
    assert_eq!(health.file_sink.status, SinkStatus::Healthy);
    assert_eq!(
        health.file_sink.active_log_path.as_deref(),
        Some(expected_path)
    );
    assert_eq!(health.console_sink.status, SinkStatus::Disabled);
}

fn assert_serialized_shape(health: &BridgeHealthReport) {
    let json = serde_json::to_value(health).unwrap();
    assert_eq!(json["lifecycle"], "running");
    assert_eq!(json["state"], "healthy");
    assert_eq!(json["logger"]["writer_state"], "running");
    assert_eq!(json["file_sink"]["status"], "healthy");
    assert_eq!(json["console_sink"]["status"], "disabled");
    assert!(json["dropped_events"]["not_installed"].is_u64());
    let decoded: BridgeHealthReport = serde_json::from_value(json).unwrap();
    assert_eq!(&decoded, health);
}

#[test]
fn health_snapshot_tracks_the_lifecycle() {
    let root = tempfile::tempdir().unwrap();
    let mut config = LoggerConfig::default_for(
        ServiceName::new("health-snapshot").unwrap(),
        root.path().to_path_buf(),
    );
    config.level = LevelFilter::Info;
    config.enable_console_sink = false;
    config.queue_capacity = QUEUE_CAPACITY;
    let options = sc_observability_log::BridgeOptions {
        default_action: ActionName::new("log.record").unwrap(),
        parse_bracket_action: false,
    };
    let guard = sc_observability_log::init(config, options).unwrap();
    let path = guard.active_log_path().unwrap().to_path_buf();
    let control = guard.control();
    assert_eq!(control.active_log_path().as_deref(), Some(path.as_path()));

    log::info!(target: "health", "a record before the snapshot");
    guard.flush(Duration::from_secs(5)).unwrap();

    control.flush(Duration::from_secs(5)).unwrap();
    let from_guard = guard.health();
    assert_running(&from_guard, &path);
    assert_running(&control.health(), &path);
    assert_eq!(from_guard.dropped_events, guard.dropped_events());
    assert_serialized_shape(&from_guard);

    guard.shutdown(Duration::from_secs(5)).unwrap();

    // Defined result after shutdown: the final report of the stopped logger.
    let stopped = control.health();
    assert_eq!(stopped.lifecycle, BridgeLifecycle::Stopped);
    assert_eq!(stopped.state, BridgeHealthState::Unavailable);
    let logger = stopped.logger.as_ref().expect("final logger health");
    assert_eq!(logger.writer_state, WriterStatus::Stopped);
    assert_eq!(logger.queue.depth, 0);
    assert_eq!(stopped.file_sink.status, SinkStatus::Unavailable);
    assert_eq!(
        stopped.file_sink.active_log_path.as_deref(),
        Some(path.as_path())
    );

    assert_eq!(
        control.active_log_path(),
        Some(path.clone()),
        "the owned path stays available after shutdown"
    );

    // A control outlives the guard without owning it: a late flush is rejected clearly.
    assert!(matches!(
        control.flush(Duration::from_secs(1)),
        Err(sc_observability_log::FlushError::ShutDown)
    ));

    // A record after shutdown is filtered before the bridge: not written, not counted.
    let dropped = stopped.dropped_events;
    log::error!(target: "health", "a record after shutdown");
    let after = control.health();
    assert_eq!(after.dropped_events, dropped);
    assert_eq!(after.lifecycle, BridgeLifecycle::Stopped);
    assert!(
        !std::fs::read_to_string(&path)
            .unwrap()
            .contains("a record after shutdown")
    );
    assert_eq!(dropped.get(DropCause::LoggerPanicked), 0);
}
