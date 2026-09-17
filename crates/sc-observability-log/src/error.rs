//! Public error enums and the dropped-event cause.
//!
//! Every error type is a discriminated union: callers `match` on the variant and
//! read its typed fields. Each public error enum also exposes a stable
//! [`ErrorCode`] and a mandatory [`Remediation`] per variant; variants wrapping an
//! sc-observability error return that error's own code and remediation. Each
//! also projects into the serializable [`FailureReport`](crate::FailureReport) with
//! `report()`, so bindings never parse a display string.

use std::time::Duration;

use sc_observability_types::{DiagnosticInfo, ErrorCode, LevelFilter, Remediation};
use serde::{Deserialize, Serialize};

use crate::{BridgeLifecycle, error_codes};

/// Lifecycle projection used by the reviewed B.P3 direct/control contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecyclePhase {
    /// The bridge is installed and can admit work.
    Running,
    /// The owner has begun final shutdown.
    Stopping,
    /// Final shutdown was confirmed.
    Stopped,
    /// Completion could not be confirmed; this never claims the writer stopped.
    Failed,
}

/// Reason a direct producer field cannot enter the bridge event.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum FieldKeyError {
    /// The raw key is empty.
    #[error("field key is empty")]
    Empty,
    /// The raw key belongs to bridge-owned metadata.
    #[error("field key uses a reserved prefix")]
    ReservedPrefix,
    /// Two distinct raw keys normalize to the same output key.
    #[error("field key collides with {other_raw_key:?}")]
    Collision { other_raw_key: String },
}

/// Typed direct-admission failure; each path has already recorded exactly one drop cause.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum EmitError {
    /// A producer field cannot be represented safely.
    #[error("invalid field {raw_key:?}: {reason}")]
    InvalidField {
        raw_key: String,
        reason: FieldKeyError,
    },
    /// The core rejected the assembled event.
    #[error("invalid event: {diagnostic}")]
    InvalidEvent {
        diagnostic: sc_observability_types::OperationDiagnostic,
    },
    /// The core queue is full.
    #[error("writer queue is full: {diagnostic}")]
    QueueFull {
        diagnostic: sc_observability_types::OperationDiagnostic,
    },
    /// The writer cannot accept more work.
    #[error("writer is degraded: {diagnostic}")]
    WriterDegraded {
        diagnostic: sc_observability_types::OperationDiagnostic,
    },
    /// The core shutdown deadline has elapsed.
    #[error("logger shutdown timed out: {diagnostic}")]
    ShutdownTimedOut {
        diagnostic: sc_observability_types::OperationDiagnostic,
    },
    /// The lifecycle is no longer running.
    #[error("logger is not running: {phase:?}")]
    NotRunning { phase: LifecyclePhase },
    /// The producer re-entered the guarded path.
    #[error("reentrant emission")]
    Reentrant,
    /// A logger callback panic was contained.
    #[error("logger callback panicked")]
    Panicked,
}

/// Failure of a read-only control operation.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ControlError {
    /// The lifecycle does not permit the request.
    #[error("logger is not running: {phase:?}")]
    NotRunning { phase: LifecyclePhase },
    /// A core query failed.
    #[error("query failed: {diagnostic}")]
    Query {
        diagnostic: sc_observability_types::OperationDiagnostic,
    },
    /// A snapshot or capability is unavailable.
    #[error("control operation unavailable: {diagnostic}")]
    Unavailable {
        diagnostic: sc_observability_types::OperationDiagnostic,
    },
}

/// Failure while waiting for owner shutdown completion.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum WaitError {
    /// No bridge lifecycle has begun in this process.
    #[error("logger was not started")]
    NotStarted,
    /// The completed owner result was not observed by the deadline.
    #[error("shutdown wait timed out after {timeout:?}")]
    TimedOut { timeout: Duration },
    /// Observation state is unavailable.
    #[error("shutdown state unavailable: {diagnostic}")]
    Unavailable {
        diagnostic: sc_observability_types::OperationDiagnostic,
    },
}

