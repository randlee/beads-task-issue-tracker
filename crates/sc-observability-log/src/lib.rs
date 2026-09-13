//! `log` facade bridge for sc-observability structured JSONL logging.
//!
//! [`init`] installs a `log::Log` implementation that maps every `log` record to
//! a `sc_observability_types::LogEvent` and writes it through one process-wide
//! `sc_observability::Logger`. Existing `log::info!` (and friends) call sites keep
//! working unchanged.
//!
//! # Quick start
//!
//! ```no_run
//! use std::time::Duration;
//! use sc_observability_log::{ActionName, BridgeOptions, LoggerConfig, ServiceName};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = LoggerConfig::default_for(ServiceName::new("my-app")?, "/tmp/my-app".into());
//! let options = BridgeOptions {
//!     default_action: ActionName::new("log.record")?,
//!     parse_bracket_action: true,
//! };
//! let guard = sc_observability_log::init(config, options)?;
//! log::info!(target: "my_app::sync", "[sync.start] syncing {} items", 3);
//! guard.flush(Duration::from_secs(1))?;
//! guard.shutdown(Duration::from_secs(5))?;
//! # Ok(())
//! # }
//! ```
//!
//! # Guarantees
//!
//! - The emit path never blocks on I/O or queue capacity and never panics; every
//!   dropped event is counted under one [`DropCause`].
//! - `log::logger().flush()` does nothing. Call [`LogGuard::flush`] for a flush
//!   bounded by a timeout.
//! - [`LogGuard::shutdown`] and `Drop for LogGuard` are bounded by a timeout.

pub mod error_codes;

mod bridge;
mod error;
mod handle;
mod mapping;

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, PoisonError};
use std::time::Duration;

#[doc(inline)]
pub use error::{DropCause, FlushError, InitError, ShutdownError};
#[doc(inline)]
pub use sc_observability::LoggerConfig;
// Re-exported so consumers need no direct sc-observability-types dependency.
#[doc(inline)]
pub use sc_observability_types::{
    ActionName, ErrorCode, LevelFilter, ProcessIdentityPolicy, Remediation, ServiceName,
    TargetCategory,
};
// `Level` is intentionally NOT re-exported: a-2 defines a tracing-style
// `sc_observability_log::Level` (associated consts TRACE..ERROR).

use handle::{INSTALLED, Installed, SLOT, THRESHOLD};

/// Bridge behavior. The level threshold is `LoggerConfig.level` only.
#[derive(Debug, Clone)]
pub struct BridgeOptions {
    /// Action used when a record carries no leading `[tag]`.
    pub default_action: ActionName,
    /// Strip a leading `[tag] ` from the message into `LogEvent.action`.
    pub parse_bracket_action: bool,
}

/// Snapshot of dropped-event counters, keyed by [`DropCause`].
///
/// The derives are pinned by `tests/api_freeze.rs`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DroppedEvents {
    queue_full: u64,
    invalid_event: u64,
    writer_degraded: u64,
    shutdown_timed_out: u64,
    not_installed: u64,
    logger_panicked: u64,
    reentrant_emit: u64,
}

impl DroppedEvents {
    pub(crate) fn from_counter(read: impl Fn(DropCause) -> u64) -> Self {
        Self {
            queue_full: read(DropCause::QueueFull),
            invalid_event: read(DropCause::InvalidEvent),
            writer_degraded: read(DropCause::WriterDegraded),
            shutdown_timed_out: read(DropCause::ShutdownTimedOut),
            not_installed: read(DropCause::NotInstalled),
            logger_panicked: read(DropCause::LoggerPanicked),
            reentrant_emit: read(DropCause::ReentrantEmit),
        }
    }

    /// Number of events dropped for `cause`.
    #[must_use]
    pub fn get(&self, cause: DropCause) -> u64 {
        match cause {
            DropCause::QueueFull => self.queue_full,
            DropCause::InvalidEvent => self.invalid_event,
            DropCause::WriterDegraded => self.writer_degraded,
            DropCause::ShutdownTimedOut => self.shutdown_timed_out,
            DropCause::NotInstalled => self.not_installed,
            DropCause::LoggerPanicked => self.logger_panicked,
            DropCause::ReentrantEmit => self.reentrant_emit,
        }
    }

