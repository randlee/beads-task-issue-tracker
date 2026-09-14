//! Single lifecycle owner for the process-wide `LogGuard` (review finding R-A4-001).
//!
//! [`LogLifecycle`] is the only place the non-cloneable guard lives. It
//! serializes the three operations that touch the logger's lifetime:
//!
//! - **clear** ([`LogLifecycle::clear`]) never holds the guard. It takes a
//!   non-owning handle (`LogGuard::handle`) under the lock, releases the lock,
//!   flushes through the handle, then re-acquires the lock to claim the single
//!   *clear-write slot* before truncating files. Once shutdown has begun it is
//!   rejected with [`ClearError::ShutdownStarted`] and touches no file.
//! - **exit** ([`LogLifecycle::exit`]) moves the guard out and marks the phase
//!   `ShuttingDown` in one critical section, so later clears are rejected. It
//!   waits (bounded by the exit deadline, lock released while waiting) only for
//!   a clear that already holds the write slot, so the final records are never
//!   truncated away. A flush still in progress is not awaited here: the bridge's
//!   shutdown waits for it inside the same deadline.
//! - **final shutdown** runs exactly once, on the exit thread, bounded by what
//!   is left of the exit timeout, and its `Result` is returned to the caller in
//!   [`ExitOutcome::ShutDown`]. A second exit reports
//!   [`ExitOutcome::AlreadyShutDown`] without another shutdown request.
//!
//! No code path holds the lock across a flush, a shutdown or file I/O, and exit
//! as a whole is bounded by its timeout even while a clear is flushing or
//! truncating. `Drop for LogGuard` is only a fallback for a guard that never
//! reached `exit` (the static owner itself is never dropped).

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use sc_observability_log::{DropCause, FlushError, LogGuard, LogHandle, ShutdownError};

use super::{clear_log_files, ClearError, LogDirEntry};

/// Guard operations the lifecycle owner needs: `LogGuard` in production, a fake in tests.
pub(crate) trait OwnedGuard: Send {
    /// Non-owning flush capability handed to a clear.
    type Handle;

    /// Returns a handle that neither keeps the logger alive nor can shut it down.
    fn handle(&self) -> Self::Handle;

    /// Records final diagnostics while the logger still accepts records.
    fn report_before_shutdown(&self);

    /// Final, bounded shutdown; called at most once per lifecycle.
    ///
    /// # Errors
    ///
    /// Returns the bridge's [`ShutdownError`] unchanged.
    fn shutdown(self, timeout: Duration) -> Result<(), ShutdownError>;
}

impl OwnedGuard for LogGuard {
    type Handle = LogHandle;

    fn handle(&self) -> LogHandle {
        LogGuard::handle(self)
    }

    /// Logs one `warn` with per-cause drop counts (when any) and one `info` with the health snapshot.
    fn report_before_shutdown(&self) {
        let health = self.health();
        let dropped = health.dropped_events;
        if dropped.total() > 0 {
            let summary: Vec<String> = DropCause::ALL
                .iter()
                .filter(|cause| dropped.get(**cause) > 0)
                .map(|cause| format!("{cause:?}={}", dropped.get(*cause)))
                .collect();
            log::warn!("[logging] dropped events: {}", summary.join(", "));
        }
        match serde_json::to_string(&health) {
            Ok(json) => log::info!(health = json.as_str(); "[logging] health at exit"),
            Err(e) => log::warn!("[logging] health at exit could not be serialized: {e}"),
        }
    }

    fn shutdown(self, timeout: Duration) -> Result<(), ShutdownError> {
        LogGuard::shutdown(self, timeout)
    }
}

/// Lifecycle phase; `Running` is the only phase that owns a guard.
enum Phase<G> {
    Uninstalled,
    Running(G),
    ShuttingDown,
    Stopped,
}

struct State<G> {
    phase: Phase<G>,
    /// A clear holds the single write slot (it is truncating or deleting files).
    clear_writing: bool,
}

/// The single owner of the process-wide log guard.
pub(crate) struct LogLifecycle<G> {
    state: Mutex<State<G>>,
    /// Signalled whenever `clear_writing` is released or the phase becomes `Stopped`.
    changed: Condvar,
    /// Number of final shutdowns requested; at most one per lifecycle.
    shutdown_requests: AtomicUsize,
}

