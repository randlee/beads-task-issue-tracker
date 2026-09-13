#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
//! sc-observability-log bridge wiring: install, exit shutdown, and the log commands.
//!
//! The active log file is `<app_log_dir>/logs/beads-task-issue-tracker.log.jsonl`
//! (`LogGuard::active_log_path`, captured once at [`install_logging`]). Every
//! entry point that touches the process-wide guard — [`install_logging`],
//! [`on_run_event`] and [`clear_logs`] — takes or clones the `Arc` and releases
//! the `LOG_GUARD` lock before doing any I/O, so no code path holds the lock
//! across a flush or a shutdown.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::Duration;

use sc_observability_log::{
    ActionName, BridgeOptions, DropCause, LevelFilter, LogGuard, LoggerConfig, ServiceName, init,
};
use tauri::Manager;

// Global flags for logging
pub(crate) static LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) static VERBOSE_LOGGING: AtomicBool = AtomicBool::new(false);

// Conditional logging macros
macro_rules! log_info {
    ($($arg:tt)*) => {
        if $crate::logging::LOGGING_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            log::info!($($arg)*);
        }
    };
}

macro_rules! log_warn {
    ($($arg:tt)*) => {
        if $crate::logging::LOGGING_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            log::warn!($($arg)*);
        }
    };
}

macro_rules! log_error {
    ($($arg:tt)*) => {
        if $crate::logging::LOGGING_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            log::error!($($arg)*);
        }
    };
}

macro_rules! log_debug {
    ($($arg:tt)*) => {
        if $crate::logging::LOGGING_ENABLED.load(std::sync::atomic::Ordering::Relaxed) && $crate::logging::VERBOSE_LOGGING.load(std::sync::atomic::Ordering::Relaxed) {
            log::debug!($($arg)*);
        }
    };
}

// ============================================================================
// Bridge lifecycle
// ============================================================================

static LOG_GUARD: Mutex<Option<Arc<LogGuard>>> = Mutex::new(None);
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Bound on every blocking bridge call this module makes (`flush`/`shutdown`).
const LOG_IO_TIMEOUT: Duration = Duration::from_secs(2);

/// Installs the sc-observability-log bridge as the process-wide `log` logger.
///
/// Must be called first in `run()`'s `setup`, before any other startup logging.
///
/// # Errors
///
/// Propagates [`sc_observability_log::InitError`] (via `?` at the call site,
/// which fails `setup`) and a `ValueValidationError` from `ServiceName::new` /
/// `ActionName::new`, boxed through `Box<dyn std::error::Error>`. The latter
/// cannot occur in practice: both names are constant literals.
pub(crate) fn install_logging(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let log_root = app.path().app_log_dir()?;
    let mut config =
        LoggerConfig::default_for(ServiceName::new("beads-task-issue-tracker")?, log_root);
    config.level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    config.enable_console_sink = true;
    let guard = init(
        config,
        BridgeOptions {
            default_action: ActionName::new("log")?,
            parse_bracket_action: true,
        },
    )?;
    if let Some(path) = guard.active_log_path() {
        let _ = LOG_PATH.set(path.to_path_buf());
    }
    *LOG_GUARD.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(guard));
    Ok(())
}

/// Runs on every Tauri run-loop event; only `RunEvent::Exit` does anything.
///
/// Takes the guard out of `LOG_GUARD` and releases the lock in the same
/// statement, so exit never waits behind a `clear_logs` flush. Logs one `warn`
/// with the per-`DropCause` counts when any events were dropped, then makes
/// exactly one bounded call: `shutdown` when this is the sole owner, otherwise
/// `flush` on the still-shared clone (a `clear_logs` flush holds the other
/// clone at that instant). Either branch is bounded by 1x `LOG_IO_TIMEOUT`.
pub(crate) fn on_run_event(_app: &tauri::AppHandle, event: &tauri::RunEvent) {
    if !matches!(event, tauri::RunEvent::Exit) {
        return;
    }
    let taken = LOG_GUARD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    let Some(shared) = taken else {
        return;
    };
    let dropped = shared.dropped_events();
    if dropped.total() > 0 {
        let summary: Vec<String> = DropCause::ALL
            .iter()
            .filter(|cause| dropped.get(**cause) > 0)
            .map(|cause| format!("{cause:?}={}", dropped.get(*cause)))
            .collect();
        log::warn!("[logging] dropped events: {}", summary.join(", "));
    }
    match Arc::try_unwrap(shared) {
        Ok(guard) => {
            if let Err(e) = guard.shutdown(LOG_IO_TIMEOUT) {
                let _ = writeln!(
                    std::io::stderr(),
                    "beads-task-issue-tracker: log shutdown failed: {e:?}"
                );
            }
        }
        // A clear_logs flush holds the other clone: flush once instead (still
        // bounded by 1x LOG_IO_TIMEOUT). Shutdown cannot consume a shared guard.
        Err(shared) => {
            if let Err(e) = shared.flush(LOG_IO_TIMEOUT) {
                let _ = writeln!(
                    std::io::stderr(),
                    "beads-task-issue-tracker: log flush at exit failed: {e:?}"
                );
            }
        }
    }
}

