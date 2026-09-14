//! Read-only, bridge-owned health snapshot of the installed logger.
//!
//! [`BridgeHealthReport`] is a curated projection of `sc_observability::Logger::health()`
//! plus the bridge's own lifecycle and dropped-event counters. It is returned by
//! [`LogControl::health`](crate::LogControl::health) and
//! [`LogGuard::health`](crate::LogGuard::health) and never exposes the mutable
//! `Logger` or any ownership handle.
//!
//! # Stability and bindings
//!
//! The shape is versioned by [`BRIDGE_HEALTH_SCHEMA_VERSION`], carried in every
//! snapshot as `schema_version`. Every type derives `serde::Serialize` and
//! `serde::Deserialize`; enum-like data is a unit-variant enum serialized as a
//! `snake_case` string (for example `"degraded_dropping"`), counters are `u64`,
//! and diagnostic codes keep the stable `ErrorCode` string, so generated
//! TypeScript or Python bindings never parse free-form text to recover a state.
//! The structs are `#[non_exhaustive]`: fields may be added in a compatible
//! release together with a schema-version bump.
//!
//! # Thread safety and cost
//!
//! A snapshot may be taken from any thread at any time, concurrently with
//! logging, `flush` and `shutdown`. It never blocks on I/O or the writer queue:
//! it clones the installed `Arc` under the slot read lock (released at once) and
//! reads sc-observability's health counters. sc-observability 1.2.0 `health()`
//! `expect`s on internal mutexes; a panic there is caught and reported as
//! `logger: None` with `state: Unavailable` instead of unwinding.
//!
//! # After shutdown: timeout versus final stop
//!
//! Shutdown sets `lifecycle` to `ShuttingDown` when it starts. When it returns:
//!
//! - `Stopped` — final. The logger was shut down (also after
//!   `ShutdownError::FinalFlush`), or shutdown failed in a way that leaves
//!   nothing to finish (`HelperSpawn`, `HelperLost`).
//! - `ShutdownTimedOut` — `ShutdownError::TimedOut` was returned, but the
//!   detached helper is still waiting for the logger (for example for a flush or
//!   a submission blocked in a sink). When it completes, the lifecycle becomes
//!   `Stopped` and the final report appears; that transition is how a caller
//!   observes late completion. A process that exits first never sees it.
//!
//! Once the logger has been taken out of the slot, a snapshot reports the final
//! `Logger::health()` captured right after `Logger::shutdown` (writer state
//! `Stopped`), or `logger: None` while that has not happened (for example during
//! `ShutdownTimedOut`). A snapshot whose
//! `lifecycle` is not `Running` always has `state: Unavailable`, and the file and
//! console sinks report `Unavailable` (or `Disabled`), because nothing is written
//! any more. `dropped_events` stays readable; records logged after shutdown are
//! filtered by the `log` facade level (`Off`) and the emit threshold before they
//! reach the bridge, so they are neither written nor counted.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock, PoisonError};

use sc_observability::constants::{CONSOLE_SINK_NAME, JSONL_FILE_SINK_NAME};
use sc_observability::error_codes as upstream_codes;
use sc_observability_types::{
    DiagnosticSummary, ErrorCode, LoggingHealthReport, LoggingHealthState, Remediation, SinkHealth,
    SinkHealthState, Timestamp, WriterState,
};
use serde::{Deserialize, Serialize};

use crate::{DroppedEvents, handle};

/// Version of the [`BridgeHealthReport`] shape; bumped whenever a field or variant changes.
pub const BRIDGE_HEALTH_SCHEMA_VERSION: u32 = 1;

/// Point-in-time health of the bridge and its logger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BridgeHealthReport {
    /// [`BRIDGE_HEALTH_SCHEMA_VERSION`] of the producer.
    pub schema_version: u32,
    /// Bridge lifecycle phase.
    pub lifecycle: BridgeLifecycle,
    /// Aggregate state; `Unavailable` whenever `lifecycle` is not `Running`.
    pub state: BridgeHealthState,
    /// Logger runtime health; `None` when it could not be read (see the module docs).
    pub logger: Option<LoggerHealth>,
    /// The built-in JSONL file sink.
    pub file_sink: FileSinkHealth,
    /// The built-in console sink.
    pub console_sink: SinkHealthSnapshot,
    /// Process-wide dropped-event counters, the same values as `dropped_events()`.
    pub dropped_events: DroppedEvents,
}

