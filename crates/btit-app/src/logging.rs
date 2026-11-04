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
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
use std::time::Duration;

use sc_observability_log::{
    init, ActionName, BridgeOptions, ErrorCode, FlushError, LevelFilter, LogGuard, LoggerConfig,
    Remediation, ServiceName,
};
use tauri::Manager;

use lifecycle::{ExitOutcome, LogLifecycle};

// Global logging flags and the gated `log_*!` macros live in btit-beads (b-3); the
// statics are re-exported so the debug commands below flip the switches every crate reads.
pub(crate) use btit_beads::logging::{LOGGING_ENABLED, VERBOSE_LOGGING};

/// Locks `mutex`, recovering the value if an earlier panic poisoned it.
///
/// Recovery is logged at error level through the plain `log` macro (not gated by
/// `LOGGING_ENABLED`), so a panic that happened while the lock was held still leaves
/// a trace in the log. The poison flag is then cleared, so the record is written once
/// per poisoning rather than on every later lock.
pub(crate) fn lock_recovering<'a, T>(
    mutex: &'a std::sync::Mutex<T>,
    name: &str,
) -> std::sync::MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::error!(
                "[lock] {name} was poisoned by an earlier panic; continuing with the recovered value"
            );
            mutex.clear_poison();
            poisoned.into_inner()
        }
    }
}

// ============================================================================
// Bridge lifecycle
// ============================================================================

/// The single owner of the process-wide `LogGuard`.
static LOGGING: LogLifecycle<LogGuard> = LogLifecycle::new();
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Bound on every blocking bridge call this module makes: one `flush` in
/// `clear_logs`, the wait for another clear's write slot, and the whole exit
/// sequence (waiting for an in-progress clear plus the final `shutdown`). Two seconds keeps quit responsive while leaving
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

/// Stable code written by [`report_exit`] for [`ExitOutcome::ShutDownWhileClearWriting`].
const EXIT_CLEAR_STILL_WRITING: &str = "BTIT_LOG_EXIT_CLEAR_STILL_WRITING";

/// Writes a failed or degraded final shutdown to stderr; a clean or no-op exit is silent.
///
/// The logger is gone by now, so stderr is the only channel (`writeln!` is used
/// because `eprintln!` panics on a closed stream).
fn report_exit(outcome: &ExitOutcome) {
    let _ = write_exit_report(&mut std::io::stderr(), outcome);
}

fn write_exit_report(out: &mut impl std::io::Write, outcome: &ExitOutcome) -> std::io::Result<()> {
    match outcome {
        ExitOutcome::ShutDown { result: Err(e) } => writeln!(
            out,
            "beads-task-issue-tracker: log shutdown failed [{}]: {e}",
            e.code()
        ),
        ExitOutcome::ShutDownWhileClearWriting { result } => {
            let shutdown = match result {
                Ok(()) => "ok".to_owned(),
                Err(e) => format!("failed [{}]: {e}", e.code()),
            };
            writeln!(
                out,
                "beads-task-issue-tracker: [{EXIT_CLEAR_STILL_WRITING}] exit budget ran out while \
                 clear_logs was still writing; the at-exit health records were skipped and the \
                 final log records may be truncated (shutdown {shutdown})"
            )
        }
        ExitOutcome::ShutDown { result: Ok(()) }
        | ExitOutcome::NotInstalled
        | ExitOutcome::AlreadyShutDown => Ok(()),
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
                LOG_IO_TIMEOUT,
                |control| control.flush(LOG_IO_TIMEOUT),
                read_log_dir,
            )
            .map_err(|e| report_clear_error(&e))?;
        log_info!("[debug] Logs cleared");
        Ok(())
    })
    .await
    .map_err(|e| {
        report_clear_error(&ClearError::TaskFailed {
            reason: e.to_string(),
        })
    })?
}

/// Logs a failed clear with its code and remediation, and returns the command's error string.
///
/// After `ShutdownStarted` the record is filtered by the stopping logger, which is fine:
/// the frontend still receives the coded string.
fn report_clear_error(error: &ClearError) -> String {
    let code = error.code();
    let remediation = serde_json::to_string(&error.remediation()).unwrap_or_default();
    log_warn!(
        code = code.as_str(), remediation = remediation.as_str();
        "[debug] Logs not cleared: {error}"
    );
    error.command_message()
}

/// Stable `ClearError` codes; the `clear_logs` command's `Err(String)` starts with one.
pub(crate) mod clear_error_codes {
    use sc_observability_log::ErrorCode;

