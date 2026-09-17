//! [`LogControl`]: the cloneable, non-owning control surface of the installed bridge.
//!
//! `LogControl` is what code other than the lifecycle owner receives: a command
//! handler that flushes before clearing files, a status endpoint that reports
//! health, a frontend or binding that submits structured records. It exposes
//! exactly four operations — bounded [`flush`](LogControl::flush),
//! [`health`](LogControl::health), [`active_log_path`](LogControl::active_log_path)
//! and nonblocking [`submit`](LogControl::submit) — and nothing that owns,
//! replaces or stops the logger. The [`LogGuard`](crate::LogGuard) is the only
//! value that can shut the bridge down.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::health::BridgeLifecycle;
use crate::{
    BridgeHealthReport, ControlError, EmitError, FieldKeyError, FlushError, Level, LifecyclePhase,
    SubmitError, handle, health, mapping,
};

/// A JSON value, as stored in `LogEvent.fields` (re-exported `serde_json::Value`).
pub type JsonValue = serde_json::Value;

/// The field map of a [`StructuredRecord`] (a `serde_json::Map`).
pub type JsonMap = serde_json::Map<String, JsonValue>;

/// Typed direct producer input. Bridge-owned envelope identity and timestamps
/// remain absent, so a caller cannot replace host provenance.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeEvent {
    /// Canonical core event level.
    pub level: sc_observability_types::Level,
    /// Pre-validated event target.
    pub target: sc_observability_types::TargetCategory,
    /// Optional action; the bridge default is used when omitted.
    pub action: Option<sc_observability_types::ActionName>,
    /// Optional producer message.
    pub message: Option<String>,
    /// Optional operation outcome.
    pub outcome: Option<sc_observability_types::OutcomeLabel>,
    /// Producer fields, validated by the same key policy as macros.
    pub fields: JsonMap,
    /// Optional request identifier.
    pub request_id: Option<sc_observability_types::CorrelationId>,
    /// Optional correlation identifier.
    pub correlation_id: Option<sc_observability_types::CorrelationId>,
    /// Optional explicit trace context.
    pub trace: Option<sc_observability_types::TraceContext>,
}

/// Compatibility spelling for the core admission result, not an independent enum.
pub type EmitOutcome = sc_observability_types::AdmissionOutcome;

/// Cloneable, non-owning control of the installed bridge.
///
/// Obtained with [`LogGuard::control`](crate::LogGuard::control). It holds no
/// reference to the logger: keeping, cloning or dropping any number of controls
/// never delays or triggers shutdown, it cannot be turned back into a
/// `LogGuard`, and every method has a defined result after the owner has shut
/// the bridge down. It is `Send + Sync`.
///
/// The bridge is installed once per process, so every `LogControl` refers to
/// that one installation; there is nothing to select or reconnect.
#[derive(Debug, Clone)]
pub struct LogControl {
    _private: (),
}

impl LogControl {
    pub(crate) const fn new() -> Self {
        Self { _private: () }
    }

    /// Flushes on a helper thread, bounded by `timeout`.
    ///
    /// A flush that started before the owner's shutdown completes normally, and
    /// shutdown waits for it within its own timeout; a flush requested after
    /// shutdown started does nothing.
    ///
    /// # Errors
    ///
    /// [`FlushError::TimedOut`] when the writer does not acknowledge within
    /// `timeout` (the helper is detached and may complete later),
    /// [`FlushError::Logger`] when a sink flush fails,
    /// [`FlushError::HelperSpawn`] / [`FlushError::HelperLost`] for helper-thread
    /// failures, and [`FlushError::ShutDown`] once shutdown has started.
    pub fn flush(&self, timeout: Duration) -> Result<(), FlushError> {
        handle::flush_installed(timeout)
    }

    /// Read-only, serializable health snapshot; defined in every lifecycle phase.
    ///
    /// Never blocks on I/O or the writer queue and never panics. See
    /// [`BridgeHealthReport`] for the fields and the post-shutdown contract.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError::Unavailable`] when no readable core report is retained.
    pub fn health(&self) -> Result<BridgeHealthReport, ControlError> {
        health::snapshot()
    }