/// Result of [`LogLifecycle::exit`].
#[derive(Debug)]
pub(crate) enum ExitOutcome {
    /// No guard was ever installed; nothing to shut down.
    NotInstalled,
    /// An earlier exit already owns (or finished) the final shutdown.
    AlreadyShutDown,
    /// This call performed the one final shutdown.
    ShutDown {
        /// The bridge's shutdown result.
        result: Result<(), ShutdownError>,
    },
}

/// Why [`LogLifecycle::install`] rejected a guard.
#[derive(Debug)]
pub(crate) enum InstallError {
    /// A guard is already installed or shutdown has begun; the new guard was shut down.
    NotUninstalled {
        /// Result of shutting down the rejected guard.
        rejected_shutdown: Result<(), ShutdownError>,
    },
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotUninstalled { rejected_shutdown } => write!(
                f,
                "log guard rejected: logging is already installed or shut down \
                 (rejected guard shutdown: {rejected_shutdown:?})"
            ),
        }
    }
}

impl std::error::Error for InstallError {}

/// Releases the clear-write slot on every exit path, including unwinding.
struct ClearWrite<'a, G> {
    lifecycle: &'a LogLifecycle<G>,
}

impl<G> Drop for ClearWrite<'_, G> {
    fn drop(&mut self) {
        self.lifecycle.lock().clear_writing = false;
        self.lifecycle.changed.notify_all();
    }
}

/// Time left until `deadline`; the full `timeout` when the deadline overflowed.
fn remaining(deadline: Option<Instant>, timeout: Duration) -> Duration {
    deadline.map_or(timeout, |d| d.saturating_duration_since(Instant::now()))
}

impl<G> LogLifecycle<G> {
    /// An empty lifecycle (`Uninstalled`).
    pub(crate) const fn new() -> Self {
        Self {
            state: Mutex::new(State {
                phase: Phase::Uninstalled,
                clear_writing: false,
            }),
            changed: Condvar::new(),
            shutdown_requests: AtomicUsize::new(0),
        }
    }

    /// Locks the state, recovering from poisoning: every transition is a single assignment.
    fn lock(&self) -> MutexGuard<'_, State<G>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[cfg(test)]
    pub(crate) fn shutdown_requests(&self) -> usize {
        self.shutdown_requests.load(Ordering::SeqCst)
    }
}

impl<G: OwnedGuard> LogLifecycle<G> {
    /// Takes ownership of `guard`; only valid once, before any exit.
    ///
    /// # Errors
    ///
    /// [`InstallError::NotUninstalled`] when a guard is already installed or
    /// shutdown has begun; the rejected guard is shut down (bounded by `timeout`)
    /// and that result is returned in the error.
    pub(crate) fn install(&self, guard: G, timeout: Duration) -> Result<(), InstallError> {
        let rejected = {
            let mut state = self.lock();
            if matches!(state.phase, Phase::Uninstalled) {
                state.phase = Phase::Running(guard);
                return Ok(());
            }
            guard
        };
        Err(InstallError::NotUninstalled {
            rejected_shutdown: rejected.shutdown(timeout),
        })
    }

    /// Flushes through a handle, then truncates the log files; serialized with exit.
    ///
    /// `flush` receives the non-owning handle (production:
    /// `|h| h.flush(LOG_IO_TIMEOUT)`); it is not called before installation.
    /// `list_dir` is the directory reader passed to [`clear_log_files`].
    ///
    /// # Errors
    ///
    /// - [`ClearError::ShutdownStarted`] when shutdown began before the flush or
    ///   before the write slot was claimed; no file is touched.
    /// - [`ClearError::Flush`] when the flush failed (no file is touched).
    /// - Every file error of [`clear_log_files`].
    pub(crate) fn clear<I>(
        &self,
        log_path: &Path,
        flush: impl FnOnce(&G::Handle) -> Result<(), FlushError>,
        list_dir: impl FnOnce(&Path) -> std::io::Result<I>,
    ) -> Result<(), ClearError>
    where
        I: IntoIterator<Item = std::io::Result<LogDirEntry>>,
    {
        let handle = {
            let state = self.lock();
            match &state.phase {
                Phase::Uninstalled => None,
                Phase::Running(guard) => Some(guard.handle()),
                Phase::ShuttingDown | Phase::Stopped => return Err(ClearError::ShutdownStarted),
            }
        };
        let flushed = handle.as_ref().map_or(Ok(()), flush);
        let _write = self.claim_clear_write()?;
        flushed.map_err(|source| ClearError::Flush { source })?;
        clear_log_files(log_path, list_dir)
    }

