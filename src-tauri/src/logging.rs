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
//! (`LogGuard::active_log_path`, captured once at [`install_logging`]). The guard
//! has exactly one owner, the static [`LOGGING`] lifecycle (see
//! [`lifecycle`]): [`install_logging`] hands the guard to it, [`clear_logs`]
//! flushes through a non-owning `LogControl` and is serialized with exit, and
//! [`on_run_event`] performs the single final shutdown on `RunEvent::Exit`. No
//! code path holds the lifecycle lock across a flush, a shutdown or file I/O.

mod lifecycle;

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use sc_observability_log::{
    init, ActionName, BridgeOptions, FlushError, LevelFilter, LogGuard, LoggerConfig, ServiceName,
};
use tauri::Manager;

use lifecycle::{ExitOutcome, LogLifecycle};

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

/// The single owner of the process-wide `LogGuard`.
static LOGGING: LogLifecycle<LogGuard> = LogLifecycle::new();
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Bound on every blocking bridge call this module makes: one `flush` in
/// `clear_logs`, and the whole exit sequence (waiting for an in-progress clear
/// plus the final `shutdown`). Two seconds keeps quit responsive while leaving
/// the writer time to drain a normal queue.
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
    LOGGING.install(guard, LOG_IO_TIMEOUT)?;
    Ok(())
}

/// Runs on every Tauri run-loop event; only `RunEvent::Exit` does anything.
///
/// Delegates to [`LogLifecycle::exit`](lifecycle::LogLifecycle::exit): the
/// guard's single final shutdown, bounded by 1x `LOG_IO_TIMEOUT` in total even
/// while `clear_logs` is flushing or truncating. The shutdown result is
/// reported here: a failure is written to stderr (the logger is gone, and
/// `eprintln!` panics on a closed stream). A repeated `Exit` does nothing.
pub(crate) fn on_run_event(_app: &tauri::AppHandle, event: &tauri::RunEvent) {
    if !matches!(event, tauri::RunEvent::Exit) {
        return;
    }
    report_exit(&LOGGING.exit(LOG_IO_TIMEOUT));
}

/// Writes a failed final shutdown to stderr; success and no-op exits are silent.
fn report_exit(outcome: &ExitOutcome) {
    if let ExitOutcome::ShutDown { result: Err(e) } = outcome {
        let _ = writeln!(
            std::io::stderr(),
            "beads-task-issue-tracker: log shutdown failed [{}]: {e}",
            e.code()
        );
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
    // The lifecycle hands out a non-owning control, never the guard, and rejects
    // the clear once exit has begun; see `lifecycle` for the serialization.
    let log_path = get_log_path();
    tauri::async_runtime::spawn_blocking(move || {
        LOGGING
            .clear(
                &log_path,
                |control| control.flush(LOG_IO_TIMEOUT),
                read_log_dir,
            )
            .map_err(|e| e.to_string())?;
        log_info!("[debug] Logs cleared");
        Ok(())
    })
    .await
    .map_err(|e| format!("Failed to clear logs: {e}"))?
}

/// Why `clear_logs` failed; rendered into the command's `Err(String)`.
#[derive(Debug)]
pub(crate) enum ClearError {
    /// Exit has begun: nothing was flushed or removed, so the final records survive.
    ShutdownStarted,
    /// The bounded flush before clearing failed; no file was touched.
    Flush { source: FlushError },
    /// Truncating the active JSONL file failed.
    Truncate { source: std::io::Error },
    /// The log directory could not be opened for listing.
    ListDir { source: std::io::Error },
    /// One directory entry could not be read while listing.
    ReadDirEntry { source: std::io::Error },
    /// A rotated `<active>.<N>` file could not be removed.
    RemoveRotated {
        file_name: String,
        source: std::io::Error,
    },
}

impl std::fmt::Display for ClearError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ShutdownStarted => f.write_str("Logs were not cleared: logging is shutting down"),
            Self::Flush { source } => write!(f, "Failed to flush logs: {source}"),
            Self::Truncate { source } => write!(f, "Failed to clear logs: {source}"),
            Self::ListDir { source } => write!(f, "Failed to list log dir: {source}"),
            Self::ReadDirEntry { source } => write!(f, "Failed to read log dir entry: {source}"),
            Self::RemoveRotated { file_name, source } => {
                write!(f, "Failed to remove rotated log {file_name}: {source}")
            }
        }
    }
}