    /// `ClearError::ShutdownStarted`.
    pub(crate) const SHUTDOWN_STARTED: ErrorCode =
        ErrorCode::new_static("BTIT_LOG_CLEAR_SHUTDOWN_STARTED");
    /// `ClearError::WriteSlotTimedOut`.
    pub(crate) const WRITE_SLOT_TIMED_OUT: ErrorCode =
        ErrorCode::new_static("BTIT_LOG_CLEAR_WRITE_SLOT_TIMED_OUT");
    /// `ClearError::Flush`.
    pub(crate) const FLUSH_FAILED: ErrorCode = ErrorCode::new_static("BTIT_LOG_CLEAR_FLUSH_FAILED");
    /// `ClearError::Truncate`.
    pub(crate) const TRUNCATE_FAILED: ErrorCode =
        ErrorCode::new_static("BTIT_LOG_CLEAR_TRUNCATE_FAILED");
    /// `ClearError::ListDir`.
    pub(crate) const LIST_DIR_FAILED: ErrorCode =
        ErrorCode::new_static("BTIT_LOG_CLEAR_LIST_DIR_FAILED");
    /// `ClearError::ReadDirEntry`.
    pub(crate) const READ_DIR_ENTRY_FAILED: ErrorCode =
        ErrorCode::new_static("BTIT_LOG_CLEAR_READ_DIR_ENTRY_FAILED");
    /// `ClearError::RemoveRotated`.
    pub(crate) const REMOVE_ROTATED_FAILED: ErrorCode =
        ErrorCode::new_static("BTIT_LOG_CLEAR_REMOVE_ROTATED_FAILED");
    /// `ClearError::TaskFailed`.
    pub(crate) const TASK_FAILED: ErrorCode = ErrorCode::new_static("BTIT_LOG_CLEAR_TASK_FAILED");
}

/// Why `clear_logs` failed.
///
/// Each variant has a stable [`ClearError::code`] and a [`ClearError::remediation`],
/// mirroring the `sc-observability-log` error enums. The command returns
/// [`ClearError::command_message`], `"<CODE>: <message>"`, so the frontend can
/// match on the code prefix without parsing the message.
#[derive(Debug)]
pub(crate) enum ClearError {
    /// Exit has begun: nothing was flushed or removed, so the final records survive.
    ShutdownStarted,
    /// Another clear still held the write slot after the timeout; no file was touched.
    WriteSlotTimedOut { timeout: Duration },
    /// The bounded flush before clearing failed; no file was touched.
    ///
    /// All flush causes share `BTIT_LOG_CLEAR_FLUSH_FAILED`; the message carries the
    /// source's own code (for example `SC_OBSERVABILITY_LOG_FLUSH_IN_PROGRESS` when an
    /// earlier clear's flush is still stuck on its detached helper).
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
    /// The blocking clear task could not run to completion (it panicked or was cancelled).
    TaskFailed { reason: String },
}

impl ClearError {
    /// Stable code per variant (`BTIT_LOG_CLEAR_*`).
    pub(crate) fn code(&self) -> ErrorCode {
        match self {
            Self::ShutdownStarted => clear_error_codes::SHUTDOWN_STARTED,
            Self::WriteSlotTimedOut { .. } => clear_error_codes::WRITE_SLOT_TIMED_OUT,
            Self::Flush { .. } => clear_error_codes::FLUSH_FAILED,
            Self::Truncate { .. } => clear_error_codes::TRUNCATE_FAILED,
            Self::ListDir { .. } => clear_error_codes::LIST_DIR_FAILED,
            Self::ReadDirEntry { .. } => clear_error_codes::READ_DIR_ENTRY_FAILED,
            Self::RemoveRotated { .. } => clear_error_codes::REMOVE_ROTATED_FAILED,
            Self::TaskFailed { .. } => clear_error_codes::TASK_FAILED,
        }
    }

    /// Mandatory remediation per variant.
    pub(crate) fn remediation(&self) -> Remediation {
        match self {
            Self::ShutdownStarted => Remediation::not_recoverable(
                "the app is quitting; the logs are kept so the final records survive",
            ),
            Self::WriteSlotTimedOut { .. } => Remediation::recoverable(
                "wait for the other clear to finish, then clear again",
                ["check that the log directory's disk is responsive"],
            ),
            Self::Flush {
                source: FlushError::InProgress,
            } => Remediation::recoverable(
                "wait for the previous flush to finish, then retry the clear; no file was touched",
                ["check that the log directory's disk is responsive"],
            ),
            Self::Flush { source } => Remediation::recoverable(
                "retry the clear; no file was touched",
                [format!(
                    "inspect the logger health: flush failed with {}",
                    source.code()
                )],
            ),
            Self::Truncate { .. } => Remediation::recoverable(
                "check that the active log file is writable and not locked by another process",
                ["retry the clear"],
            ),
            Self::ListDir { .. } | Self::ReadDirEntry { .. } => Remediation::recoverable(
                "check the permissions of the log directory",
                ["retry the clear; rotated log files may remain until it succeeds"],
            ),
            Self::RemoveRotated { .. } => Remediation::recoverable(
                "check that the rotated log file is not locked and its directory is writable",
                ["retry the clear"],
            ),
            Self::TaskFailed { .. } => Remediation::recoverable(
                "retry the clear",
                ["report the failure with the app log if it repeats"],
            ),
        }
    }