    /// Waits for any other clear's write slot, then claims it unless shutdown has begun.
    fn claim_clear_write(&self) -> Result<ClearWrite<'_, G>, ClearError> {
        let mut state = self.lock();
        loop {
            if matches!(state.phase, Phase::ShuttingDown | Phase::Stopped) {
                return Err(ClearError::ShutdownStarted);
            }
            if !state.clear_writing {
                break;
            }
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
        state.clear_writing = true;
        Ok(ClearWrite { lifecycle: self })
    }

    /// Performs the one final shutdown, bounded by `timeout` in total.
    pub(crate) fn exit(&self, timeout: Duration) -> ExitOutcome {
        let deadline = Instant::now().checked_add(timeout);
        let guard = {
            let mut state = self.lock();
            let guard = match std::mem::replace(&mut state.phase, Phase::ShuttingDown) {
                Phase::Running(guard) => guard,
                Phase::Uninstalled => {
                    state.phase = Phase::Stopped;
                    return ExitOutcome::NotInstalled;
                }
                previous @ (Phase::ShuttingDown | Phase::Stopped) => {
                    state.phase = previous;
                    return ExitOutcome::AlreadyShutDown;
                }
            };
            // Let a clear that is already truncating finish, so the final
            // records written below survive; never past the exit deadline.
            while state.clear_writing {
                let left = remaining(deadline, timeout);
                if left.is_zero() {
                    break;
                }
                state = self
                    .changed
                    .wait_timeout(state, left)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
            guard
        };
        guard.report_before_shutdown();
        self.shutdown_requests.fetch_add(1, Ordering::SeqCst);
        let result = guard.shutdown(remaining(deadline, timeout));
        self.lock().phase = Phase::Stopped;
        self.changed.notify_all();
        ExitOutcome::ShutDown { result }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc;
    use std::sync::{Arc, Barrier};

    use sc_observability_log::{
        ActionName, BridgeHealthState, BridgeLifecycle, BridgeOptions, LevelFilter, LoggerConfig,
        ServiceName,
    };

    use super::*;
    use crate::logging::read_log_dir;
    use crate::logging::tests::{ScratchDir, TestError};

    /// Bound used by these tests, equal to production's `LOG_IO_TIMEOUT`.
    const TIMEOUT: Duration = crate::logging::LOG_IO_TIMEOUT;

    /// Records every call the lifecycle makes on a guard.
    #[derive(Default)]
    struct Calls {
        shutdown_timeouts: Mutex<Vec<Duration>>,
        reports: AtomicUsize,
    }

    struct FakeGuard(Arc<Calls>);

    impl OwnedGuard for FakeGuard {
        type Handle = ();

        fn handle(&self) {}

        fn report_before_shutdown(&self) {
            self.0.reports.fetch_add(1, Ordering::SeqCst);
        }

        fn shutdown(self, timeout: Duration) -> Result<(), ShutdownError> {
            self.0
                .shutdown_timeouts
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(timeout);
            Ok(())
        }
    }

    fn installed_fake() -> Result<(LogLifecycle<FakeGuard>, Arc<Calls>), TestError> {
        let calls = Arc::new(Calls::default());
        let lifecycle = LogLifecycle::new();
        lifecycle
            .install(FakeGuard(Arc::clone(&calls)), TIMEOUT)
            .map_err(|e| TestError(e.to_string()))?;
        Ok((lifecycle, calls))
    }

    fn shutdown_timeouts(calls: &Calls) -> Vec<Duration> {
        calls
            .shutdown_timeouts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// R-A4-001 required evidence, against the real bridge (the only `init` in
    /// this test binary): a clear whose flush is held while exit is delivered.
    #[test]
    fn exit_during_a_held_clear_flush_shuts_down_once_and_stops_the_logger() -> Result<(), TestError>
    {
        let scratch = ScratchDir::new("lifecycle-real")?;
        let mut config = LoggerConfig::default_for(
            ServiceName::new("lifecycle-test").map_err(|e| TestError(e.to_string()))?,
            scratch.path().to_path_buf(),
        );
        config.level = LevelFilter::Info;
        config.enable_console_sink = false;
        let options = BridgeOptions {
            default_action: ActionName::new("log").map_err(|e| TestError(e.to_string()))?,
            parse_bracket_action: true,
        };
        let guard =
            sc_observability_log::init(config, options).map_err(|e| TestError(e.to_string()))?;
        let log_path = guard
            .active_log_path()
            .ok_or_else(|| TestError("file sink disabled".to_owned()))?
            .to_path_buf();
        let late_handle = guard.handle();
        let lifecycle = LogLifecycle::new();
        lifecycle
            .install(guard, TIMEOUT)
            .map_err(|e| TestError(e.to_string()))?;

        log::info!("[lifecycle] record before exit");

        let (held_tx, held_rx) = mpsc::channel::<()>();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let (clear_result, exit_elapsed, outcome) = std::thread::scope(|scope| {
            let (lifecycle, log_path) = (&lifecycle, &log_path);
            let clearing = scope.spawn(move || {
                lifecycle.clear(
                    log_path,
                    move |handle: &LogHandle| {
                        // Hold the clear-side flush until exit has returned.
                        let _ = held_tx.send(());
                        let _ = release_rx.recv();
                        handle.flush(TIMEOUT)
                    },
                    read_log_dir,
                )
            });
            let held = held_rx.recv();
            let started = Instant::now();
            let outcome = lifecycle.exit(TIMEOUT);
            let exit_elapsed = started.elapsed();
            log::error!("[lifecycle] record after shutdown");
            let _ = release_tx.send(());
            let clear_result = clearing.join();
            (held.map(|()| clear_result), exit_elapsed, outcome)
        });

        assert!(
            exit_elapsed < TIMEOUT,
            "exit took {exit_elapsed:?} with a clear flush held"
        );
        assert!(
            matches!(outcome, ExitOutcome::ShutDown { result: Ok(()) }),
            "final shutdown outcome: {outcome:?}"
        );
        assert_eq!(lifecycle.shutdown_requests(), 1);
        assert!(matches!(
            lifecycle.exit(TIMEOUT),
            ExitOutcome::AlreadyShutDown
        ));
        assert_eq!(
            lifecycle.shutdown_requests(),
            1,
            "exactly one final shutdown"
        );

        let clear_result = clear_result
            .map_err(|e| TestError(e.to_string()))?
            .map_err(|_| TestError("clear thread panicked".to_owned()))?;
        assert!(
            matches!(clear_result, Err(ClearError::ShutdownStarted)),
            "a clear racing exit is rejected, got {clear_result:?}"
        );

        let health = late_handle.health();
        assert_eq!(health.lifecycle, BridgeLifecycle::Stopped);
        assert_eq!(health.state, BridgeHealthState::Unavailable);

        let contents = fs::read_to_string(&log_path)?;
        assert!(
            contents.contains("record before exit"),
            "the rejected clear must not truncate the final records"
        );
        assert!(contents.contains("health at exit"));
        assert!(
            !contents.contains("record after shutdown"),
            "the logger accepted a record after shutdown"
        );
        Ok(())
    }

    #[test]
    fn exit_waits_for_a_truncating_clear_but_never_past_its_timeout() -> Result<(), TestError> {
        let scratch = ScratchDir::new("lifecycle-truncating")?;
        let log_path = scratch.path().join("app.log.jsonl");
        fs::write(&log_path, "active")?;
        let (lifecycle, calls) = installed_fake()?;
        let exit_timeout = Duration::from_millis(200);

        let (writing_tx, writing_rx) = mpsc::channel::<()>();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let (outcome, elapsed, clear_result) = std::thread::scope(|scope| {
            let (lifecycle, log_path) = (&lifecycle, &log_path);
            let clearing = scope.spawn(move || {
                lifecycle.clear(
                    log_path,
                    |()| Ok(()),
                    move |dir: &Path| {
                        // The write slot is held here: truncation is in progress.
                        let _ = writing_tx.send(());
                        let _ = release_rx.recv();
                        read_log_dir(dir)
                    },
                )
            });
            let writing = writing_rx.recv();
            let started = Instant::now();
            let outcome = lifecycle.exit(exit_timeout);
            let elapsed = started.elapsed();
            let _ = release_tx.send(());
            (writing.map(|()| outcome), elapsed, clearing.join())
        });
        let outcome = outcome.map_err(|e| TestError(e.to_string()))?;

        assert!(matches!(outcome, ExitOutcome::ShutDown { result: Ok(()) }));
        assert!(
            elapsed >= exit_timeout,
            "exit must wait for the truncating clear"
        );
        assert!(elapsed < TIMEOUT, "exit stays bounded: {elapsed:?}");
        // The whole budget went to waiting, so shutdown got what was left: nothing.
        assert_eq!(shutdown_timeouts(&calls), vec![Duration::ZERO]);
        assert_eq!(calls.reports.load(Ordering::SeqCst), 1);
        let clear_result = clear_result.map_err(|_| TestError("clear panicked".to_owned()))?;
        assert!(clear_result.is_ok(), "the in-progress clear completes");
        Ok(())
    }

    #[test]
    fn exit_passes_the_full_budget_to_shutdown_when_no_clear_is_writing() -> Result<(), TestError> {
        let (lifecycle, calls) = installed_fake()?;
        assert!(matches!(
            lifecycle.exit(TIMEOUT),
            ExitOutcome::ShutDown { result: Ok(()) }
        ));
        let timeouts = shutdown_timeouts(&calls);
        assert_eq!(timeouts.len(), 1);
        assert!(timeouts
            .iter()
            .all(|t| *t <= TIMEOUT && *t > Duration::ZERO));
        Ok(())
    }

    #[test]
    fn clear_is_rejected_once_shutdown_has_begun() -> Result<(), TestError> {
        let scratch = ScratchDir::new("lifecycle-rejected")?;
        let log_path = scratch.path().join("app.log.jsonl");
        fs::write(&log_path, "final records")?;
        let (lifecycle, _calls) = installed_fake()?;
        assert!(matches!(
            lifecycle.exit(TIMEOUT),
            ExitOutcome::ShutDown { .. }
        ));

        let result = lifecycle.clear(&log_path, |()| Ok(()), read_log_dir);

        assert!(matches!(result, Err(ClearError::ShutdownStarted)));
        assert_eq!(fs::read_to_string(&log_path)?, "final records");
        Ok(())
    }

    #[test]
    fn concurrent_exits_request_exactly_one_shutdown() -> Result<(), TestError> {
        let (lifecycle, calls) = installed_fake()?;
        let barrier = Barrier::new(4);
        let outcomes: Vec<ExitOutcome> = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        lifecycle.exit(TIMEOUT)
                    })
                })
                .collect();
            workers.into_iter().filter_map(|w| w.join().ok()).collect()
        });
        assert_eq!(outcomes.len(), 4);
        let shut_down = outcomes
            .iter()
            .filter(|o| matches!(o, ExitOutcome::ShutDown { .. }))
            .count();
        assert_eq!(shut_down, 1);
        assert!(outcomes
            .iter()
            .all(|o| !matches!(o, ExitOutcome::NotInstalled)));
        assert_eq!(lifecycle.shutdown_requests(), 1);
        assert_eq!(shutdown_timeouts(&calls).len(), 1);
        Ok(())
    }

    #[test]
    fn exit_before_install_is_a_noop_and_rejects_a_later_install() -> Result<(), TestError> {
        let lifecycle: LogLifecycle<FakeGuard> = LogLifecycle::new();
        assert!(matches!(lifecycle.exit(TIMEOUT), ExitOutcome::NotInstalled));
        assert_eq!(lifecycle.shutdown_requests(), 0);

        let calls = Arc::new(Calls::default());
        let result = lifecycle.install(FakeGuard(Arc::clone(&calls)), TIMEOUT);
        assert!(matches!(
            result,
            Err(InstallError::NotUninstalled {
                rejected_shutdown: Ok(())
            })
        ));
        assert_eq!(shutdown_timeouts(&calls), vec![TIMEOUT]);
        Ok(())
    }
}
