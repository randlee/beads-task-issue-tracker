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

use crate::{
    BridgeHealthReport, BridgeLifecycle, FlushError, Level, SubmitError, handle, health, mapping,
};

/// A JSON value, as stored in `LogEvent.fields` (re-exported `serde_json::Value`).
pub type JsonValue = serde_json::Value;

/// The field map of a [`StructuredRecord`] (a `serde_json::Map`).
pub type JsonMap = serde_json::Map<String, JsonValue>;

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
    #[must_use]
    pub fn health(&self) -> BridgeHealthReport {
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
        if !handle::level_enabled(record.level.into()) {
            // Shutdown sets the threshold to Off: tell "stopped" from "filtered".
            let lifecycle = handle::lifecycle();
            if lifecycle != BridgeLifecycle::Running {
                return Err(stopped(lifecycle));
            }
            return Ok(SubmitOutcome::Filtered);
        }
        let lifecycle = handle::lifecycle();
        if lifecycle != BridgeLifecycle::Running {
            return Err(stopped(lifecycle));
        }
        handle::submit_guarded(|| {
            let installed = handle::current_installed().ok_or(SubmitError::Stopped {
                lifecycle: handle::lifecycle(),
            })?;
            let parts = mapping::structured_to_parts(record).map_err(SubmitError::InvalidInput)?;
            handle::submit_to(&installed, parts).map_err(SubmitError::from_drop_cause)
        })
        .map(|()| SubmitOutcome::Accepted)
    }
}

/// Counts and returns a rejection outside the guard (lifecycle not running).
fn stopped(lifecycle: BridgeLifecycle) -> SubmitError {
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