    /// The command's error string: `"<CODE>: <message>"`.
    pub(crate) fn command_message(&self) -> String {
        format!("{}: {self}", self.code())
    }
}

impl std::fmt::Display for ClearError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ShutdownStarted => f.write_str("Logs were not cleared: logging is shutting down"),
            Self::WriteSlotTimedOut { timeout } => write!(
                f,
                "Logs were not cleared: another clear was still writing after {timeout:?}"
            ),
            Self::Flush { source } => {
                write!(f, "Failed to flush logs [{}]: {source}", source.code())
            }
            Self::Truncate { source } => write!(f, "Failed to clear logs: {source}"),
            Self::ListDir { source } => write!(f, "Failed to list log dir: {source}"),
            Self::ReadDirEntry { source } => write!(f, "Failed to read log dir entry: {source}"),
            Self::RemoveRotated { file_name, source } => {
                write!(f, "Failed to remove rotated log {file_name}: {source}")
            }
            Self::TaskFailed { reason } => write!(f, "Failed to clear logs: {reason}"),
        }
    }
}

impl std::error::Error for ClearError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ShutdownStarted | Self::WriteSlotTimedOut { .. } | Self::TaskFailed { .. } => {
                None
            }
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

const FRONTEND_LOG_INPUT_MAX_BYTES: usize = 4 * 1024;
const FRONTEND_LOG_TRUNCATION_MARKER: &str = "… [truncated]";

fn bounded_frontend_log_input(value: &str) -> String {
    if value.len() <= FRONTEND_LOG_INPUT_MAX_BYTES {
        return value.to_owned();
    }
    let prefix_limit = FRONTEND_LOG_INPUT_MAX_BYTES - FRONTEND_LOG_TRUNCATION_MARKER.len();
    let mut end = prefix_limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{FRONTEND_LOG_TRUNCATION_MARKER}", &value[..end])
}