impl std::error::Error for ClearError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ShutdownStarted => None,
            Self::Flush { source } => Some(source),
            Self::Truncate { source }
            | Self::ListDir { source }
            | Self::ReadDirEntry { source }
            | Self::RemoveRotated { source, .. } => Some(source),
        }
    }
}

/// One directory entry, as `remove_rotated_logs` needs it.
#[derive(Debug)]
struct LogDirEntry {
    file_name: std::ffi::OsString,
    path: PathBuf,
}

/// Lists `dir` with `fs::read_dir`; the production reader for [`clear_log_files`].
fn read_log_dir(dir: &Path) -> std::io::Result<impl Iterator<Item = std::io::Result<LogDirEntry>>> {
    Ok(fs::read_dir(dir)?.map(|entry| {
        entry.map(|entry| LogDirEntry {
            file_name: entry.file_name(),
            path: entry.path(),
        })
    }))
}

/// Truncates the active file and deletes its rotated siblings.
///
/// A missing or empty `log_path` is a no-op. `list_dir` is the directory reader
/// (`read_log_dir` in production; tests inject entry errors).
fn clear_log_files<I>(
    log_path: &Path,
    list_dir: impl FnOnce(&Path) -> std::io::Result<I>,
) -> Result<(), ClearError>
where
    I: IntoIterator<Item = std::io::Result<LogDirEntry>>,
{
    if log_path.as_os_str().is_empty() || !log_path.exists() {
        return Ok(());
    }
    fs::write(log_path, "").map_err(|source| ClearError::Truncate { source })?;
    remove_rotated_logs(log_path, list_dir)
}

/// Deletes `<active file name>.<N>` siblings, the names sc-observability rotates to.
///
/// Every entry result is checked: an entry that cannot be read fails the clear
/// with [`ClearError::ReadDirEntry`] instead of being skipped.
fn remove_rotated_logs<I>(
    active: &Path,
    list_dir: impl FnOnce(&Path) -> std::io::Result<I>,
) -> Result<(), ClearError>
where
    I: IntoIterator<Item = std::io::Result<LogDirEntry>>,
{
    let (Some(dir), Some(name)) = (active.parent(), active.file_name().and_then(|n| n.to_str()))
    else {
        return Ok(());
    };
    let prefix = format!("{name}.");
    let entries = list_dir(dir).map_err(|source| ClearError::ListDir { source })?;
    for entry in entries {
        let entry = entry.map_err(|source| ClearError::ReadDirEntry { source })?;
        let Some(candidate) = entry.file_name.to_str() else {
            continue; // rotated names are always UTF-8 (`<active>.<N>`)
        };
        let is_rotated = candidate
            .strip_prefix(prefix.as_str())
            .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()));
        if is_rotated {
            fs::remove_file(&entry.path).map_err(|source| ClearError::RemoveRotated {
                file_name: candidate.to_owned(),
                source,
            })?;
        }
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn export_logs() -> Result<String, String> {
    let log_path = get_log_path();
    tauri::async_runtime::spawn_blocking(move || {
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
    })
    .await
    .map_err(|e| format!("Failed to export logs: {e}"))?
}