/// Outcome retained after owner shutdown has begun.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum UnconfirmedShutdown {
    /// The shutdown helper could not be started.
    HelperSpawn {
        diagnostic: sc_observability_types::OperationDiagnostic,
    },
    /// The shutdown helper disappeared before a final result.
    HelperLost {
        diagnostic: sc_observability_types::OperationDiagnostic,
    },
}

/// Final or observable pending shutdown state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ShutdownOutcome {
    /// The core writer reported a confirmed stop.
    Stopped,
    /// The writer stopped despite a final flush error.
    StoppedWithFlushError {
        diagnostic: sc_observability_types::OperationDiagnostic,
    },
    /// Completion was not confirmed; callers must not infer stopped.
    Unconfirmed { cause: UnconfirmedShutdown },
}

/// Read-only shutdown observation returned to controls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShutdownReport {
    /// Completion outcome.
    pub outcome: ShutdownOutcome,
    /// Health projection taken at observation time.
    pub health: crate::BridgeHealthReport,
}

impl EmitError {
    /// Stable code for this direct-admission failure.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidField { .. } => error_codes::SC_OBSERVABILITY_LOG_INVALID_FIELD,
            Self::InvalidEvent { diagnostic }
            | Self::QueueFull { diagnostic }
            | Self::WriterDegraded { diagnostic }
            | Self::ShutdownTimedOut { diagnostic } => diagnostic.code.clone(),
            Self::NotRunning { .. } => error_codes::SC_OBSERVABILITY_LOG_NOT_RUNNING,
            Self::Reentrant => error_codes::SC_OBSERVABILITY_LOG_REENTRANT_EMIT,
            Self::Panicked => error_codes::SC_OBSERVABILITY_LOG_LOGGER_PANICKED,
        }
    }

    /// Recovery guidance preserved from the staged core where available.
    #[must_use]
    pub fn remediation(&self) -> Remediation {
        match self {
            Self::InvalidEvent { diagnostic }
            | Self::QueueFull { diagnostic }
            | Self::WriterDegraded { diagnostic }
            | Self::ShutdownTimedOut { diagnostic } => diagnostic.remediation.clone(),
            Self::InvalidField { .. } => {
                Remediation::recoverable("correct the field key", ["resubmit the event"])
            }
            Self::NotRunning { .. } => {
                Remediation::not_recoverable("the owner has stopped the bridge")
            }
            Self::Reentrant => Remediation::recoverable(
                "emit after the enclosing logger callback returns",
                std::iter::empty::<String>(),
            ),
            Self::Panicked => Remediation::recoverable(
                "inspect the sink or redactor callback",
                std::iter::empty::<String>(),
            ),
        }
    }
}

impl ControlError {
    /// Stable code for this control failure.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::NotRunning { .. } => error_codes::SC_OBSERVABILITY_LOG_NOT_RUNNING,
            Self::Query { diagnostic } | Self::Unavailable { diagnostic } => {
                diagnostic.code.clone()
            }
        }
    }

    /// Recovery guidance for this control failure.
    #[must_use]
    pub fn remediation(&self) -> Remediation {
        match self {
            Self::Query { diagnostic } | Self::Unavailable { diagnostic } => {
                diagnostic.remediation.clone()
            }
            Self::NotRunning { .. } => {
                Remediation::not_recoverable("the owner has stopped the bridge")
            }
        }
    }
}

impl WaitError {
    /// Stable code for this wait failure.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::NotStarted => error_codes::SC_OBSERVABILITY_LOG_SHUTDOWN_NOT_STARTED,
            Self::TimedOut { .. } => error_codes::SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT,
            Self::Unavailable { diagnostic } => diagnostic.code.clone(),
        }
    }

    /// Recovery guidance for this wait failure.
    #[must_use]
    pub fn remediation(&self) -> Remediation {
        match self {
            Self::Unavailable { diagnostic } => diagnostic.remediation.clone(),
            Self::NotStarted => Remediation::recoverable(
                "initialize the bridge before waiting",
                std::iter::empty::<String>(),
            ),
            Self::TimedOut { .. } => Remediation::recoverable(
                "wait longer for the existing owner shutdown",
                std::iter::empty::<String>(),
            ),
        }
    }
}

