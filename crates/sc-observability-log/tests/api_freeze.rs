// crates/sc-observability-log/tests/api_freeze.rs — frozen after a-1 merges
//
// Intentional changes since a-1 (each approved in docs/plans/phase-a/review-a-5.md):
// - a-5 R-A4-004: `LogGuard::health`, `LogGuard::handle`, `LogHandle::health`, the
//   `BridgeHealth` snapshot types, `BRIDGE_HEALTH_SCHEMA_VERSION`, the `Timestamp`
//   re-export, and `Serialize`/`Deserialize` on `DroppedEvents`.
// - a-5 R-A4-001: `LogHandle::flush`, `FlushError::ShutDown` and its code
//   `SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN` (flush after shutdown started).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration test: helper fns are not covered by clippy.toml allow-*-in-tests"
)]
#![allow(
    clippy::items_after_statements,
    clippy::match_same_arms,
    reason = "signature lock: derive-probe fns sit beside the assertions they serve, and one arm per variant keeps each match exhaustive by name"
)]

use sc_observability_log::{
    BridgeHealth, BridgeHealthState, BridgeLifecycle, BridgeOptions, DropCause, DroppedEvents,
    FileSinkHealth, FlushError, HealthDiagnostic, InitError, LogGuard, LogHandle, LoggerConfig,
    LoggerHealth, QueueHealth, ShutdownError, SinkHealthSnapshot, SinkStatus, WriterStatus,
};
use std::time::Duration;

#[test]
fn a1_public_api_is_frozen() {
    let _: fn(LoggerConfig, BridgeOptions) -> Result<LogGuard, InitError> =
        sc_observability_log::init;
    let _: fn(&LogGuard, Duration) -> Result<(), FlushError> = LogGuard::flush;
    let _: fn(LogGuard, Duration) -> Result<(), ShutdownError> = LogGuard::shutdown;
    let _: fn(&LogGuard) -> DroppedEvents = LogGuard::dropped_events;
    let _: for<'a> fn(&'a LogGuard) -> Option<&'a std::path::Path> = LogGuard::active_log_path;
    let _: fn(&DroppedEvents, DropCause) -> u64 = DroppedEvents::get;
    let _: fn(&DroppedEvents) -> u64 = DroppedEvents::total;
    let _: fn(&InitError) -> sc_observability_log::ErrorCode = InitError::code;
    let _: fn(&InitError) -> sc_observability_log::Remediation = InitError::remediation;
    let _: fn(&FlushError) -> sc_observability_log::ErrorCode = FlushError::code;
    let _: fn(&FlushError) -> sc_observability_log::Remediation = FlushError::remediation;
    let _: fn(&ShutdownError) -> sc_observability_log::ErrorCode = ShutdownError::code;
    let _: fn(&ShutdownError) -> sc_observability_log::Remediation = ShutdownError::remediation;
    // Derives: removing one breaks compilation.
    fn dropped_events_derives<T: std::fmt::Debug + Clone + Copy + Default + PartialEq + Eq>() {}
    fn drop_cause_derives<T: std::fmt::Debug + Clone + Copy + PartialEq + Eq + std::hash::Hash>() {}
    dropped_events_derives::<DroppedEvents>();
    drop_cause_derives::<DropCause>();
    let _ = |a: sc_observability_log::ActionName| BridgeOptions {
        default_action: a,
        parse_bracket_action: true,
    };
    let _: Duration = sc_observability_log::DEFAULT_DROP_SHUTDOWN_TIMEOUT;
    let _: &[sc_observability_log::ErrorCode] = sc_observability_log::error_codes::ALL;
    // Exhaustive matches: adding, removing or reshaping a variant breaks this test.
    let _ = |e: InitError| match e {
        InitError::AlreadyInitialized => (),
        InitError::ForeignLoggerInstalled { source: _ } => (),
        InitError::IdentityResolution { source: _ } => (),
        InitError::Logger { source: _ } => (),
    };
    let _ = |e: FlushError| match e {
        FlushError::TimedOut { timeout: _ } | FlushError::Logger { source: _ } => (),
        FlushError::HelperSpawn { source: _ } | FlushError::HelperLost => (),
        FlushError::ShutDown => (),
    };
    let _ = |e: ShutdownError| match e {
        ShutdownError::TimedOut { timeout: _ } | ShutdownError::FinalFlush { source: _ } => (),
        ShutdownError::HelperSpawn { source: _ } | ShutdownError::HelperLost => (),
    };
    let _ = |c: DropCause| match c {
        DropCause::QueueFull | DropCause::InvalidEvent | DropCause::WriterDegraded => (),
        DropCause::ShutdownTimedOut | DropCause::NotInstalled => (),
        DropCause::LoggerPanicked | DropCause::ReentrantEmit => (),
    };
    let _: [DropCause; 7] = DropCause::ALL;
}