// ============================================================================
// Debug / Logging Commands
// ============================================================================

#[tauri::command]
pub(crate) async fn get_logging_enabled() -> bool {
    LOGGING_ENABLED.load(Ordering::Relaxed)
}

#[tauri::command]
pub(crate) async fn set_logging_enabled(enabled: bool) {
    LOGGING_ENABLED.store(enabled, Ordering::Relaxed);
    if enabled {
        log_info!("[debug] Logging enabled");
    }
}

#[tauri::command]
pub(crate) async fn get_verbose_logging() -> bool {
    VERBOSE_LOGGING.load(Ordering::Relaxed)
}

#[tauri::command]
pub(crate) async fn set_verbose_logging(enabled: bool) {
    VERBOSE_LOGGING.store(enabled, Ordering::Relaxed);
    log_info!(
        "[debug] Verbose logging: {}",
        if enabled { "ON" } else { "OFF" }
    );
}

#[tauri::command]
pub(crate) async fn clear_logs() -> Result<(), String> {
    // Clone the Arc and release the lock before flushing: an exit that arrives
    // during the flush takes the guard at once and stays bounded by
    // 1x LOG_IO_TIMEOUT.
    let shared = LOG_GUARD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    if let Some(guard) = shared {
        guard
            .flush(LOG_IO_TIMEOUT)
            .map_err(|e| format!("Failed to flush logs: {e}"))?;
    } // None: logger not installed (before setup) or already taken at exit — no flush.
    let log_path = get_log_path();
    if log_path.as_os_str().is_empty() || !log_path.exists() {
        return Ok(());
    }
    fs::write(&log_path, "").map_err(|e| format!("Failed to clear logs: {e}"))?;
    remove_rotated_logs(&log_path)?;
    log_info!("[debug] Logs cleared");
    Ok(())
}