/// Error returned by [`init`](crate::init).
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    /// The bridge is already installed (or being installed) in this process.
    #[error("sc-observability-log is already initialized in this process")]
    AlreadyInitialized,
    /// Another `log::Log` implementation owns the `log` facade.
    #[error("another log::Log implementation is already installed")]
    ForeignLoggerInstalled {
        /// Error reported by `log::set_boxed_logger`.
        #[source]
        source: log::SetLoggerError,
    },
    /// The requested baseline cannot be represented by the executable's
    /// compile-time `log` cap. This is rejected before global installation.
    #[error("configured level {configured:?} exceeds available static level {available:?}")]
    UnsupportedLevel {
        /// Requested configured baseline.
        configured: LevelFilter,
        /// Most verbose facade level compiled into this executable.
        available: LevelFilter,
    },
    /// `ProcessIdentityPolicy::Resolver` failed, or `Auto` could not resolve a non-empty hostname.
    #[error("process identity resolution failed")]
    IdentityResolution {
        /// The identity failure, carrying the stable code and the path-specific remediation.
        #[source]
        source: sc_observability_types::IdentityError,
    },
    /// `sc_observability::Logger::new` failed.
    #[error("sc-observability logger construction failed")]
    Logger {
        /// Error reported by sc-observability.
        #[source]
        source: sc_observability_types::InitError,
    },
}

/// Error returned by [`LogGuard::flush`](crate::LogGuard::flush) and [`LogControl::flush`](crate::LogControl::flush).
#[derive(Debug, thiserror::Error)]
pub enum FlushError {
    /// The writer did not acknowledge the flush within `timeout`.
    #[error("flush did not complete within {timeout:?}")]
    TimedOut {
        /// The timeout that elapsed.
        timeout: Duration,
    },
    /// A sink flush failed or the writer disconnected.
    #[error("sc-observability flush failed")]
    Logger {
        /// Error reported by sc-observability.
        #[source]
        source: sc_observability_types::FlushError,
    },
    /// The flush helper thread could not be started.
    #[error("could not start the flush helper thread")]
    HelperSpawn {
        /// Error reported by `std::thread::Builder::spawn`.
        #[source]
        source: std::io::Error,
    },
    /// The flush helper thread ended without a result.
    #[error("the flush helper thread ended without a result")]
    HelperLost,
    /// Shutdown has started (or finished): there is no logger left to flush.
    #[error("the logger is shutting down or has shut down; nothing was flushed")]
    ShutDown,
    /// A previous flush helper is still running (possibly detached after its caller's
    /// timeout); no new helper was started and nothing new was flushed.
    #[error("a previous flush is still running; no new flush was started")]
    InProgress,
}

/// Error returned by [`LogGuard::shutdown`](crate::LogGuard::shutdown).
#[derive(Debug, thiserror::Error)]
pub enum ShutdownError {
    /// Sole ownership, final flush and writer join did not finish within `timeout`.
    #[error("shutdown did not complete within {timeout:?}")]
    TimedOut {
        /// The timeout that elapsed.
        timeout: Duration,
    },
    /// The final flush failed; the logger was still shut down.
    #[error("final flush failed; the logger was still shut down")]
    FinalFlush {
        /// Error reported by sc-observability.
        #[source]
        source: sc_observability_types::FlushError,
    },
    /// The shutdown helper thread could not be started.
    #[error("could not start the shutdown helper thread")]
    HelperSpawn {
        /// Error reported by `std::thread::Builder::spawn`.
        #[source]
        source: std::io::Error,
    },
    /// The shutdown helper thread ended without a result.
    #[error("the shutdown helper thread ended without a result")]
    HelperLost,
}