/// Bridge lifecycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeLifecycle {
    /// The bridge is installed and accepting records.
    Running,
    /// `shutdown` has started: records are no longer accepted.
    ShuttingDown,
    /// `shutdown` returned `ShutdownError::TimedOut`; a detached helper may still complete it.
    ShutdownTimedOut,
    /// Final: the logger has shut down, or shutdown ended with nothing left to complete.
    Stopped,
}

/// Aggregate health state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeHealthState {
    /// The logger reports normal operation.
    Healthy,
    /// The logger is writing but has dropped events or failed flushes.
    Degraded,
    /// The logger is not writing: shutting down, stopped, or its health is unreadable.
    Unavailable,
}

/// Background writer-thread state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriterStatus {
    /// The writer is active and accepting queue work.
    Running,
    /// The writer is active but degraded during write, flush or shutdown work.
    Degraded,
    /// The writer has stopped.
    Stopped,
}

/// Health of one sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SinkStatus {
    /// The sink is disabled in `LoggerConfig`.
    Disabled,
    /// The sink is writing normally.
    Healthy,
    /// The sink is writing but dropping writes.
    DegradedDropping,
    /// The sink is not writing (failed, or the logger is shutting down or stopped).
    Unavailable,
}

/// Logger runtime health, projected from `Logger::health()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LoggerHealth {
    /// Background writer-thread state.
    pub writer_state: WriterStatus,
    /// Writer queue pressure.
    pub queue: QueueHealth,
    /// Last writer-thread error, if any.
    pub last_writer_error: Option<HealthDiagnostic>,
    /// Last logging error of any kind, if any.
    pub last_error: Option<HealthDiagnostic>,
    /// Events sc-observability itself dropped (bridge counters are in `dropped_events`).
    pub dropped_events_total: u64,
    /// Flush failures recorded by sc-observability.
    pub flush_errors_total: u64,
}

/// Writer queue pressure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct QueueHealth {
    /// Records admitted but not yet written.
    pub depth: u64,
    /// Configured bounded capacity (`LoggerConfig.queue_capacity`).
    pub capacity: u64,
    /// Highest observed depth since `init`.
    pub high_water_mark: u64,
    /// Records rejected because the queue was full.
    pub full_drops_total: u64,
}

/// Health of the built-in JSONL file sink.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FileSinkHealth {
    /// Sink status; `Disabled` when `LoggerConfig.enable_file_sink` is false.
    pub status: SinkStatus,
    /// Active JSONL file, as captured at `init`; `None` when the sink is disabled.
    pub active_log_path: Option<PathBuf>,
    /// Last sink error, if any.
    pub last_error: Option<HealthDiagnostic>,
}

/// Health of a sink that has no path (the console sink).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SinkHealthSnapshot {
    /// Sink status; `Disabled` when the sink is disabled in `LoggerConfig`.
    pub status: SinkStatus,
    /// Last sink error, if any.
    pub last_error: Option<HealthDiagnostic>,
}

/// One recorded failure with its stable code and a remediation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HealthDiagnostic {
    /// Stable sc-observability error code, when the runtime recorded one.
    pub code: Option<ErrorCode>,
    /// Human-readable summary (informational; match on `code`, not on this text).
    pub message: String,
    /// UTC time the failure was recorded.
    pub at: Timestamp,
    /// What to do about it, chosen by `code` (see `docs/mapping.md`, "Health").
    pub remediation: Remediation,
}

/// Sink configuration captured once at `init`; read by every snapshot.
#[derive(Debug)]
pub(crate) struct SinkConfig {
    pub(crate) file_enabled: bool,
    pub(crate) console_enabled: bool,
    pub(crate) active_log_path: Option<PathBuf>,
}

static SINK_CONFIG: OnceLock<SinkConfig> = OnceLock::new();
static FINAL_REPORT: Mutex<Option<LoggingHealthReport>> = Mutex::new(None);

/// Records the sink configuration of the (single) successful `init`.
pub(crate) fn set_sink_config(config: SinkConfig) {
    // `init` succeeds at most once per process, so the cell is always empty here.
    let _ = SINK_CONFIG.set(config);
}

/// The active JSONL path captured at `init`; `None` before `init` or with the file sink disabled.
pub(crate) fn active_log_path() -> Option<PathBuf> {
    SINK_CONFIG
        .get()
        .filter(|config| config.file_enabled)
        .and_then(|config| config.active_log_path.clone())
}