#[test]
fn a5_health_api_is_frozen() {
    let _: fn(&LogGuard) -> BridgeHealth = LogGuard::health;
    let _: fn(&LogGuard) -> LogHandle = LogGuard::handle;
    let _: fn(&LogHandle) -> BridgeHealth = LogHandle::health;
    let _: fn(&LogHandle, Duration) -> Result<(), FlushError> = LogHandle::flush;
    let _: sc_observability_log::ErrorCode =
        sc_observability_log::error_codes::SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN;
    let _: u32 = sc_observability_log::BRIDGE_HEALTH_SCHEMA_VERSION;
    fn serde_derives<T: serde::Serialize + serde::de::DeserializeOwned>() {}
    fn snapshot_derives<T: std::fmt::Debug + Clone + PartialEq>() {}
    fn state_derives<T: std::fmt::Debug + Clone + Copy + PartialEq + Eq + std::hash::Hash>() {}
    fn handle_derives<T: std::fmt::Debug + Clone + Copy + Send + Sync>() {}
    serde_derives::<DroppedEvents>();
    serde_derives::<BridgeHealth>();
    snapshot_derives::<BridgeHealth>();
    snapshot_derives::<LoggerHealth>();
    snapshot_derives::<QueueHealth>();
    snapshot_derives::<FileSinkHealth>();
    snapshot_derives::<SinkHealthSnapshot>();
    snapshot_derives::<HealthDiagnostic>();
    state_derives::<BridgeLifecycle>();
    state_derives::<BridgeHealthState>();
    state_derives::<WriterStatus>();
    state_derives::<SinkStatus>();
    handle_derives::<LogHandle>();
    // Field names and types: renaming, retyping or removing a field breaks this test.
    let _ = |h: BridgeHealth| {
        let _: (u32, BridgeLifecycle, BridgeHealthState) = (h.schema_version, h.lifecycle, h.state);
        let _: (Option<LoggerHealth>, FileSinkHealth) = (h.logger, h.file_sink);
        let _: (SinkHealthSnapshot, DroppedEvents) = (h.console_sink, h.dropped_events);
    };
    let _ = |l: LoggerHealth| {
        let _: (WriterStatus, QueueHealth) = (l.writer_state, l.queue);
        let _: (Option<HealthDiagnostic>, Option<HealthDiagnostic>) =
            (l.last_writer_error, l.last_error);
        let _: (u64, u64) = (l.dropped_events_total, l.flush_errors_total);
    };
    let _ = |q: QueueHealth| -> [u64; 4] {
        [q.depth, q.capacity, q.high_water_mark, q.full_drops_total]
    };
    let _ = |f: FileSinkHealth| {
        let _: (SinkStatus, Option<std::path::PathBuf>) = (f.status, f.active_log_path);
        let _: Option<HealthDiagnostic> = f.last_error;
    };
    let _ = |c: SinkHealthSnapshot| -> (SinkStatus, Option<HealthDiagnostic>) {
        (c.status, c.last_error)
    };
    let _ = |d: HealthDiagnostic| {
        let _: (Option<sc_observability_log::ErrorCode>, String) = (d.code, d.message);
        let _: (
            sc_observability_log::Timestamp,
            sc_observability_log::Remediation,
        ) = (d.at, d.remediation);
    };
    // Exhaustive matches over the state enums.
    let _ = |l: BridgeLifecycle| match l {
        BridgeLifecycle::Running | BridgeLifecycle::ShuttingDown | BridgeLifecycle::Stopped => (),
    };
    let _ = |s: BridgeHealthState| match s {
        BridgeHealthState::Healthy
        | BridgeHealthState::Degraded
        | BridgeHealthState::Unavailable => (),
    };
    let _ = |w: WriterStatus| match w {
        WriterStatus::Running | WriterStatus::Degraded | WriterStatus::Stopped => (),
    };
    let _ = |s: SinkStatus| match s {
        SinkStatus::Disabled | SinkStatus::Healthy => (),
        SinkStatus::DegradedDropping | SinkStatus::Unavailable => (),
    };
}