impl InitError {
    /// Stable code per variant; wrapped sc-observability errors return their own code.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::AlreadyInitialized => error_codes::SC_OBSERVABILITY_LOG_ALREADY_INITIALIZED,
            Self::ForeignLoggerInstalled { .. } => {
                error_codes::SC_OBSERVABILITY_LOG_FOREIGN_LOGGER_INSTALLED
            }
            Self::UnsupportedLevel { .. } => error_codes::SC_OBSERVABILITY_LOG_UNSUPPORTED_LEVEL,
            Self::IdentityResolution { source } => source.diagnostic().code.clone(),
            Self::Logger { source } => source.diagnostic().code.clone(),
        }
    }

    /// Mandatory remediation per variant; wrapped errors return their own remediation.
    #[must_use]
    pub fn remediation(&self) -> Remediation {
        match self {
            Self::AlreadyInitialized => Remediation::not_recoverable(
                "the log facade logger cannot be replaced; keep the first LogGuard",
            ),
            Self::ForeignLoggerInstalled { .. } => Remediation::recoverable(
                "remove the other log::Log implementation",
                ["or call sc_observability_log::init before it is installed"],
            ),
            Self::UnsupportedLevel { .. } => Remediation::not_recoverable(
                "rebuild without the static cap or choose a supported startup baseline",
            ),
            Self::IdentityResolution { source } => source.diagnostic().remediation.clone(),
            Self::Logger { source } => source.diagnostic().remediation.clone(),
        }
    }
}

impl FlushError {
    /// Stable code per variant; wrapped sc-observability errors return their own code.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::TimedOut { .. } => error_codes::SC_OBSERVABILITY_LOG_FLUSH_TIMED_OUT,
            Self::Logger { source } => source.diagnostic().code.clone(),
            Self::HelperSpawn { .. } => error_codes::SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED,
            Self::HelperLost => error_codes::SC_OBSERVABILITY_LOG_HELPER_LOST,
            Self::ShutDown => error_codes::SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN,
            Self::InProgress => error_codes::SC_OBSERVABILITY_LOG_FLUSH_IN_PROGRESS,
        }
    }

    /// Mandatory remediation per variant; wrapped errors return their own remediation.
    #[must_use]
    pub fn remediation(&self) -> Remediation {
        match self {
            Self::TimedOut { .. } => Remediation::recoverable(
                "retry the flush later or raise the timeout",
                ["a shutdown before the detached flush returns reports ShutdownError::TimedOut"],
            ),
            Self::Logger { source } => source.diagnostic().remediation.clone(),
            Self::HelperSpawn { .. } => {
                Remediation::recoverable("retry the flush", ["check process thread limits"])
            }
            Self::HelperLost => Remediation::recoverable(
                "shut the LogGuard down",
                ["the logger may be degraded after a panic inside sc-observability"],
            ),
            Self::ShutDown => Remediation::not_recoverable(
                "the lifecycle owner has shut the logger down; the final shutdown flushed what was queued",
            ),
            Self::InProgress => Remediation::recoverable(
                "wait for the previous flush to finish, then retry",
                [
                    "BridgeHealthReport.helpers.flush_in_flight turns false when it finishes",
                    "a flush that never finishes points at a stuck sink or disk",
                ],
            ),
        }
    }
}

impl ShutdownError {
    /// Stable code per variant; wrapped sc-observability errors return their own code.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::TimedOut { .. } => error_codes::SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT,
            Self::FinalFlush { source } => source.diagnostic().code.clone(),
            Self::HelperSpawn { .. } => error_codes::SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED,
            Self::HelperLost => error_codes::SC_OBSERVABILITY_LOG_HELPER_LOST,
        }
    }

    /// Mandatory remediation per variant; wrapped errors return their own remediation.
    #[must_use]
    pub fn remediation(&self) -> Remediation {
        match self {
            Self::TimedOut { .. } => Remediation::not_recoverable(
                "none needed at process exit; still-queued events may be lost",
            ),
            Self::FinalFlush { source } => source.diagnostic().remediation.clone(),
            Self::HelperSpawn { .. } => Remediation::not_recoverable(
                "none needed at process exit; the logger slot is already empty",
            ),
            Self::HelperLost => Remediation::not_recoverable(
                "none needed at process exit; the shutdown helper ended without a result",
            ),
        }
    }
}