/// Stores the final report read from the stopped logger.
pub(crate) fn store_final_report(report: LoggingHealthReport) {
    *FINAL_REPORT.lock().unwrap_or_else(PoisonError::into_inner) = Some(report);
}

/// Reads `Logger::health()` without letting a panic in sc-observability unwind.
pub(crate) fn read_report<State>(
    logger: &sc_observability::Logger<State>,
) -> Option<LoggingHealthReport> {
    catch_unwind(AssertUnwindSafe(|| logger.health())).ok()
}

/// Takes a snapshot; see the module docs for the contract.
pub(crate) fn snapshot() -> BridgeHealthReport {
    let lifecycle = handle::lifecycle();
    let report = match handle::current_installed() {
        Some(installed) => read_report(&installed.logger),
        None => FINAL_REPORT
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone(),
    };
    project(
        lifecycle,
        report.as_ref(),
        SINK_CONFIG.get(),
        handle::dropped_events(),
    )
}

/// Pure projection of one report; unit-tested without a logger.
fn project(
    lifecycle: BridgeLifecycle,
    report: Option<&LoggingHealthReport>,
    config: Option<&SinkConfig>,
    dropped_events: DroppedEvents,
) -> BridgeHealthReport {
    let running = lifecycle == BridgeLifecycle::Running;
    let state = match (running, report) {
        (true, Some(report)) => match report.state {
            LoggingHealthState::Healthy => BridgeHealthState::Healthy,
            LoggingHealthState::DegradedDropping => BridgeHealthState::Degraded,
            LoggingHealthState::Unavailable => BridgeHealthState::Unavailable,
        },
        _ => BridgeHealthState::Unavailable,
    };
    let file_enabled = config.is_some_and(|c| c.file_enabled);
    let console_enabled = config.is_some_and(|c| c.console_enabled);
    let file = sink_snapshot(file_enabled, running, report, JSONL_FILE_SINK_NAME);
    BridgeHealthReport {
        schema_version: BRIDGE_HEALTH_SCHEMA_VERSION,
        lifecycle,
        state,
        logger: report.map(logger_health),
        file_sink: FileSinkHealth {
            status: file.status,
            active_log_path: config
                .filter(|c| c.file_enabled)
                .and_then(|c| c.active_log_path.clone()),
            last_error: file.last_error,
        },
        console_sink: sink_snapshot(console_enabled, running, report, CONSOLE_SINK_NAME),
        dropped_events,
    }
}

fn logger_health(report: &LoggingHealthReport) -> LoggerHealth {
    LoggerHealth {
        writer_state: match report.writer_state {
            WriterState::Running => WriterStatus::Running,
            WriterState::Degraded => WriterStatus::Degraded,
            WriterState::Stopped => WriterStatus::Stopped,
        },
        queue: QueueHealth {
            depth: report.queue_depth,
            capacity: report.queue_capacity,
            high_water_mark: report.queue_high_water_mark,
            full_drops_total: report.queue_full_drops_total,
        },
        last_writer_error: report.last_writer_error.as_ref().map(diagnostic),
        last_error: report.last_error.as_ref().map(diagnostic),
        dropped_events_total: report.dropped_events_total,
        flush_errors_total: report.flush_errors_total,
    }
}

fn sink_snapshot(
    enabled: bool,
    running: bool,
    report: Option<&LoggingHealthReport>,
    name: &str,
) -> SinkHealthSnapshot {
    if !enabled {
        return SinkHealthSnapshot {
            status: SinkStatus::Disabled,
            last_error: None,
        };
    }
    let sink: Option<&SinkHealth> =
        report.and_then(|r| r.sink_statuses.iter().find(|s| s.name.as_str() == name));
    let status = match (running, sink) {
        (true, Some(sink)) => match sink.state {
            SinkHealthState::Healthy => SinkStatus::Healthy,
            SinkHealthState::DegradedDropping => SinkStatus::DegradedDropping,
            SinkHealthState::Unavailable => SinkStatus::Unavailable,
        },
        _ => SinkStatus::Unavailable,
    };
    SinkHealthSnapshot {
        status,
        last_error: sink.and_then(|s| s.last_error.as_ref()).map(diagnostic),
    }
}

fn diagnostic(summary: &DiagnosticSummary) -> HealthDiagnostic {
    HealthDiagnostic {
        code: summary.code.clone(),
        message: summary.message.clone(),
        at: summary.at,
        remediation: remediation_for(summary.code.as_ref()),
    }
}

