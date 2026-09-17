//! Stable `ErrorCode` registry for `sc-observability-log`.
//!
//! Every crate-owned public error variant maps to exactly one code here. Variants
//! that wrap an sc-observability error return that error's own code instead.

use sc_observability_types::ErrorCode;

/// `InitError::AlreadyInitialized`: the bridge was already installed in this process.
pub const SC_OBSERVABILITY_LOG_ALREADY_INITIALIZED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_ALREADY_INITIALIZED");
/// `InitError::ForeignLoggerInstalled`: another `log::Log` implementation owns the facade.
pub const SC_OBSERVABILITY_LOG_FOREIGN_LOGGER_INSTALLED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_FOREIGN_LOGGER_INSTALLED");
/// `InitError::IdentityResolution`: the identity resolver failed or `Auto` found no hostname.
pub const SC_OBSERVABILITY_LOG_IDENTITY_RESOLUTION_FAILED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_IDENTITY_RESOLUTION_FAILED");
/// `FlushError::TimedOut`: the writer did not acknowledge a flush within the timeout.
pub const SC_OBSERVABILITY_LOG_FLUSH_TIMED_OUT: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_FLUSH_TIMED_OUT");
/// `ShutdownError::TimedOut`: shutdown did not finish within the timeout.
pub const SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT");
/// `FlushError::HelperSpawn` / `ShutdownError::HelperSpawn`: the helper thread could not start.
pub const SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED");
/// `FlushError::HelperLost` / `ShutdownError::HelperLost`: the helper thread ended without a result.
pub const SC_OBSERVABILITY_LOG_HELPER_LOST: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_HELPER_LOST");
/// `InitError::UnsupportedLevel`: the executable's static facade cap cannot retain the baseline.
pub const SC_OBSERVABILITY_LOG_UNSUPPORTED_LEVEL: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_UNSUPPORTED_LEVEL");
/// Direct runtime setup failed before bridge admission.
pub const SC_OBSERVABILITY_LOG_RUNTIME_START_FAILED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_RUNTIME_START_FAILED");
/// A direct/control request arrived outside the running lifecycle.
pub const SC_OBSERVABILITY_LOG_NOT_RUNNING: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_NOT_RUNNING");
/// A typed direct event supplied an invalid field key.
pub const SC_OBSERVABILITY_LOG_INVALID_FIELD: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_INVALID_FIELD");
/// A producer re-entered the guarded bridge submission path.
pub const SC_OBSERVABILITY_LOG_REENTRANT_EMIT: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_REENTRANT_EMIT");
/// A callback panic was contained by the bridge admission boundary.
pub const SC_OBSERVABILITY_LOG_LOGGER_PANICKED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_LOGGER_PANICKED");
/// A read-only state/query observation is unavailable.
pub const SC_OBSERVABILITY_LOG_STATUS_UNAVAILABLE: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_STATUS_UNAVAILABLE");
/// A wait request was made before initialization.
pub const SC_OBSERVABILITY_LOG_SHUTDOWN_NOT_STARTED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_SHUTDOWN_NOT_STARTED");
/// `FlushError::ShutDown`: a flush was requested after shutdown had started.
pub const SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN");
/// `FlushError::InProgress`: a previous flush helper is still running; no new flush was started.
pub const SC_OBSERVABILITY_LOG_FLUSH_IN_PROGRESS: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_FLUSH_IN_PROGRESS");
/// `SubmitError::QueueFull`: the writer queue was full.
pub const SC_OBSERVABILITY_LOG_SUBMIT_QUEUE_FULL: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_SUBMIT_QUEUE_FULL");
/// `SubmitError::InvalidInput`: the structured record was invalid; see its reason.
pub const SC_OBSERVABILITY_LOG_SUBMIT_INVALID_INPUT: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_SUBMIT_INVALID_INPUT");
/// `SubmitError::Stopped`: the bridge lifecycle is not running.
pub const SC_OBSERVABILITY_LOG_SUBMIT_STOPPED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_SUBMIT_STOPPED");
/// `SubmitError::Reentrant`: submitted from inside another submission on the same thread.
pub const SC_OBSERVABILITY_LOG_SUBMIT_REENTRANT: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_SUBMIT_REENTRANT");
/// `SubmitError::WriterDegraded`: the writer thread is degraded.
pub const SC_OBSERVABILITY_LOG_SUBMIT_WRITER_DEGRADED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_SUBMIT_WRITER_DEGRADED");
/// `SubmitError::BackendShutdownTimedOut`: the logger runtime exceeded its shutdown threshold.
pub const SC_OBSERVABILITY_LOG_SUBMIT_BACKEND_SHUTDOWN_TIMED_OUT: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_SUBMIT_BACKEND_SHUTDOWN_TIMED_OUT");
/// `SubmitError::ContainedPanic`: a panic inside the logger, a sink or a redactor was contained.
pub const SC_OBSERVABILITY_LOG_SUBMIT_CONTAINED_PANIC: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_SUBMIT_CONTAINED_PANIC");

/// Every code defined by this crate, in declaration order.
pub const ALL: &[ErrorCode] = &[
    SC_OBSERVABILITY_LOG_ALREADY_INITIALIZED,
    SC_OBSERVABILITY_LOG_FOREIGN_LOGGER_INSTALLED,
    SC_OBSERVABILITY_LOG_IDENTITY_RESOLUTION_FAILED,
    SC_OBSERVABILITY_LOG_FLUSH_TIMED_OUT,
    SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT,
    SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED,
    SC_OBSERVABILITY_LOG_HELPER_LOST,
    SC_OBSERVABILITY_LOG_UNSUPPORTED_LEVEL,
    SC_OBSERVABILITY_LOG_RUNTIME_START_FAILED,
    SC_OBSERVABILITY_LOG_NOT_RUNNING,
    SC_OBSERVABILITY_LOG_INVALID_FIELD,
    SC_OBSERVABILITY_LOG_REENTRANT_EMIT,
    SC_OBSERVABILITY_LOG_LOGGER_PANICKED,
    SC_OBSERVABILITY_LOG_STATUS_UNAVAILABLE,
    SC_OBSERVABILITY_LOG_SHUTDOWN_NOT_STARTED,
    SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN,
    SC_OBSERVABILITY_LOG_FLUSH_IN_PROGRESS,
    SC_OBSERVABILITY_LOG_SUBMIT_QUEUE_FULL,
    SC_OBSERVABILITY_LOG_SUBMIT_INVALID_INPUT,
    SC_OBSERVABILITY_LOG_SUBMIT_STOPPED,
    SC_OBSERVABILITY_LOG_SUBMIT_REENTRANT,
    SC_OBSERVABILITY_LOG_SUBMIT_WRITER_DEGRADED,
    SC_OBSERVABILITY_LOG_SUBMIT_BACKEND_SHUTDOWN_TIMED_OUT,
    SC_OBSERVABILITY_LOG_SUBMIT_CONTAINED_PANIC,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_codes_are_unique_and_prefixed() {
        let mut seen = std::collections::HashSet::new();
        for code in ALL {
            assert!(code.as_str().starts_with("SC_OBSERVABILITY_LOG_"));
            assert!(seen.insert(code.as_str()), "duplicate code {code}");
        }
        assert_eq!(ALL.len(), 24);
    }
}
