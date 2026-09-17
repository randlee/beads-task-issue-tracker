//! Serializable, versioned projection of every public error.
//!
//! Rust callers match on [`InitError`], [`FlushError`], [`ShutdownError`] and
//! [`SubmitError`] directly. Consumers across a language boundary (generated
//! TypeScript or Python bindings, a frontend command channel) receive an
//! [`FailureReport`] instead: plain data with a schema version, a tagged failure
//! discriminant, the stable [`ErrorCode`] and the [`Remediation`]. The report
//! holds no source error, no ownership handle and nothing that must be parsed
//! out of a display string; `message` is informational only.
//!
//! # Wire shape
//!
//! ```json
//! {
//!   "schema_version": 1,
//!   "failure": { "operation": "submit", "kind": "invalid_input",
//!                "reason": "reserved_field_key", "key": "sc_observability_log.x" },
//!   "code": "SC_OBSERVABILITY_LOG_SUBMIT_INVALID_INPUT",
//!   "message": "invalid structured record: ...",
//!   "remediation": { "kind": "recoverable", "steps": ["..."] }
//! }
//! ```
//!
//! `failure` is internally tagged by `operation` (`init`, `flush`, `shutdown`,
//! `submit`) and then by `kind`; every tag value is `snake_case`.

use std::time::Duration;

use sc_observability_types::{ErrorCode, Remediation};
use serde::{Deserialize, Serialize};

use crate::{FlushError, InitError, ShutdownError, SubmitError};

/// Version of the control contract: [`FailureReport`], [`StructuredRecord`](crate::StructuredRecord)
/// and [`SubmitOutcome`](crate::SubmitOutcome). Bumped whenever a field or variant changes.
pub const CONTROL_SCHEMA_VERSION: u32 = 1;

/// Serializable report of one failed bridge operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FailureReport {
    /// [`CONTROL_SCHEMA_VERSION`] of the producer.
    pub schema_version: u32,
    /// Which operation failed and how; match on this, never on `message`.
    pub failure: Failure,
    /// Stable code; for a wrapped sc-observability error, that error's own code.
    pub code: ErrorCode,
    /// Human-readable summary (informational).
    pub message: String,
    /// What to do about it.
    pub remediation: Remediation,
}

/// The failed operation and its discriminant; internally tagged by `operation`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum Failure {
    /// [`init`](crate::init) failed.
    Init(InitFailure),
    /// A bounded flush failed.
    Flush(FlushFailure),
    /// The final shutdown failed.
    Shutdown(ShutdownFailure),
    /// A structured submission was rejected.
    Submit(SubmitError),
}

/// Data-only discriminant of [`InitError`]; tagged by `kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InitFailure {
    /// [`InitError::AlreadyInitialized`].
    AlreadyInitialized,
    /// [`InitError::ForeignLoggerInstalled`].
    ForeignLoggerInstalled,
    /// [`InitError::IdentityResolution`].
    IdentityResolution,
    /// [`InitError::Logger`].
    Logger,
}

/// Data-only discriminant of [`FlushError`]; tagged by `kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FlushFailure {
    /// [`FlushError::TimedOut`]: the flush may still complete on a detached helper.
    TimedOut {
        /// The elapsed timeout in milliseconds (saturating).
        timeout_ms: u64,
    },
    /// [`FlushError::Logger`].
    Logger,
    /// [`FlushError::HelperSpawn`].
    HelperSpawn,
    /// [`FlushError::HelperLost`].
    HelperLost,
    /// [`FlushError::NotRunning`].
    ShutDown,
    /// [`FlushError::InProgress`]: a previous flush helper is still running.
    InProgress,
}

/// Data-only discriminant of [`ShutdownError`]; tagged by `kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShutdownFailure {
    /// [`ShutdownError::TimedOut`]: completion continues on a detached helper and
    /// is observable as `BridgeLifecycle::ShutdownTimedOut` -> `Stopped`.
    TimedOut {
        /// The elapsed timeout in milliseconds (saturating).
        timeout_ms: u64,
    },
    /// [`ShutdownError::FinalFlush`].
    FinalFlush,
    /// [`ShutdownError::HelperSpawn`].
    HelperSpawn,
    /// [`ShutdownError::HelperLost`].
    HelperLost,
}

fn millis(timeout: Duration) -> u64 {
    u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX)
}

fn report(
    failure: Failure,
    code: ErrorCode,
    message: String,
    remediation: Remediation,
) -> FailureReport {
    FailureReport {
        schema_version: CONTROL_SCHEMA_VERSION,
        failure,
        code,
        message,
        remediation,
    }
}

impl InitError {
    /// Serializable projection: discriminant, [`code`](Self::code) and [`remediation`](Self::remediation).
    #[must_use]
    pub fn report(&self) -> FailureReport {
        let failure = match self {
            Self::AlreadyInitialized => InitFailure::AlreadyInitialized,
            Self::ForeignLoggerInstalled { .. } => InitFailure::ForeignLoggerInstalled,
            Self::UnsupportedLevel { .. } | Self::Logger { .. } => InitFailure::Logger,
            Self::IdentityResolution { .. } => InitFailure::IdentityResolution,
        };
        report(
            Failure::Init(failure),
            self.code(),
            self.to_string(),
            self.remediation(),
        )
    }
}