#[tauri::command]
pub(crate) async fn log_frontend(level: String, message: String) {
    let level = bounded_frontend_log_input(&level);
    let message = bounded_frontend_log_input(&message);
    match level.as_str() {
        "error" => log::log!(target: "frontend", log::Level::Error, "{message}"),
        "warn" => log::log!(target: "frontend", log::Level::Warn, "{message}"),
        "info" => log::log!(target: "frontend", log::Level::Info, "{message}"),
        // Unknown level (should not happen: the TypeScript wrapper restricts the
        // level to error/warn/info): log at Info, same as today, but keep the raw
        // value visible in JSONL as a kv field.
        other => {
            log::log!(target: "frontend", log::Level::Info, frontend_level = other; "{message}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontend_log_input_is_utf8_safe_and_marked_when_truncated() {
        let short = "é".repeat(FRONTEND_LOG_INPUT_MAX_BYTES / 2);
        assert_eq!(bounded_frontend_log_input(&short), short);

        let long = "é".repeat(FRONTEND_LOG_INPUT_MAX_BYTES);
        let bounded = bounded_frontend_log_input(&long);
        assert!(bounded.ends_with(FRONTEND_LOG_TRUNCATION_MARKER));
        let prefix_end = bounded.len() - FRONTEND_LOG_TRUNCATION_MARKER.len();
        assert!(bounded.is_char_boundary(prefix_end));
        assert!(bounded.len() <= FRONTEND_LOG_INPUT_MAX_BYTES);
    }

    /// A poisoned lock yields its value, and the poison flag is cleared so the
    /// recovery is logged once (QA-1 RSH-001/RSH-002, b-12).
    #[test]
    fn lock_recovering_returns_value_and_clears_poison() -> Result<(), TestError> {
        let mutex = std::sync::Arc::new(std::sync::Mutex::new(7_u32));
        let held = std::sync::Arc::clone(&mutex);
        let joined = std::thread::spawn(move || {
            let _guard = held.lock();
            // `resume_unwind` poisons the held lock without running the panic hook.
            std::panic::resume_unwind(Box::new("poison the lock"));
        })
        .join();
        if joined.is_ok() || !mutex.is_poisoned() {
            return Err(TestError(
                "the helper thread did not poison the lock".to_owned(),
            ));
        }
        let value = *lock_recovering(&mutex, "TEST_LOCK");
        if value != 7 || mutex.is_poisoned() {
            return Err(TestError(format!(
                "value {value}, still poisoned: {}",
                mutex.is_poisoned()
            )));
        }
        Ok(())
    }

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

    fn every_clear_error() -> Vec<ClearError> {
        vec![
            ClearError::ShutdownStarted,
            ClearError::WriteSlotTimedOut {
                timeout: LOG_IO_TIMEOUT,
            },
            ClearError::Flush {
                source: FlushError::NotRunning {
                    phase: sc_observability_log::LifecyclePhase::Stopped,
                },
            },
            ClearError::Truncate {
                source: std::io::Error::other("truncate"),
            },
            ClearError::ListDir {
                source: std::io::Error::other("list"),
            },
            ClearError::ReadDirEntry {
                source: std::io::Error::other("entry"),
            },
            ClearError::RemoveRotated {
                file_name: "app.log.jsonl.1".to_owned(),
                source: std::io::Error::other("remove"),
            },
            ClearError::TaskFailed {
                reason: "cancelled".to_owned(),
            },
        ]
    }

    /// RBP-F001: every variant has a unique stable code and a non-empty remediation.
    #[test]
    fn clear_error_codes_are_stable_and_remediations_non_empty() {
        let errors = every_clear_error();
        let mut codes = std::collections::HashSet::new();
        for error in &errors {
            let code = error.code();
            assert!(code.as_str().starts_with("BTIT_LOG_CLEAR_"), "{code}");
            assert!(codes.insert(code.as_str().to_owned()), "duplicate {code}");
            match error.remediation() {
                Remediation::Recoverable { steps } => {
                    assert!(!steps.steps().is_empty());
                    assert!(steps.steps().iter().all(|step| !step.is_empty()));
                }
                Remediation::NotRecoverable { justification } => {
                    assert!(!justification.is_empty());
                }
            }
            let message = error.command_message();
            assert!(
                message.starts_with(&format!("{code}: ")),
                "the command string leads with the code: {message}"
            );
        }
        assert_eq!(codes.len(), errors.len());
        assert_eq!(
            ClearError::ShutdownStarted.command_message(),
            "BTIT_LOG_CLEAR_SHUTDOWN_STARTED: Logs were not cleared: logging is shutting down"
        );
    }

    /// QA-2 RSH-004: a clear retried behind a stuck flush keeps the clear code and names the cause.
    #[test]
    fn clear_behind_a_flush_in_progress_reports_the_source_code() -> Result<(), TestError> {
        let error = ClearError::Flush {
            source: FlushError::InProgress,
        };
        assert_eq!(error.code(), clear_error_codes::FLUSH_FAILED);
        assert_eq!(
            error.command_message(),
            "BTIT_LOG_CLEAR_FLUSH_FAILED: Failed to flush logs \
             [SC_OBSERVABILITY_LOG_FLUSH_IN_PROGRESS]: a previous flush is still running; \
             no new flush was started"
        );
        let Remediation::Recoverable { steps } = error.remediation() else {
            return Err(TestError("expected a recoverable remediation".to_owned()));
        };
        assert!(steps
            .steps()
            .first()
            .is_some_and(|step| step.contains("wait for the previous flush")));
        Ok(())
    }

    /// RSH-001: the degraded exit is reported with its stable code.
    #[test]
    fn exit_report_names_the_degraded_clear_case() -> Result<(), TestError> {
        let mut out = Vec::new();
        write_exit_report(
            &mut out,
            &ExitOutcome::ShutDownWhileClearWriting { result: Ok(()) },
        )?;
        let text = String::from_utf8(out).map_err(|e| TestError(e.to_string()))?;
        assert!(
            text.contains("[BTIT_LOG_EXIT_CLEAR_STILL_WRITING]"),
            "{text}"
        );
        assert!(text.contains("(shutdown ok)"), "{text}");

        let mut silent = Vec::new();
        write_exit_report(&mut silent, &ExitOutcome::ShutDown { result: Ok(()) })?;
        write_exit_report(&mut silent, &ExitOutcome::AlreadyShutDown)?;
        assert!(silent.is_empty());
        Ok(())
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
        // The command's Err(String) carries the stable code and the failure to the frontend.
        let rendered = ClearError::ReadDirEntry { source }.command_message();
        assert_eq!(
            rendered,
            "BTIT_LOG_CLEAR_READ_DIR_ENTRY_FAILED: Failed to read log dir entry: injected entry failure"
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