    /// The active JSONL file captured at `init`, as an owned value.
    ///
    /// `None` when `LoggerConfig.enable_file_sink` is false. The path stays
    /// available after shutdown (the file remains on disk).
    #[must_use]
    pub fn active_log_path(&self) -> Option<PathBuf> {
        health::active_log_path()
    }

    /// Snapshot of exact-once bridge rejection counters. This does not expose
    /// lifecycle ownership.
    #[must_use]
    pub fn dropped_events(&self) -> crate::DroppedEvents {
        handle::dropped_events()
    }

    /// Waits only for the already-started owner shutdown. Controls neither
    /// initiate shutdown nor retain the owner while waiting.
    ///
    /// # Errors
    ///
    /// Returns [`WaitError`](crate::WaitError) before initialization or when
    /// the requested observation deadline passes.
    pub fn wait_stopped(
        &self,
        timeout: Duration,
    ) -> Result<crate::ShutdownReport, crate::WaitError> {
        handle::wait_stopped(timeout)
    }

    /// Submits one structured record to the installed writer without blocking.
    ///
    /// The record goes through the same guarded submission core as the `log`
    /// facade and the event macros, to the same `sc_observability::Logger`. The
    /// bridge — not the caller — fills the envelope version, the UTC timestamp,
    /// the service name, the process identity and the ambient `#[instrument]`
    /// trace context, applies the configured redaction, and routes the event to
    /// the configured sinks. Labels and field keys follow the unified rules in
    /// `docs/mapping.md` ("Field keys").
    ///
    /// Never blocks on I/O or queue capacity and never panics.
    ///
    /// # Errors
    ///
    /// Every error is counted under exactly one
    /// [`DropCause`](crate::DropCause) ([`SubmitError::drop_cause`]) before it
    /// is returned:
    ///
    /// - [`SubmitError::Stopped`] when the lifecycle is not `Running`;
    /// - [`SubmitError::InvalidInput`] for an empty action, an empty or reserved
    ///   field key, or an event the logger rejects (nothing is written);
    /// - [`SubmitError::QueueFull`], [`SubmitError::WriterDegraded`],
    ///   [`SubmitError::BackendShutdownTimedOut`] from the logger;
    /// - [`SubmitError::Reentrant`] when called from inside another submission on
    ///   this thread (a sink, redactor or panic hook);
    /// - [`SubmitError::ContainedPanic`] when a panic in the logger, a sink or a
    ///   redactor was caught.
    ///
    /// A record below `LoggerConfig.level` is `Ok(SubmitOutcome::Filtered)` and
    /// counts nothing.
    pub fn submit(&self, record: StructuredRecord) -> Result<SubmitOutcome, SubmitError> {
        if handle::lifecycle() != BridgeLifecycle::Running {
            return Err(stopped(lifecycle_phase()));
        }
        handle::submit_guarded(|| {
            let installed = handle::current_installed().ok_or(SubmitError::Stopped {
                lifecycle: handle::lifecycle_phase(),
            })?;
            let parts = mapping::structured_to_parts(record).map_err(SubmitError::InvalidInput)?;
            handle::submit_to(&installed, parts).map_err(SubmitError::from_drop_cause)
        })
        .map(|outcome| match outcome {
            sc_observability_types::AdmissionOutcome::Accepted => SubmitOutcome::Accepted,
            sc_observability_types::AdmissionOutcome::Filtered => SubmitOutcome::Filtered,
        })
    }

    /// Admits a typed direct event through the same staged-core writer path as
    /// facade and macro producers.
    ///
    /// # Errors
    ///
    /// Returns [`EmitError`] after exact-once rejection accounting when the
    /// input, lifecycle, queue, writer, or guarded callback rejects admission.
    pub fn try_log(&self, event: BridgeEvent) -> Result<EmitOutcome, EmitError> {
        handle::submit_guarded(|| {
            if handle::lifecycle() != BridgeLifecycle::Running {
                return Err(not_running());
            }
            let installed = handle::current_installed().ok_or_else(not_running)?;
            let event = direct_event(event, &installed)?;
            installed
                .logger
                .try_log_with_outcome(event)
                .map_err(|error| core_emit_error(&error))
        })
    }