impl FlushError {
    /// Serializable projection: discriminant, [`code`](Self::code) and [`remediation`](Self::remediation).
    #[must_use]
    pub fn report(&self) -> FailureReport {
        let failure = match self {
            Self::TimedOut { timeout } => FlushFailure::TimedOut {
                timeout_ms: millis(*timeout),
            },
            Self::Logger { .. } => FlushFailure::Logger,
            Self::HelperSpawn { .. } => FlushFailure::HelperSpawn,
            Self::HelperLost => FlushFailure::HelperLost,
            Self::NotRunning { .. } => FlushFailure::ShutDown,
            Self::InProgress => FlushFailure::InProgress,
        };
        report(
            Failure::Flush(failure),
            self.code(),
            self.to_string(),
            self.remediation(),
        )
    }
}

impl ShutdownError {
    /// Serializable projection: discriminant, [`code`](Self::code) and [`remediation`](Self::remediation).
    #[must_use]
    pub fn report(&self) -> FailureReport {
        let failure = match self {
            Self::TimedOut { timeout } => ShutdownFailure::TimedOut {
                timeout_ms: millis(*timeout),
            },
            Self::FinalFlush { .. } => ShutdownFailure::FinalFlush,
            Self::HelperSpawn { .. } => ShutdownFailure::HelperSpawn,
            Self::HelperLost => ShutdownFailure::HelperLost,
        };
        report(
            Failure::Shutdown(failure),
            self.code(),
            self.to_string(),
            self.remediation(),
        )
    }
}

impl SubmitError {
    /// Serializable projection: the error itself, [`code`](Self::code) and [`remediation`](Self::remediation).
    #[must_use]
    pub fn report(&self) -> FailureReport {
        report(
            Failure::Submit(self.clone()),
            self.code(),
            self.to_string(),
            self.remediation(),
        )
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{BridgeLifecycle, InvalidInputReason, error_codes};

    #[test]
    fn submit_report_has_tagged_failure_code_and_remediation() {
        let error = SubmitError::InvalidInput(InvalidInputReason::ReservedFieldKey {
            key: "sc_observability_log.x".to_owned(),
        });
        let json = serde_json::to_value(error.report()).unwrap();
        assert_eq!(json["schema_version"], CONTROL_SCHEMA_VERSION);
        assert_eq!(
            json["failure"],
            json!({"operation": "submit", "kind": "invalid_input", "reason": "reserved_field_key", "key": "sc_observability_log.x"})
        );
        assert_eq!(json["code"], "SC_OBSERVABILITY_LOG_SUBMIT_INVALID_INPUT");
        assert_eq!(json["remediation"]["kind"], "recoverable");
        let decoded: FailureReport = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, error.report());
    }

    #[test]
    fn every_operation_round_trips() {
        let reports = [
            InitError::AlreadyInitialized.report(),
            FlushError::TimedOut {
                timeout: Duration::from_millis(1500),
            }
            .report(),
            FlushError::NotRunning {
                phase: crate::LifecyclePhase::Stopped,
            }
            .report(),
            ShutdownError::TimedOut {
                timeout: Duration::from_secs(2),
            }
            .report(),
            ShutdownError::HelperLost.report(),
            SubmitError::Stopped {
                lifecycle: BridgeLifecycle::Stopped,
            }
            .report(),
            SubmitError::QueueFull.report(),
        ];
        for report in &reports {
            let json = serde_json::to_value(report).unwrap();
            assert!(json["failure"]["operation"].is_string());
            assert!(json["failure"]["kind"].is_string());
            let decoded: FailureReport = serde_json::from_value(json).unwrap();
            assert_eq!(&decoded, report);
        }
        assert_eq!(
            serde_json::to_value(&reports[1].failure).unwrap(),
            json!({"operation": "flush", "kind": "timed_out", "timeout_ms": 1500})
        );
        assert_eq!(
            reports[3].code,
            error_codes::SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT
        );
        assert_eq!(millis(Duration::MAX), u64::MAX);
    }

    #[test]
    fn flush_in_progress_report_round_trips() {
        let error = FlushError::InProgress;
        assert_eq!(
            error.code(),
            error_codes::SC_OBSERVABILITY_LOG_FLUSH_IN_PROGRESS
        );
        assert!(matches!(
            error.remediation(),
            Remediation::Recoverable { .. }
        ));
        let report = error.report();
        assert_eq!(report.failure, Failure::Flush(FlushFailure::InProgress));
        assert_eq!(report.code, error.code());
        assert_eq!(report.remediation, error.remediation());
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(
            json["failure"],
            json!({"operation": "flush", "kind": "in_progress"})
        );
        assert_eq!(json["code"], "SC_OBSERVABILITY_LOG_FLUSH_IN_PROGRESS");
        let decoded: FailureReport = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, report);
    }
}
