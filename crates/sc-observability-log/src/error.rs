//! Public error enums and the dropped-event cause.
//!
//! Every error type is a discriminated union: callers `match` on the variant and
//! read its typed fields. Each public error enum also exposes a stable
//! [`ErrorCode`] and a mandatory [`Remediation`] per variant; variants wrapping an
//! sc-observability error return that error's own code and remediation.

use std::time::Duration;

use sc_observability_types::{DiagnosticInfo, ErrorCode, Remediation};

use crate::error_codes;

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
    /// `ProcessIdentityPolicy::Resolver` failed.
    #[error("process identity resolution failed")]
    IdentityResolution {
        /// Error reported by the resolver.
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

/// Error returned by [`LogGuard::flush`](crate::LogGuard::flush).
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
            Self::IdentityResolution { .. } => {
                error_codes::SC_OBSERVABILITY_LOG_IDENTITY_RESOLUTION_FAILED
            }
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
            Self::IdentityResolution { .. } => Remediation::recoverable(
                "fix the ProcessIdentityResolver, or use ProcessIdentityPolicy::Auto or Fixed",
                ["call sc_observability_log::init again"],
            ),
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
    /// No logger was installed (before `init` or after shutdown).
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
                    source: sc_observability_types::IdentityError(context("X_IDENTITY")),
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

    #[test]
    fn drop_cause_all_lists_every_variant_once() {
        let unique: std::collections::HashSet<_> = DropCause::ALL.iter().collect();
        assert_eq!(unique.len(), DropCause::ALL.len());
    }
}
