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
/// `InitError::IdentityResolution`: the configured process identity resolver failed.
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
/// `FlushError::ShutDown`: a flush was requested after shutdown had started.
pub const SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN");

/// Every code defined by this crate, in declaration order.
pub const ALL: &[ErrorCode] = &[
    SC_OBSERVABILITY_LOG_ALREADY_INITIALIZED,
    SC_OBSERVABILITY_LOG_FOREIGN_LOGGER_INSTALLED,
    SC_OBSERVABILITY_LOG_IDENTITY_RESOLUTION_FAILED,
    SC_OBSERVABILITY_LOG_FLUSH_TIMED_OUT,
    SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT,
    SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED,
    SC_OBSERVABILITY_LOG_HELPER_LOST,
    SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN,
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
        assert_eq!(ALL.len(), 8);
    }
}