    /// Saturating sum over [`DropCause::ALL`].
    #[must_use]
    pub fn total(&self) -> u64 {
        DropCause::ALL
            .iter()
            .fold(0_u64, |sum, cause| sum.saturating_add(self.get(*cause)))
    }
}

/// Timeout used by `Drop for LogGuard` when `shutdown` was not called.
pub const DEFAULT_DROP_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// Keeps the bridge installed; dropping it shuts the logger down.
#[must_use = "dropping the guard shuts the logger down"]
#[derive(Debug)]
pub struct LogGuard {
    active_log_path: Option<PathBuf>,
    shut_down: bool,
}

impl LogGuard {
    /// Flushes on a helper thread, bounded by `timeout`.
    ///
    /// # Errors
    ///
    /// Returns [`FlushError::TimedOut`] when the writer does not acknowledge within
    /// `timeout` (the helper is detached), [`FlushError::Logger`] when a sink flush
    /// fails, and [`FlushError::HelperSpawn`] / [`FlushError::HelperLost`] when the
    /// helper thread cannot start or ends without a result.
    pub fn flush(&self, timeout: Duration) -> Result<(), FlushError> {
        handle::flush_installed(timeout)
    }

    /// Threshold → Off, empty the slot, flush, `Logger::shutdown`; bounded by `timeout`.
    ///
    /// # Errors
    ///
    /// Returns [`ShutdownError::TimedOut`] when sole ownership, the final flush and
    /// the writer join do not finish within `timeout`, [`ShutdownError::FinalFlush`]
    /// when the final flush fails (the logger is still shut down), and
    /// [`ShutdownError::HelperSpawn`] / [`ShutdownError::HelperLost`] for helper
    /// thread failures.
    pub fn shutdown(mut self, timeout: Duration) -> Result<(), ShutdownError> {
        self.shut_down = true;
        handle::shutdown_sequence(timeout)
    }

    /// Snapshot of the process-wide dropped-event counters.
    #[must_use]
    pub fn dropped_events(&self) -> DroppedEvents {
        handle::dropped_events()
    }

    /// `Logger::health().active_log_path` captured at init; `None` when
    /// `LoggerConfig.enable_file_sink` is false.
    #[must_use]
    pub fn active_log_path(&self) -> Option<&Path> {
        self.active_log_path.as_deref()
    }
}

impl Drop for LogGuard {
    fn drop(&mut self) {
        if !self.shut_down {
            self.shut_down = true;
            let _ = handle::shutdown_sequence(DEFAULT_DROP_SHUTDOWN_TIMEOUT);
        }
    }
}

/// Installs the bridge as the process-wide `log` logger.
///
/// Resolves `config.process_identity` once, builds the `sc_observability::Logger`,
/// installs the bridge with `log::set_boxed_logger`, and derives the `log` facade
/// level and the emit threshold from `config.level`.
///
/// # Errors
///
/// - [`InitError::AlreadyInitialized`] when `init` already succeeded, is running
///   concurrently, or earlier returned `ForeignLoggerInstalled`.
/// - [`InitError::ForeignLoggerInstalled`] when another `log::Log` is installed.
/// - [`InitError::IdentityResolution`] when `ProcessIdentityPolicy::Resolver` fails
///   (retry allowed).
/// - [`InitError::Logger`] when `Logger::new` fails (retry allowed).
pub fn init(config: LoggerConfig, options: BridgeOptions) -> Result<LogGuard, InitError> {
    if INSTALLED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        // Covers a live guard, a guard already shut down (the facade logger cannot be
        // uninstalled), and an own init running concurrently on another thread.
        return Err(InitError::AlreadyInitialized);
    }
    let identity = match mapping::resolve_identity(&config.process_identity) {
        Ok(identity) => identity,
        Err(source) => {
            INSTALLED.store(false, Ordering::SeqCst); // recoverable: allow a retry
            return Err(InitError::IdentityResolution { source });
        }
    };
    let (level, service, enable_file_sink) = (
        config.level,
        config.service_name.clone(),
        config.enable_file_sink,
    );
    let logger = match sc_observability::Logger::new(config) {
        Ok(logger) => logger,
        Err(source) => {
            INSTALLED.store(false, Ordering::SeqCst); // recoverable: allow a retry
            return Err(InitError::Logger { source });
        }
    };
    let active_log_path = enable_file_sink.then(|| logger.health().active_log_path);
    let installed = Arc::new(Installed {
        logger,
        service,
        identity,
        options,
    });
    if let Err(source) = log::set_boxed_logger(Box::new(bridge::Bridge)) {
        // INSTALLED stays set: the facade slot belongs to the other logger for the
        // rest of the process, so no retry can succeed. Later calls return
        // AlreadyInitialized without building and tearing down another Logger.
        let _ = handle::shutdown_installed(installed, DEFAULT_DROP_SHUTDOWN_TIMEOUT);
        return Err(InitError::ForeignLoggerInstalled { source });
    }
    // Install order: slot, then threshold, then the facade level. Records arriving
    // before the last step are filtered by the facade's initial `Off` level.
    *SLOT.write().unwrap_or_else(PoisonError::into_inner) = Some(installed);
    THRESHOLD.store(handle::encode_threshold(level), Ordering::SeqCst);
    log::set_max_level(handle::to_log_level_filter(level));
    Ok(LogGuard {
        active_log_path,
        shut_down: false,
    })
}