/// Deletes `<active file name>.<N>` siblings, the names sc-observability rotates to.
fn remove_rotated_logs(active: &Path) -> Result<(), String> {
    let (Some(dir), Some(name)) = (
        active.parent(),
        active.file_name().and_then(|n| n.to_str()),
    ) else {
        return Ok(());
    };
    let prefix = format!("{name}.");
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to list log dir: {e}"))?;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(candidate) = file_name.to_str() else {
            continue;
        };
        let is_rotated = candidate
            .strip_prefix(prefix.as_str())
            .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()));
        if is_rotated {
            fs::remove_file(entry.path())
                .map_err(|e| format!("Failed to remove rotated log {candidate}: {e}"))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn export_logs() -> Result<String, String> {
    let log_path = get_log_path();
    if log_path.as_os_str().is_empty() || !log_path.exists() {
        return Err("No logs to export".to_string());
    }

    // Get export folder: Downloads > Documents > Home
    let export_dir = dirs::download_dir()
        .or_else(dirs::document_dir)
        .or_else(dirs::home_dir)
        .ok_or_else(|| "Could not find a folder to export logs".to_string())?;

    // Generate filename with timestamp
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let export_filename = format!("beads-logs-{now}.log.jsonl");
    let export_path = export_dir.join(&export_filename);

    // Copy log file
    fs::copy(&log_path, &export_path).map_err(|e| format!("Failed to export logs: {e}"))?;

    Ok(export_path.to_string_lossy().to_string())
}

#[tauri::command]
pub(crate) async fn read_logs(tail_lines: Option<usize>) -> Result<String, String> {
    let log_path = get_log_path();
    if log_path.as_os_str().is_empty() || !log_path.exists() {
        return Ok(String::new());
    }

    let content = fs::read_to_string(&log_path).map_err(|e| format!("Failed to read logs: {e}"))?;

    // If tail_lines is specified, return only the last N lines
    if let Some(n) = tail_lines {
        let lines: Vec<&str> = content.lines().collect();
        let start = lines.len().saturating_sub(n);
        Ok(lines.get(start..).unwrap_or_default().join("\n"))
    } else {
        Ok(content)
    }
}

/// `LogGuard::active_log_path` captured at `install_logging`; empty before setup.
pub(crate) fn get_log_path() -> PathBuf {
    LOG_PATH.get().cloned().unwrap_or_default()
}

#[tauri::command]
pub(crate) async fn get_log_path_string() -> String {
    get_log_path().to_string_lossy().to_string()
}

#[tauri::command]
pub(crate) async fn log_frontend(level: String, message: String) {
    match level.as_str() {
        "error" => log::log!(target: "frontend", log::Level::Error, "{}", message),
        "warn" => log::log!(target: "frontend", log::Level::Warn, "{}", message),
        "info" => log::log!(target: "frontend", log::Level::Info, "{}", message),
        // Unknown level (should not happen: the TypeScript wrapper restricts the
        // level to error/warn/info): log at Info, same as today, but keep the raw
        // value visible in JSONL as a kv field.
        other => {
            log::log!(target: "frontend", log::Level::Info, frontend_level = other; "{}", message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test-local error: setup/teardown failures fail the test via `?`
    /// (returning `Result` from a `#[test]` fn) rather than panicking, so this
    /// module stays clean under the repo's no-panic grep check.
    #[derive(Debug)]
    struct TestError(String);

    impl<E: std::fmt::Display> From<E> for TestError {
        fn from(e: E) -> Self {
            Self(e.to_string())
        }
    }

    /// Unique scratch directory under `std::env::temp_dir()`, removed on drop.
    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn new(label: &str) -> Result<Self, TestError> {
            let unique = format!(
                "btit-logging-test-{label}-{}-{:?}",
                std::process::id(),
                std::time::Instant::now()
            );
            let dir = std::env::temp_dir().join(unique);
            fs::create_dir_all(&dir)?;
            Ok(Self(dir))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn get_log_path_is_empty_before_install_logging_runs() {
        // LOG_PATH is a OnceLock only ever set by `install_logging`, which this
        // test never calls.
        assert_eq!(get_log_path(), PathBuf::new());
    }

    #[tokio::test]
    async fn read_logs_returns_empty_string_when_no_path_is_set() {
        assert_eq!(read_logs(Some(10)).await, Ok(String::new()));
        assert_eq!(read_logs(None).await, Ok(String::new()));
    }

    #[tokio::test]
    async fn clear_logs_is_a_noop_when_no_guard_and_no_existing_file() {
        // LOG_GUARD is None (install_logging never ran) and get_log_path() is
        // empty, so clear_logs must return Ok(()) without touching the filesystem.
        assert_eq!(clear_logs().await, Ok(()));
    }

    #[tokio::test]
    async fn export_logs_reports_an_error_when_no_path_is_set() {
        assert!(export_logs().await.is_err());
    }

    #[test]
    fn remove_rotated_logs_deletes_only_numeric_suffixed_siblings() -> Result<(), TestError> {
        let scratch = ScratchDir::new("rotated")?;
        let active = scratch.path().join("beads-task-issue-tracker.log.jsonl");
        fs::write(&active, "active")?;
        let rotated_one = scratch
            .path()
            .join("beads-task-issue-tracker.log.jsonl.1");
        fs::write(&rotated_one, "rotated")?;
        let rotated_two = scratch
            .path()
            .join("beads-task-issue-tracker.log.jsonl.2");
        fs::write(&rotated_two, "rotated")?;
        let unrelated = scratch.path().join("beads-task-issue-tracker.log.jsonl.bak");
        fs::write(&unrelated, "kept")?;

        remove_rotated_logs(&active)?;

        assert!(active.exists(), "the active file must not be removed");
        assert!(!rotated_one.exists(), "rotated.1 must be removed");
        assert!(!rotated_two.exists(), "rotated.2 must be removed");
        assert!(
            unrelated.exists(),
            "a non-numeric suffix must not be treated as rotated"
        );
        Ok(())
    }

    #[test]
    fn remove_rotated_logs_is_a_noop_when_the_directory_has_no_siblings() -> Result<(), TestError>
    {
        let scratch = ScratchDir::new("no-siblings")?;
        let active = scratch.path().join("beads-task-issue-tracker.log.jsonl");
        fs::write(&active, "active")?;

        assert_eq!(remove_rotated_logs(&active), Ok(()));
        assert!(active.exists());
        Ok(())
    }
}