/// Error returned by [`LogControl::submit`](crate::LogControl::submit).
///
/// Every variant corresponds to exactly one [`DropCause`] ([`SubmitError::drop_cause`]),
/// which the bridge has already counted when the error is returned. The type is
/// plain data: it serializes as an object internally tagged by `"kind"`
/// (`snake_case`), with `InvalidInput` also carrying its `"reason"` tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubmitError {
    /// The bounded writer queue was full; the record was not admitted.
    #[error("the writer queue is full; the record was dropped")]
    QueueFull,
    /// The request was invalid; nothing was written.
    #[error("invalid structured record: {0}")]
    InvalidInput(InvalidInputReason),
    /// The bridge is not running: shutdown has started, timed out or finished.
    #[error("the logger is not running (lifecycle {lifecycle:?}); the record was dropped")]
    Stopped {
        /// The lifecycle phase observed when the record was rejected.
        lifecycle: BridgeLifecycle,
    },
    /// Submitted while this thread was already inside a submission (a sink, redactor or panic hook).
    #[error("submitted from inside another submission on this thread; the record was dropped")]
    Reentrant,
    /// The writer thread is degraded and rejects new records.
    #[error("the writer is degraded; the record was dropped")]
    WriterDegraded,
    /// The logger runtime exceeded its own shutdown threshold.
    #[error("the logger runtime exceeded its shutdown threshold; the record was dropped")]
    BackendShutdownTimedOut,
    /// A panic inside the logger, a sink or a redactor was caught; the record was dropped.
    #[error("a panic inside the logger was contained; the record was dropped")]
    ContainedPanic,
}

/// Why a [`StructuredRecord`](crate::StructuredRecord) was rejected.
///
/// Serialized internally tagged by `"reason"` (`snake_case`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum InvalidInputReason {
    /// `action` is present but empty after sanitizing.
    #[error("the action is empty")]
    EmptyAction,
    /// `action` was rejected by sc-observability after sanitizing (not expected).
    #[error("the action was rejected after sanitizing")]
    RejectedAction,
    /// `target` was rejected by sc-observability after sanitizing (not expected).
    #[error("the target was rejected after sanitizing")]
    RejectedTarget,
    /// A field key is empty after sanitizing.
    #[error("a field key is empty")]
    EmptyFieldKey,
    /// A field key starts with the reserved `sc_observability_log.` prefix after sanitizing.
    #[error("field key {key:?} uses the reserved prefix `sc_observability_log.`")]
    ReservedFieldKey {
        /// The key as submitted.
        key: String,
    },
    /// sc-observability rejected the assembled event (`TryLogError::InvalidEvent`).
    #[error("the logger rejected the assembled event")]
    RejectedByLogger,
}