/// Hidden support for `sc-observability-log-macros` expansions.
///
/// Outside semver: `sc-observability-log` pins the macros crate with an exact
/// `=` version so expansions and this module always move in lockstep.
#[doc(hidden)]
pub mod __private {
    use std::sync::PoisonError;

    use sc_observability::TryLogError;

    use crate::{DropCause, handle, mapping};

    pub use serde_json::{Map, Value};

    /// The single label sanitizer (`mapping.rs`).
    pub use crate::mapping::{
        LabelError, LabelKind, RESERVED_FIELD_PREFIX, action_label, field_key_label,
        sanitize_label, target_label,
    };

    /// Everything a call site controls; `emit` fills version, timestamp, service, identity and trace.
    #[derive(Debug)]
    pub struct EventParts {
        /// Event severity.
        pub level: sc_observability_types::Level,
        /// Sanitized target category.
        pub target: sc_observability_types::TargetCategory,
        /// Action; `None` uses `BridgeOptions.default_action`.
        pub action: Option<sc_observability_types::ActionName>,
        /// Formatted message.
        pub message: Option<String>,
        /// Optional outcome label.
        pub outcome: Option<sc_observability_types::OutcomeLabel>,
        /// Structured fields.
        pub fields: Map<String, Value>,
    }

    /// Lock-free check of `level` against the threshold derived from `LoggerConfig.level`.
    #[must_use]
    pub fn enabled(level: sc_observability_types::Level) -> bool {
        handle::level_enabled(level)
    }

    /// Submits one event to the installed `Logger` with `try_log`.
    ///
    /// Never blocks on I/O or queue capacity and never panics: the slot read, event
    /// assembly and `try_log` run inside a `catch_unwind` guard. It is not
    /// reentrant: a call on a thread already inside `emit` (a panic hook, sink or
    /// redactor that logs) returns at once and is counted as
    /// `DropCause::ReentrantEmit`. Every dropped event is counted under exactly one
    /// [`DropCause`]. Nothing is flushed; use `LogGuard::flush(timeout)`.
    pub fn emit(parts: EventParts) {
        handle::submit_guarded(|| {
            let Some(installed) = handle::SLOT
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
            else {
                return Err(DropCause::NotInstalled);
            };
            let event = mapping::assemble_event(
                parts,
                &installed.service,
                &installed.identity,
                &installed.options.default_action,
            );
            installed
                .logger
                .try_log(event)
                .map_err(|error| match error {
                    TryLogError::QueueFull(_) => DropCause::QueueFull,
                    TryLogError::InvalidEvent(_) => DropCause::InvalidEvent,
                    TryLogError::WriterDegraded(_) => DropCause::WriterDegraded,
                    TryLogError::ShutdownTimedOut(_) => DropCause::ShutdownTimedOut,
                })
        });
    }

    /// Counts one dropped event; a-2/a-3 call it for `DropCause::InvalidEvent` label failures.
    ///
    /// A wrapper, not `pub use`: re-exporting the `pub(crate)` fn is E0364.
    pub fn record_drop(cause: DropCause) {
        handle::record_drop(cause);
    }
}