/// Remediation for a recorded code.
///
/// sc-observability's `DiagnosticSummary` keeps only code, message and time, so
/// the bridge maps each stable code to a remediation. Wording follows the
/// remediation sc-observability 1.2.0 attaches to the same code, adapted where
/// the upstream step ("recreate the logger instance") is impossible through the
/// bridge, whose `init` succeeds once per process.
fn remediation_for(code: Option<&ErrorCode>) -> Remediation {
    let Some(code) = code else {
        return Remediation::not_recoverable(
            "the runtime recorded this failure without a stable code; report the message upstream",
        );
    };
    if *code == upstream_codes::LOGGER_QUEUE_FULL {
        Remediation::recoverable(
            "reduce logging pressure or increase LoggerConfig.queue_capacity",
            ["inspect BridgeHealthReport.logger.queue.depth and high_water_mark"],
        )
    } else if *code == upstream_codes::LOGGER_WRITER_DEGRADED {
        Remediation::recoverable(
            "restart the process: the bridge cannot reinstall the logger in-process",
            ["inspect BridgeHealthReport.logger.last_writer_error"],
        )
    } else if *code == upstream_codes::LOGGER_FLUSH_FAILED {
        Remediation::recoverable(
            "inspect the writer-thread flush failure",
            [
                "inspect BridgeHealthReport.logger.last_writer_error",
                "retry the flush after the writer recovers",
            ],
        )
    } else if *code == upstream_codes::LOGGER_SINK_WRITE_FAILED {
        Remediation::recoverable(
            "check that the log directory exists, is writable and has free space",
            ["inspect BridgeHealthReport.file_sink.active_log_path"],
        )
    } else if *code == upstream_codes::LOGGER_SHUTDOWN_TIMED_OUT {
        Remediation::not_recoverable(
            "the writer exceeded its shutdown threshold; still-queued events may be lost",
        )
    } else if *code == upstream_codes::LOGGER_SHUTDOWN {
        Remediation::not_recoverable("the logger has shut down; records are no longer written")
    } else if *code == upstream_codes::LOGGER_MAINTENANCE_FAILED
        || *code == upstream_codes::LOGGER_MAINTENANCE_JOIN_TIMEOUT
        || *code == upstream_codes::LOGGER_MAINTENANCE_WORKER_FAILED
    {
        Remediation::not_recoverable(
            "retained-log maintenance failure handling is owned by the logger runtime",
        )
    } else {
        Remediation::not_recoverable(
            "no bridge remediation is registered for this code; report the diagnostic upstream",
        )
    }
}

#[cfg(test)]
mod tests {
    use sc_observability_types::SinkName;

    use super::*;

    fn summary(code: ErrorCode) -> DiagnosticSummary {
        DiagnosticSummary {
            code: Some(code),
            message: "writer failed".to_owned(),
            at: Timestamp::UNIX_EPOCH,
        }
    }

    fn report(state: LoggingHealthState, file: SinkHealthState) -> LoggingHealthReport {
        LoggingHealthReport {
            state,
            dropped_events_total: 4,
            flush_errors_total: 1,
            active_log_path: PathBuf::from("root/logs/svc.log.jsonl"),
            sink_statuses: vec![
                SinkHealth {
                    name: SinkName::new(JSONL_FILE_SINK_NAME).unwrap(),
                    state: file,
                    last_error: Some(summary(upstream_codes::LOGGER_SINK_WRITE_FAILED)),
                },
                SinkHealth {
                    name: SinkName::new(CONSOLE_SINK_NAME).unwrap(),
                    state: SinkHealthState::Healthy,
                    last_error: None,
                },
            ],
            queue_depth: 3,
            queue_capacity: 8,
            queue_high_water_mark: 7,
            queue_full_drops_total: 2,
            writer_state: WriterState::Degraded,
            last_writer_error: Some(summary(upstream_codes::LOGGER_WRITER_DEGRADED)),
            query: None,
            maintenance: None,
            last_error: None,
        }
    }

    fn config(file_enabled: bool) -> SinkConfig {
        SinkConfig {
            file_enabled,
            console_enabled: false,
            active_log_path: Some(PathBuf::from("root/logs/svc.log.jsonl")),
        }
    }

