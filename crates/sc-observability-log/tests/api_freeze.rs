// crates/sc-observability-log/tests/api_freeze.rs — frozen after a-1 merges
//
// Intentional changes since a-1 (each approved in docs/plans/phase-a/review-a-5.md):
// - a-5 R-A4-004: `LogGuard::health`, the `BridgeHealthReport` snapshot types,
//   `BRIDGE_HEALTH_SCHEMA_VERSION`, the `Timestamp` re-export, and
//   `Serialize`/`Deserialize` on `DroppedEvents`.
// - a-5 R-A4-001: `FlushError::ShutDown` and its code
//   `SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN` (flush after shutdown started).
// - a-5 R-A4-005 (transferable public bridge contract): `LogGuard::control` and
//   `LogControl` (`flush`, `health`, `active_log_path`, `submit`) replacing the
//   round-1 `LogHandle`; `BridgeHealth` renamed `BridgeHealthReport`;
//   `BridgeLifecycle::ShutdownTimedOut`; `StructuredRecord`, `SubmitOutcome`,
//   `SubmitError`, `InvalidInputReason`, `JsonMap`/`JsonValue`; `FailureReport`,
//   `Failure`, `InitFailure`, `FlushFailure`, `ShutdownFailure`,
//   `CONTROL_SCHEMA_VERSION` and `report()` on every error; `Serialize`/
//   `Deserialize` on `Level`; the seven `SC_OBSERVABILITY_LOG_SUBMIT_*` codes.
//   `tests/ui/log_guard_not_clone.rs`, `tests/ui/log_control_not_owner.rs` and
//   `tests/ui/log_control_not_constructible.rs` prove at
//   compile time that neither type can become a second shutdown owner.
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
    BridgeHealthReport, BridgeHealthState, BridgeLifecycle, BridgeOptions, DropCause,
    DroppedEvents, Failure, FailureReport, FileSinkHealth, FlushError, FlushFailure,
    HealthDiagnostic, InitError, InitFailure, InvalidInputReason, JsonMap, JsonValue, Level,
    LogControl, LogGuard, LoggerConfig, LoggerHealth, QueueHealth, ShutdownError, ShutdownFailure,
    SinkHealthSnapshot, SinkStatus, StructuredRecord, SubmitError, SubmitOutcome, WriterStatus,
};
use sc_observability_log::{ErrorCode, Remediation};
use std::path::PathBuf;
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
    let _: fn(&LogGuard) -> BridgeHealthReport = LogGuard::health;
    let _: sc_observability_log::ErrorCode =
        sc_observability_log::error_codes::SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN;
    let _: u32 = sc_observability_log::BRIDGE_HEALTH_SCHEMA_VERSION;
    fn serde_derives<T: serde::Serialize + serde::de::DeserializeOwned>() {}
    fn snapshot_derives<T: std::fmt::Debug + Clone + PartialEq>() {}
    fn state_derives<T: std::fmt::Debug + Clone + Copy + PartialEq + Eq + std::hash::Hash>() {}
    serde_derives::<DroppedEvents>();
    serde_derives::<BridgeHealthReport>();
    snapshot_derives::<BridgeHealthReport>();
    snapshot_derives::<LoggerHealth>();
    snapshot_derives::<QueueHealth>();
    snapshot_derives::<FileSinkHealth>();
    snapshot_derives::<SinkHealthSnapshot>();
    snapshot_derives::<HealthDiagnostic>();
    state_derives::<BridgeLifecycle>();
    state_derives::<BridgeHealthState>();
    state_derives::<WriterStatus>();
    state_derives::<SinkStatus>();
    // Field names and types: renaming, retyping or removing a field breaks this test.
    let _ = |h: BridgeHealthReport| {
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
        BridgeLifecycle::Running | BridgeLifecycle::ShuttingDown => (),
        BridgeLifecycle::ShutdownTimedOut | BridgeLifecycle::Stopped => (),
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

#[test]
fn a5_control_api_is_frozen() {
    // Lifecycle owner vs control: every exported signature.
    let _: fn(&LogGuard) -> LogControl = LogGuard::control;
    let _: fn(&LogControl, Duration) -> Result<(), FlushError> = LogControl::flush;
    let _: fn(&LogControl) -> BridgeHealthReport = LogControl::health;
    let _: fn(&LogControl) -> Option<PathBuf> = LogControl::active_log_path;
    let _: fn(&LogControl, StructuredRecord) -> Result<SubmitOutcome, SubmitError> =
        LogControl::submit;
    fn control_derives<T: std::fmt::Debug + Clone + Send + Sync + 'static>() {}
    fn owner_bounds<T: std::fmt::Debug + Send + Sync + 'static>() {}
    control_derives::<LogControl>();
    owner_bounds::<LogGuard>();

    // Structured request.
    let _: fn(Level, String) -> StructuredRecord = |l, t| StructuredRecord::new(l, t);
    let _: fn(StructuredRecord, String) -> StructuredRecord = |r, a| r.with_action(a);
    let _: fn(StructuredRecord, String) -> StructuredRecord = |r, m| r.with_message(m);
    let _: fn(StructuredRecord, String, JsonValue) -> StructuredRecord =
        |r, k, v| r.with_field(k, v);
    let _ = |r: StructuredRecord| {
        let _: (Level, String) = (r.level, r.target);
        let _: (Option<String>, Option<String>) = (r.action, r.message);
        let _: JsonMap = r.fields;
    };
    fn serde_derives<T: serde::Serialize + serde::de::DeserializeOwned>() {}
    fn data_derives<T: std::fmt::Debug + Clone + PartialEq>() {}
    fn eq_derives<T: std::fmt::Debug + Clone + PartialEq + Eq>() {}
    fn state_derives<T: std::fmt::Debug + Clone + Copy + PartialEq + Eq + std::hash::Hash>() {}
    fn error_bounds<T: std::error::Error + Send + Sync + 'static>() {}
    serde_derives::<Level>();
    serde_derives::<StructuredRecord>();
    serde_derives::<SubmitOutcome>();
    serde_derives::<SubmitError>();
    serde_derives::<InvalidInputReason>();
    serde_derives::<FailureReport>();
    serde_derives::<Failure>();
    serde_derives::<InitFailure>();
    serde_derives::<FlushFailure>();
    serde_derives::<ShutdownFailure>();
    data_derives::<StructuredRecord>();
    data_derives::<FailureReport>();
    eq_derives::<SubmitError>();
    eq_derives::<InvalidInputReason>();
    eq_derives::<Failure>();
    state_derives::<SubmitOutcome>();
    state_derives::<InitFailure>();
    state_derives::<FlushFailure>();
    state_derives::<ShutdownFailure>();
    error_bounds::<SubmitError>();
    error_bounds::<InvalidInputReason>();

    // Result and error methods.
    let _: fn(&SubmitError) -> ErrorCode = SubmitError::code;
    let _: fn(&SubmitError) -> Remediation = SubmitError::remediation;
    let _: fn(&SubmitError) -> DropCause = SubmitError::drop_cause;
    let _: fn(&SubmitError) -> FailureReport = SubmitError::report;
    let _: fn(&InitError) -> FailureReport = InitError::report;
    let _: fn(&FlushError) -> FailureReport = FlushError::report;
    let _: fn(&ShutdownError) -> FailureReport = ShutdownError::report;
    let _: u32 = sc_observability_log::CONTROL_SCHEMA_VERSION;
    let _ = |r: FailureReport| {
        let _: (u32, Failure, ErrorCode) = (r.schema_version, r.failure, r.code);
        let _: (String, Remediation) = (r.message, r.remediation);
    };
    let _: [ErrorCode; 7] = [
        sc_observability_log::error_codes::SC_OBSERVABILITY_LOG_SUBMIT_QUEUE_FULL,
        sc_observability_log::error_codes::SC_OBSERVABILITY_LOG_SUBMIT_INVALID_INPUT,
        sc_observability_log::error_codes::SC_OBSERVABILITY_LOG_SUBMIT_STOPPED,
        sc_observability_log::error_codes::SC_OBSERVABILITY_LOG_SUBMIT_REENTRANT,
        sc_observability_log::error_codes::SC_OBSERVABILITY_LOG_SUBMIT_WRITER_DEGRADED,
        sc_observability_log::error_codes::SC_OBSERVABILITY_LOG_SUBMIT_BACKEND_SHUTDOWN_TIMED_OUT,
        sc_observability_log::error_codes::SC_OBSERVABILITY_LOG_SUBMIT_CONTAINED_PANIC,
    ];

    // Exhaustive matches: adding, removing or reshaping a variant breaks this test.
    let _ = |o: SubmitOutcome| match o {
        SubmitOutcome::Accepted | SubmitOutcome::Filtered => (),
    };
    let _ = |e: SubmitError| match e {
        SubmitError::QueueFull | SubmitError::Reentrant => (),
        SubmitError::InvalidInput(_reason) => (),
        SubmitError::Stopped { lifecycle: _ } => (),
        SubmitError::WriterDegraded | SubmitError::BackendShutdownTimedOut => (),
        SubmitError::ContainedPanic => (),
    };
    let _ = |r: InvalidInputReason| match r {
        InvalidInputReason::EmptyAction | InvalidInputReason::RejectedAction => (),
        InvalidInputReason::RejectedTarget | InvalidInputReason::EmptyFieldKey => (),
        InvalidInputReason::ReservedFieldKey { key: _ } => (),
        InvalidInputReason::RejectedByLogger => (),
    };
    let _ = |f: Failure| match f {
        Failure::Init(_) | Failure::Flush(_) | Failure::Shutdown(_) | Failure::Submit(_) => (),
    };
    let _ = |f: InitFailure| match f {
        InitFailure::AlreadyInitialized | InitFailure::ForeignLoggerInstalled => (),
        InitFailure::IdentityResolution | InitFailure::Logger => (),
    };
    let _ = |f: FlushFailure| match f {
        FlushFailure::TimedOut { timeout_ms: _ } | FlushFailure::Logger => (),
        FlushFailure::HelperSpawn | FlushFailure::HelperLost | FlushFailure::ShutDown => (),
    };
    let _ = |f: ShutdownFailure| match f {
        ShutdownFailure::TimedOut { timeout_ms: _ } | ShutdownFailure::FinalFlush => (),
        ShutdownFailure::HelperSpawn | ShutdownFailure::HelperLost => (),
    };
}