impl SubmitError {
    /// Stable code per variant.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::QueueFull => error_codes::SC_OBSERVABILITY_LOG_SUBMIT_QUEUE_FULL,
            Self::InvalidInput(_) => error_codes::SC_OBSERVABILITY_LOG_SUBMIT_INVALID_INPUT,
            Self::Stopped { .. } => error_codes::SC_OBSERVABILITY_LOG_SUBMIT_STOPPED,
            Self::Reentrant => error_codes::SC_OBSERVABILITY_LOG_SUBMIT_REENTRANT,
            Self::WriterDegraded => error_codes::SC_OBSERVABILITY_LOG_SUBMIT_WRITER_DEGRADED,
            Self::BackendShutdownTimedOut => {
                error_codes::SC_OBSERVABILITY_LOG_SUBMIT_BACKEND_SHUTDOWN_TIMED_OUT
            }
            Self::ContainedPanic => error_codes::SC_OBSERVABILITY_LOG_SUBMIT_CONTAINED_PANIC,
        }
    }

    /// Mandatory remediation per variant.
    #[must_use]
    pub fn remediation(&self) -> Remediation {
        match self {
            Self::QueueFull => Remediation::recoverable(
                "retry later, reduce logging pressure or raise LoggerConfig.queue_capacity",
                ["inspect BridgeHealthReport.logger.queue"],
            ),
            Self::InvalidInput(reason) => reason.remediation(),
            Self::Stopped { .. } => Remediation::not_recoverable(
                "the lifecycle owner has shut the bridge down and it cannot be reinstalled in this process",
            ),
            Self::Reentrant => Remediation::recoverable(
                "do not submit from a sink, redactor or panic hook that runs inside the logger",
                ["submit after the enclosing submission has returned"],
            ),
            Self::WriterDegraded => Remediation::recoverable(
                "restart the process: the bridge cannot reinstall the logger in-process",
                ["inspect BridgeHealthReport.logger.last_writer_error"],
            ),
            Self::BackendShutdownTimedOut => Remediation::not_recoverable(
                "the logger runtime is past its shutdown threshold; records are no longer written",
            ),
            Self::ContainedPanic => Remediation::recoverable(
                "inspect custom sinks and redactors for the panic",
                ["report the panic upstream if it comes from sc-observability"],
            ),
        }
    }

    /// The one [`DropCause`] this rejection was counted under.
    #[must_use]
    pub fn drop_cause(&self) -> DropCause {
        match self {
            Self::QueueFull => DropCause::QueueFull,
            Self::InvalidInput(_) => DropCause::InvalidEvent,
            Self::Stopped { .. } => DropCause::NotInstalled,
            Self::Reentrant => DropCause::ReentrantEmit,
            Self::WriterDegraded => DropCause::WriterDegraded,
            Self::BackendShutdownTimedOut => DropCause::ShutdownTimedOut,
            Self::ContainedPanic => DropCause::LoggerPanicked,
        }
    }

    /// Maps the cause reported by the unguarded submit core.
    pub(crate) fn from_drop_cause(cause: DropCause) -> Self {
        match cause {
            DropCause::QueueFull => Self::QueueFull,
            DropCause::InvalidEvent => Self::InvalidInput(InvalidInputReason::RejectedByLogger),
            DropCause::WriterDegraded => Self::WriterDegraded,
            DropCause::ShutdownTimedOut => Self::BackendShutdownTimedOut,
            DropCause::NotInstalled => Self::Stopped {
                lifecycle: crate::handle::lifecycle(),
            },
            DropCause::LoggerPanicked => Self::ContainedPanic,
            DropCause::ReentrantEmit => Self::Reentrant,
        }
    }
}

impl crate::handle::Rejection for SubmitError {
    fn drop_cause(&self) -> DropCause {
        SubmitError::drop_cause(self)
    }

    fn reentrant() -> Self {
        Self::Reentrant
    }

    fn panicked() -> Self {
        Self::ContainedPanic
    }
}

impl InvalidInputReason {
    fn remediation(&self) -> Remediation {
        match self {
            Self::EmptyAction => Remediation::recoverable(
                "omit the action or give it at least one character from [A-Za-z0-9._-]",
                ["resubmit the record"],
            ),
            Self::RejectedAction | Self::RejectedTarget => Remediation::recoverable(
                "use a label made of [A-Za-z0-9._-]",
                ["report the label upstream: sanitized labels are expected to validate"],
            ),
            Self::EmptyFieldKey => {
                Remediation::recoverable("remove the empty field key", ["resubmit the record"])
            }
            Self::ReservedFieldKey { .. } => Remediation::recoverable(
                "rename the field: keys starting with sc_observability_log. are owned by the bridge",
                ["resubmit the record"],
            ),
            Self::RejectedByLogger => Remediation::recoverable(
                "inspect the record contents",
                ["report the record upstream: the bridge owns every envelope field"],
            ),
        }
    }
}

/// Why an event was dropped on the non-blocking emit path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropCause {
    /// `TryLogError::QueueFull`: the bounded writer queue was full.
    QueueFull,
    /// `TryLogError::InvalidEvent`, or a runtime label or field key failing the sanitizer.
    InvalidEvent,
    /// `TryLogError::WriterDegraded`: the writer thread is degraded.
    WriterDegraded,
    /// `TryLogError::ShutdownTimedOut`: the logger exceeded its shutdown threshold.
    ShutdownTimedOut,
    /// No logger was installed (after shutdown took it), or a structured submission after shutdown.
    NotInstalled,
    /// A panic inside the emit guard: sc-observability `try_log` or a `log` record's formatting.
    LoggerPanicked,
    /// A record was emitted while the emit guard was active on the thread (formatter, hook, sink).
    ReentrantEmit,
}