    #[test]
    fn running_projection_keeps_every_field() {
        let report = report(
            LoggingHealthState::DegradedDropping,
            SinkHealthState::DegradedDropping,
        );
        let health = project(
            BridgeLifecycle::Running,
            Some(&report),
            Some(&config(true)),
            DroppedEvents::default(),
        );
        assert_eq!(health.schema_version, BRIDGE_HEALTH_SCHEMA_VERSION);
        assert_eq!(health.state, BridgeHealthState::Degraded);
        let logger = health.logger.as_ref().unwrap();
        assert_eq!(logger.writer_state, WriterStatus::Degraded);
        assert_eq!(
            logger.queue,
            QueueHealth {
                depth: 3,
                capacity: 8,
                high_water_mark: 7,
                full_drops_total: 2
            }
        );
        assert_eq!(
            (logger.dropped_events_total, logger.flush_errors_total),
            (4, 1)
        );
        let writer_error = logger.last_writer_error.as_ref().unwrap();
        assert_eq!(
            writer_error.code,
            Some(upstream_codes::LOGGER_WRITER_DEGRADED)
        );
        assert!(matches!(
            writer_error.remediation,
            Remediation::Recoverable { .. }
        ));
        assert_eq!(health.file_sink.status, SinkStatus::DegradedDropping);
        assert_eq!(
            health.file_sink.active_log_path,
            Some(PathBuf::from("root/logs/svc.log.jsonl"))
        );
        assert_eq!(
            health.file_sink.last_error.as_ref().unwrap().code,
            Some(upstream_codes::LOGGER_SINK_WRITE_FAILED)
        );
        assert_eq!(health.console_sink.status, SinkStatus::Disabled);
    }

    #[test]
    fn non_running_lifecycle_is_unavailable() {
        let report = report(LoggingHealthState::Healthy, SinkHealthState::Healthy);
        for lifecycle in [
            BridgeLifecycle::ShuttingDown,
            BridgeLifecycle::ShutdownTimedOut,
            BridgeLifecycle::Stopped,
        ] {
            let health = project(
                lifecycle,
                Some(&report),
                Some(&config(true)),
                DroppedEvents::default(),
            );
            assert_eq!(health.state, BridgeHealthState::Unavailable);
            assert_eq!(health.file_sink.status, SinkStatus::Unavailable);
            assert!(health.logger.is_some(), "the final report stays readable");
        }
        let unreadable = project(
            BridgeLifecycle::Running,
            None,
            Some(&config(false)),
            DroppedEvents::default(),
        );
        assert_eq!(unreadable.state, BridgeHealthState::Unavailable);
        assert!(unreadable.logger.is_none());
        assert_eq!(unreadable.file_sink.status, SinkStatus::Disabled);
        assert_eq!(unreadable.file_sink.active_log_path, None);
    }

    #[test]
    fn every_known_code_has_a_non_empty_remediation() {
        let mut codes: Vec<Option<ErrorCode>> =
            upstream_codes::ALL.iter().cloned().map(Some).collect();
        codes.push(None);
        codes.push(Some(ErrorCode::new_static("SOME_FUTURE_CODE")));
        for code in &codes {
            match remediation_for(code.as_ref()) {
                Remediation::Recoverable { steps } => assert!(!steps.steps().is_empty()),
                Remediation::NotRecoverable { justification } => {
                    assert!(!justification.is_empty());
                }
            }
        }
    }

    #[test]
    fn snapshot_round_trips_through_serde_with_string_enums() {
        let report = report(LoggingHealthState::Healthy, SinkHealthState::Healthy);
        let health = project(
            BridgeLifecycle::ShuttingDown,
            Some(&report),
            Some(&config(true)),
            DroppedEvents::default(),
        );
        let json = serde_json::to_value(&health).unwrap();
        assert_eq!(json["lifecycle"], "shutting_down");
        assert_eq!(json["state"], "unavailable");
        assert_eq!(json["logger"]["writer_state"], "degraded");
        assert_eq!(json["console_sink"]["status"], "disabled");
        assert_eq!(json["dropped_events"]["queue_full"], 0);
        let writer_error = &json["logger"]["last_writer_error"];
        assert_eq!(
            writer_error["code"],
            "SC_OBSERVABILITY_LOGGER_WRITER_DEGRADED"
        );
        assert_eq!(writer_error["remediation"]["kind"], "recoverable");
        assert_eq!(writer_error["at"], "1970-01-01T00:00:00Z");
        let decoded: BridgeHealthReport = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, health);
    }
}