    /// Executes a typed core query without yielding owner or shutdown authority.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError`] when the bridge is not running or the staged
    /// core rejects the query.
    pub fn query(
        &self,
        query: &sc_observability_types::LogQuery,
    ) -> Result<sc_observability_types::LogSnapshot, ControlError> {
        let installed = handle::current_installed().ok_or_else(|| ControlError::NotRunning {
            phase: lifecycle_phase(),
        })?;
        installed.logger.query(query).map_err(|error| {
            let diagnostic = error.diagnostic();
            ControlError::Query {
                diagnostic: operation_diagnostic(
                    error.code(),
                    error.to_string(),
                    diagnostic.remediation.clone(),
                ),
            }
        })
    }
}

fn direct_event(
    event: BridgeEvent,
    installed: &handle::Installed,
) -> Result<sc_observability_types::LogEvent, EmitError> {
    let mut fields = JsonMap::new();
    let mut raw_keys = std::collections::BTreeMap::new();
    for (raw, value) in event.fields {
        let key = mapping::field_key_label(&raw)
            .map_err(|error| EmitError::InvalidField {
                raw_key: raw.clone(),
                reason: match error {
                    mapping::LabelError::Empty { .. } | mapping::LabelError::Rejected { .. } => {
                        FieldKeyError::Empty
                    }
                    mapping::LabelError::ReservedPrefix { .. } => FieldKeyError::ReservedPrefix,
                },
            })?
            .into_owned();
        if let Some(other_raw_key) = raw_keys.insert(key.clone(), raw.clone()) {
            return Err(EmitError::InvalidField {
                raw_key: raw,
                reason: FieldKeyError::Collision { other_raw_key },
            });
        }
        fields.insert(key, value);
    }
    let observation = sc_observability_types::Observation::new(installed.service.clone(), ());
    Ok(sc_observability_types::LogEvent {
        version: observation.version,
        timestamp: observation.timestamp,
        level: event.level,
        service: installed.service.clone(),
        target: event.target,
        action: event
            .action
            .unwrap_or_else(|| installed.options.default_action.clone()),
        message: event.message,
        identity: installed.identity.clone(),
        trace: event.trace.or_else(crate::context::current_trace),
        request_id: event.request_id,
        correlation_id: event.correlation_id,
        outcome: event.outcome,
        diagnostic: None,
        state_transition: None,
        fields,
    })
}

fn lifecycle_phase() -> LifecyclePhase {
    handle::lifecycle_phase()
}

fn not_running() -> EmitError {
    EmitError::NotRunning {
        phase: lifecycle_phase(),
    }
}

fn operation_diagnostic(
    code: sc_observability_types::ErrorCode,
    message: String,
    remediation: sc_observability_types::Remediation,
) -> sc_observability_types::OperationDiagnostic {
    sc_observability_types::OperationDiagnostic {
        code,
        message,
        remediation,
        at: sc_observability_types::Timestamp::now_utc(),
    }
}

fn core_emit_error(error: &sc_observability::TryLogError) -> EmitError {
    let error_text = error.to_string();
    let diagnostic = |code| {
        operation_diagnostic(
            code,
            error_text.clone(),
            sc_observability_types::Remediation::recoverable(
                "inspect the bridge health and retry when the logger is running",
                std::iter::empty::<String>(),
            ),
        )
    };
    match error {
        sc_observability::TryLogError::InvalidEvent(_) => EmitError::InvalidEvent {
            diagnostic: diagnostic(crate::error_codes::SC_OBSERVABILITY_LOG_SUBMIT_INVALID_INPUT),
        },
        sc_observability::TryLogError::QueueFull(_) => EmitError::QueueFull {
            diagnostic: diagnostic(crate::error_codes::SC_OBSERVABILITY_LOG_SUBMIT_QUEUE_FULL),
        },
        sc_observability::TryLogError::WriterDegraded(_) => EmitError::WriterDegraded {
            diagnostic: diagnostic(crate::error_codes::SC_OBSERVABILITY_LOG_SUBMIT_WRITER_DEGRADED),
        },
        sc_observability::TryLogError::ShutdownTimedOut(_) => EmitError::ShutdownTimedOut {
            diagnostic: diagnostic(
                crate::error_codes::SC_OBSERVABILITY_LOG_SUBMIT_BACKEND_SHUTDOWN_TIMED_OUT,
            ),
        },
    }
}