impl DropCause {
    /// Every variant, in declaration order.
    pub const ALL: [DropCause; 7] = [
        Self::QueueFull,
        Self::InvalidEvent,
        Self::WriterDegraded,
        Self::ShutdownTimedOut,
        Self::NotInstalled,
        Self::LoggerPanicked,
        Self::ReentrantEmit,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_observability_types::ErrorContext;

    static NOP: NopLogger = NopLogger;
    struct NopLogger;
    impl log::Log for NopLogger {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            false
        }
        fn log(&self, _: &log::Record<'_>) {}
        fn flush(&self) {}
    }

    fn context(code: &'static str) -> Box<ErrorContext> {
        Box::new(ErrorContext::new(
            ErrorCode::new_static(code),
            "wrapped failure",
            Remediation::recoverable("wrapped step", ["second wrapped step"]),
        ))
    }

    fn assert_non_empty(remediation: &Remediation) {
        match remediation {
            Remediation::Recoverable { steps } => {
                assert!(!steps.steps().is_empty());
                assert!(steps.steps().iter().all(|step| !step.is_empty()));
            }
            Remediation::NotRecoverable { justification } => assert!(!justification.is_empty()),
        }
    }

    fn set_logger_error() -> log::SetLoggerError {
        // A unit-test-only foreign logger: the facade level stays Off, so it is inert.
        let _ = log::set_logger(&NOP);
        log::set_logger(&NOP).expect_err("second set_logger must fail")
    }

    #[test]
    fn init_error_codes_and_remediations() {
        let cases = [
            (
                InitError::AlreadyInitialized,
                error_codes::SC_OBSERVABILITY_LOG_ALREADY_INITIALIZED,
            ),
            (
                InitError::ForeignLoggerInstalled {
                    source: set_logger_error(),
                },
                error_codes::SC_OBSERVABILITY_LOG_FOREIGN_LOGGER_INSTALLED,
            ),
            (
                InitError::IdentityResolution {
                    source: sc_observability_types::IdentityError(context(
                        "SC_OBSERVABILITY_LOG_IDENTITY_RESOLUTION_FAILED",
                    )),
                },
                error_codes::SC_OBSERVABILITY_LOG_IDENTITY_RESOLUTION_FAILED,
            ),
            (
                InitError::Logger {
                    source: sc_observability_types::InitError(context(
                        "SC_OBSERVABILITY_LOGGER_INIT_FAILED",
                    )),
                },
                ErrorCode::new_static("SC_OBSERVABILITY_LOGGER_INIT_FAILED"),
            ),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code);
            assert_non_empty(&error.remediation());
        }
        let wrapped = InitError::Logger {
            source: sc_observability_types::InitError(context("W")),
        };
        assert_eq!(
            wrapped.remediation(),
            Remediation::recoverable("wrapped step", ["second wrapped step"])
        );
        let identity = InitError::IdentityResolution {
            source: sc_observability_types::IdentityError(context("I")),
        };
        assert_eq!(identity.code(), ErrorCode::new_static("I"));
        assert_eq!(
            identity.remediation(),
            Remediation::recoverable("wrapped step", ["second wrapped step"])
        );
    }