#[tauri::command]
pub(crate) async fn read_logs(tail_lines: Option<usize>) -> Result<String, String> {
    let log_path = get_log_path();
    tauri::async_runtime::spawn_blocking(move || {
        if log_path.as_os_str().is_empty() || !log_path.exists() {
            return Ok(String::new());
        }

        let content =
            fs::read_to_string(&log_path).map_err(|e| format!("Failed to read logs: {e}"))?;

        // If tail_lines is specified, return only the last N lines
        if let Some(n) = tail_lines {
            let lines: Vec<&str> = content.lines().collect();
            let start = lines.len().saturating_sub(n);
            Ok(lines.get(start..).unwrap_or_default().join("\n"))
        } else {
            Ok(content)
        }
    })
    .await
    .map_err(|e| format!("Failed to read logs: {e}"))?
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
    pub(super) struct TestError(pub(super) String);

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "test setup/teardown failed: {}", self.0)
        }
    }

    impl From<std::io::Error> for TestError {
        fn from(e: std::io::Error) -> Self {
            Self(e.to_string())
        }
    }

    impl From<String> for TestError {
        fn from(e: String) -> Self {
            Self(e)
        }
    }

    /// Unique scratch directory under `std::env::temp_dir()`, removed on drop.
    pub(super) struct ScratchDir(PathBuf);

    impl ScratchDir {
        pub(super) fn new(label: &str) -> Result<Self, TestError> {
            // Path-safe on every OS: `Instant`'s Debug output contains `:` and
            // braces, which Windows rejects in directory names (os error 267).
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let unique = format!(
                "btit-logging-test-{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            );
            let dir = std::env::temp_dir().join(unique);
            fs::create_dir_all(&dir)?;
            Ok(Self(dir))
        }

        pub(super) fn path(&self) -> &Path {
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
        // LOGGING is Uninstalled (install_logging never ran) and get_log_path() is
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
        let rotated_one = scratch.path().join("beads-task-issue-tracker.log.jsonl.1");
        fs::write(&rotated_one, "rotated")?;
        let rotated_two = scratch.path().join("beads-task-issue-tracker.log.jsonl.2");
        fs::write(&rotated_two, "rotated")?;
        let unrelated = scratch
            .path()
            .join("beads-task-issue-tracker.log.jsonl.bak");
        fs::write(&unrelated, "kept")?;

        remove_rotated_logs(&active, read_log_dir).map_err(|e| e.to_string())?;

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
    fn remove_rotated_logs_is_a_noop_when_the_directory_has_no_siblings() -> Result<(), TestError> {
        let scratch = ScratchDir::new("no-siblings")?;
        let active = scratch.path().join("beads-task-issue-tracker.log.jsonl");
        fs::write(&active, "active")?;

        remove_rotated_logs(&active, read_log_dir).map_err(|e| e.to_string())?;
        assert!(active.exists());
        Ok(())
    }

    /// R-A4-002: an unreadable directory entry fails the clear instead of being skipped.
    #[test]
    fn clear_log_files_reports_a_directory_entry_error() -> Result<(), TestError> {
        let scratch = ScratchDir::new("entry-error")?;
        let active = scratch.path().join("beads-task-issue-tracker.log.jsonl");
        fs::write(&active, "active")?;
        let rotated = scratch.path().join("beads-task-issue-tracker.log.jsonl.1");
        fs::write(&rotated, "rotated")?;
        let left_behind = scratch.path().join("beads-task-issue-tracker.log.jsonl.2");
        fs::write(&left_behind, "rotated")?;
        let rotated_entry = LogDirEntry {
            file_name: "beads-task-issue-tracker.log.jsonl.1".into(),
            path: rotated.clone(),
        };
        let failing_reader = move |_: &Path| -> std::io::Result<Vec<std::io::Result<LogDirEntry>>> {
            Ok(vec![
                Ok(rotated_entry),
                Err(std::io::Error::other("injected entry failure")),
            ])
        };

        let result = clear_log_files(&active, failing_reader);

        let Err(ClearError::ReadDirEntry { source }) = result else {
            return Err(TestError(format!("expected ReadDirEntry, got {result:?}")));
        };
        assert_eq!(source.to_string(), "injected entry failure");
        // The command's Err(String) carries the failure to the frontend.
        let rendered = ClearError::ReadDirEntry { source }.to_string();
        assert_eq!(
            rendered,
            "Failed to read log dir entry: injected entry failure"
        );
        assert!(
            !rotated.exists(),
            "entries before the failure are processed"
        );
        assert!(
            left_behind.exists(),
            "the clear stopped at the failing entry"
        );
        Ok(())
    }

    #[test]
    fn clear_log_files_reports_a_listing_error() -> Result<(), TestError> {
        let scratch = ScratchDir::new("list-error")?;
        let active = scratch.path().join("beads-task-issue-tracker.log.jsonl");
        fs::write(&active, "active")?;
        let result = clear_log_files(&active, |_: &Path| {
            Err::<Vec<std::io::Result<LogDirEntry>>, _>(std::io::Error::other("no listing"))
        });
        assert!(matches!(result, Err(ClearError::ListDir { .. })));
        Ok(())
    }
}