/// Counts and returns a rejection outside the guard (lifecycle not running).
fn stopped(lifecycle: LifecyclePhase) -> SubmitError {
    let error = SubmitError::Stopped { lifecycle };
    handle::record_drop(error.drop_cause());
    error
}

/// Successful result of [`LogControl::submit`]; serialized as a `snake_case` string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmitOutcome {
    /// The record was admitted to the writer queue.
    Accepted,
    /// The record is below `LoggerConfig.level`; nothing was written or counted.
    Filtered,
}

/// A structured record submitted through [`LogControl::submit`].
///
/// It carries only what a producer controls. The envelope version, timestamp,
/// service name, process identity, trace context, redaction and sink routing
/// belong to the bridge and cannot be supplied; deserializing a request that
/// names any other key fails (`deny_unknown_fields`).
///
/// Serialized shape (`level` uses the `LogEvent` spelling):
///
/// ```json
/// { "level": "Info", "target": "app.ui", "action": "ui.click",
///   "message": "clicked", "fields": { "button": "save" } }
/// ```
///
/// # Examples
///
/// ```
/// use sc_observability_log::{Level, StructuredRecord};
///
/// let record = StructuredRecord::new(Level::INFO, "app.ui")
///     .with_action("ui.click")
///     .with_message("clicked")
///     .with_field("button", "save");
/// assert_eq!(record.target, "app.ui");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct StructuredRecord {
    /// Event severity, compared with `LoggerConfig.level`.
    pub level: Level,
    /// Target category; sanitized like every target (`::` -> `.`, empty -> `log`).
    pub target: String,
    /// Action; `None` uses `BridgeOptions.default_action`. Sanitized; empty is invalid.
    #[serde(default)]
    pub action: Option<String>,
    /// Message text, stored as given.
    #[serde(default)]
    pub message: Option<String>,
    /// Structured fields; keys are stored in canonical sanitized form, and empty or reserved
    /// keys are invalid.
    #[serde(default)]
    pub fields: JsonMap,
}

impl StructuredRecord {
    /// A record with `level` and `target`, no action, no message and no fields.
    #[must_use]
    pub fn new(level: Level, target: impl Into<String>) -> Self {
        Self {
            level,
            target: target.into(),
            action: None,
            message: None,
            fields: JsonMap::new(),
        }
    }

    /// Sets the action.
    #[must_use]
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Sets the message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Inserts one field; a later value for the same key replaces the earlier one.
    #[must_use]
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<JsonValue>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_record_round_trips_and_rejects_bridge_owned_keys() {
        let record = StructuredRecord::new(Level::WARN, "app.ui")
            .with_action("ui.click")
            .with_message("clicked")
            .with_field("button", "save");
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"level": "Warn", "target": "app.ui", "action": "ui.click", "message": "clicked", "fields": {"button": "save"}})
        );
        assert_eq!(
            serde_json::from_value::<StructuredRecord>(json).unwrap(),
            record
        );
        let minimal: StructuredRecord =
            serde_json::from_str(r#"{"level":"Error","target":"t"}"#).unwrap();
        assert_eq!(minimal, StructuredRecord::new(Level::ERROR, "t"));
        for owned in ["timestamp", "service", "identity", "version", "trace"] {
            let text = format!(r#"{{"level":"Info","target":"t","{owned}":"x"}}"#);
            assert!(
                serde_json::from_str::<StructuredRecord>(&text).is_err(),
                "{owned} must not be accepted from a producer"
            );
        }
    }

    #[test]
    fn submit_outcome_is_a_snake_case_string() {
        assert_eq!(
            serde_json::to_value(SubmitOutcome::Accepted).unwrap(),
            "accepted"
        );
        assert_eq!(
            serde_json::to_value(SubmitOutcome::Filtered).unwrap(),
            "filtered"
        );
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn control_is_send_and_sync() {
        assert_send_sync::<LogControl>();
    }
}