    #[test]
    fn flush_error_codes_and_remediations() {
        let cases = [
            (
                FlushError::TimedOut {
                    timeout: Duration::from_millis(5),
                },
                error_codes::SC_OBSERVABILITY_LOG_FLUSH_TIMED_OUT,
            ),
            (
                FlushError::Logger {
                    source: sc_observability_types::FlushError(context(
                        "SC_OBSERVABILITY_LOGGER_FLUSH_FAILED",
                    )),
                },
                ErrorCode::new_static("SC_OBSERVABILITY_LOGGER_FLUSH_FAILED"),
            ),
            (
                FlushError::HelperSpawn {
                    source: std::io::Error::other("spawn"),
                },
                error_codes::SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED,
            ),
            (
                FlushError::HelperLost,
                error_codes::SC_OBSERVABILITY_LOG_HELPER_LOST,
            ),
            (
                FlushError::ShutDown,
                error_codes::SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN,
            ),
            (
                FlushError::InProgress,
                error_codes::SC_OBSERVABILITY_LOG_FLUSH_IN_PROGRESS,
            ),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code);
            assert_non_empty(&error.remediation());
        }
    }

    #[test]
    fn shutdown_error_codes_and_remediations() {
        let cases = [
            (
                ShutdownError::TimedOut {
                    timeout: Duration::from_millis(5),
                },
                error_codes::SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT,
            ),
            (
                ShutdownError::FinalFlush {
                    source: sc_observability_types::FlushError(context(
                        "SC_OBSERVABILITY_LOGGER_WRITER_DEGRADED",
                    )),
                },
                ErrorCode::new_static("SC_OBSERVABILITY_LOGGER_WRITER_DEGRADED"),
            ),
            (
                ShutdownError::HelperSpawn {
                    source: std::io::Error::other("spawn"),
                },
                error_codes::SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED,
            ),
            (
                ShutdownError::HelperLost,
                error_codes::SC_OBSERVABILITY_LOG_HELPER_LOST,
            ),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code);
            assert_non_empty(&error.remediation());
        }
    }

    fn submit_errors() -> Vec<SubmitError> {
        vec![
            SubmitError::QueueFull,
            SubmitError::InvalidInput(InvalidInputReason::EmptyAction),
            SubmitError::InvalidInput(InvalidInputReason::RejectedAction),
            SubmitError::InvalidInput(InvalidInputReason::RejectedTarget),
            SubmitError::InvalidInput(InvalidInputReason::EmptyFieldKey),
            SubmitError::InvalidInput(InvalidInputReason::ReservedFieldKey {
                key: "sc_observability_log.x".to_owned(),
            }),
            SubmitError::InvalidInput(InvalidInputReason::RejectedByLogger),
            SubmitError::Stopped {
                lifecycle: BridgeLifecycle::Stopped,
            },
            SubmitError::Reentrant,
            SubmitError::WriterDegraded,
            SubmitError::BackendShutdownTimedOut,
            SubmitError::ContainedPanic,
        ]
    }

    #[test]
    fn submit_error_codes_remediations_and_drop_causes() {
        let mut causes = std::collections::HashSet::new();
        for error in submit_errors() {
            assert!(error_codes::ALL.contains(&error.code()), "{error:?}");
            assert_non_empty(&error.remediation());
            causes.insert(error.drop_cause());
            if !matches!(
                error,
                SubmitError::InvalidInput(ref reason) if *reason != InvalidInputReason::RejectedByLogger
            ) && !matches!(error, SubmitError::Stopped { .. })
            {
                assert_eq!(SubmitError::from_drop_cause(error.drop_cause()), error);
            }
        }
        assert_eq!(causes.len(), DropCause::ALL.len(), "one cause per variant");
    }

    #[test]
    fn submit_error_serializes_with_tagged_discriminants() {
        let reserved = SubmitError::InvalidInput(InvalidInputReason::ReservedFieldKey {
            key: "sc_observability_log.x".to_owned(),
        });
        let json = serde_json::to_value(&reserved).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"kind": "invalid_input", "reason": "reserved_field_key", "key": "sc_observability_log.x"})
        );
        let stopped = SubmitError::Stopped {
            lifecycle: BridgeLifecycle::ShutdownTimedOut,
        };
        assert_eq!(
            serde_json::to_value(&stopped).unwrap(),
            serde_json::json!({"kind": "stopped", "lifecycle": "shutdown_timed_out"})
        );
        for error in submit_errors() {
            let round_trip: SubmitError =
                serde_json::from_value(serde_json::to_value(&error).unwrap()).unwrap();
            assert_eq!(round_trip, error);
        }
    }

    #[test]
    fn drop_cause_all_lists_every_variant_once() {
        let unique: std::collections::HashSet<_> = DropCause::ALL.iter().collect();
        assert_eq!(unique.len(), DropCause::ALL.len());
    }
}
