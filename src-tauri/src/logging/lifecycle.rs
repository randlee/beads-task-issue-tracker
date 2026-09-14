//! Single lifecycle owner for the process-wide `LogGuard` (review finding R-A4-001).
//!
//! [`LogLifecycle`] is the only place the non-cloneable guard lives. It
//! serializes the three operations that touch the logger's lifetime:
//!
//! - **clear** ([`LogLifecycle::clear`]) never holds the guard. It takes a
//!   non-owning `LogControl` (`LogGuard::control`) under the lock, releases the
//!   lock, flushes through the control, then re-acquires the lock to claim the
//!   single *clear-write slot* before truncating files. Waiting for another
//!   clear's slot is bounded by the clear's timeout
//!   ([`ClearError::WriteSlotTimedOut`]). Once shutdown has begun it is rejected
//!   with [`ClearError::ShutdownStarted`] and touches no file.
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
//! - **degraded exit** — if the exit deadline passes while a clear still holds
//!   the write slot, exit stops waiting on that file I/O: it skips the
//!   `[logging] health at exit` records (they could be truncated away or
//!   interleave with the truncation) and performs the one final shutdown with the
//!   remaining budget (zero), reporting [`ExitOutcome::ShutDownWhileClearWriting`].
//!   **Residual risk:** the clear may still truncate or delete files after the
//!   logger's final flush, so the last records before quit (including ones
//!   queued before the clear started) can be lost. Exit stays bounded; this
//!   trade-off is reached only when clear I/O stalls for the whole exit budget.
//!
//! No code path holds the lock across a flush, a shutdown or file I/O, and exit
//! as a whole is bounded by its timeout even while a clear is flushing or
//! truncating. `Drop for LogGuard` is only a fallback for a guard that never
//! reached `exit` (the static owner itself is never dropped).

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use sc_observability_log::{DropCause, FlushError, LogControl, LogGuard, ShutdownError};

use super::{clear_log_files, ClearError, LogDirEntry};

/// Guard operations the lifecycle owner needs: `LogGuard` in production, a fake in tests.
pub(crate) trait OwnedGuard: Send {
    /// Non-owning flush capability handed to a clear.
    type Control;

    /// Returns a control that neither keeps the logger alive nor can shut it down.
    fn control(&self) -> Self::Control;

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
    type Control = LogControl;

    fn control(&self) -> LogControl {
        LogGuard::control(self)
    }

    /// Writes the at-exit diagnostics: the last records before the final shutdown.
    ///
    /// Always on (plain `log::` macros, not gated by `LOGGING_ENABLED`), because
    /// after shutdown there is no other place the bridge's final state is kept.
    /// Emitted by every exit that owns the guard, except the degraded
    /// `ShutDownWhileClearWriting` case.
    ///
    /// 1. When any event was dropped, one `Warn` record: message
    ///    `dropped events: <Cause>=<n>, ...` (only causes with `n > 0`, `DropCause`
    ///    `Debug` names), action `logging`.
    /// 2. One `Info` record: message `health at exit`, action `logging`, target
    ///    `app_lib.logging.lifecycle`, and `fields.health` holding the
    ///    `BridgeHealthReport` serialized as a JSON **string**
    ///    (`schema_version` 1; parse it a second time to read it). The snapshot is
    ///    taken while the bridge is still running, so it shows `lifecycle:
    ///    "running"` with the final queue high-water mark, writer and sink errors
    ///    and drop counters. If serialization fails, a `Warn` record
    ///    `health at exit could not be serialized: <error>` is written instead.
    ///
    /// Purpose: post-mortem evidence in the JSONL file (and the debug panel) of
    /// whether the session lost records or ran degraded.
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
    /// The exit budget ran out while a clear still held the write slot.
    ///
    /// The one final shutdown ran anyway (with the remaining budget) and the
    /// at-exit health records were skipped; the clear's truncation may race the
    /// final flush (see the module docs, "degraded exit").
    ShutDownWhileClearWriting {
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

    /// Flushes through a `LogControl`, then truncates the log files; serialized with exit.
    ///
    /// `flush` receives the non-owning control (production:
    /// `|c| c.flush(LOG_IO_TIMEOUT)`); it is not called before installation.
    /// `list_dir` is the directory reader passed to [`clear_log_files`].
    /// `timeout` bounds the wait for another clear's write slot.
    ///
    /// # Errors
    ///
    /// - [`ClearError::ShutdownStarted`] when shutdown began before the flush or
    ///   before the write slot was claimed; no file is touched.
    /// - [`ClearError::WriteSlotTimedOut`] when another clear still held the
    ///   write slot after `timeout`; no file is touched.
    /// - [`ClearError::Flush`] when the flush failed (no file is touched).
    /// - Every file error of [`clear_log_files`].
    pub(crate) fn clear<I>(
        &self,
        log_path: &Path,
        timeout: Duration,
        flush: impl FnOnce(&G::Control) -> Result<(), FlushError>,
        list_dir: impl FnOnce(&Path) -> std::io::Result<I>,
    ) -> Result<(), ClearError>
    where
        I: IntoIterator<Item = std::io::Result<LogDirEntry>>,
    {
        let control = {
            let state = self.lock();
            match &state.phase {
                Phase::Uninstalled => None,
                Phase::Running(guard) => Some(guard.control()),
                Phase::ShuttingDown | Phase::Stopped => return Err(ClearError::ShutdownStarted),
            }
        };
        let flushed = control.as_ref().map_or(Ok(()), flush);
        let _write = self.claim_clear_write(timeout)?;
        flushed.map_err(|source| ClearError::Flush { source })?;
        clear_log_files(log_path, list_dir)
    }

    /// Waits (at most `timeout`) for any other clear's write slot, then claims it
    /// unless shutdown has begun.
    fn claim_clear_write(&self, timeout: Duration) -> Result<ClearWrite<'_, G>, ClearError> {
        let deadline = Instant::now().checked_add(timeout);
        let mut state = self.lock();
        loop {
            if matches!(state.phase, Phase::ShuttingDown | Phase::Stopped) {
                return Err(ClearError::ShutdownStarted);
            }
            if !state.clear_writing {
                break;
            }
            let left = remaining(deadline, timeout);
            if left.is_zero() {
                return Err(ClearError::WriteSlotTimedOut { timeout });
            }
            state = self
                .changed
                .wait_timeout(state, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        state.clear_writing = true;
        Ok(ClearWrite { lifecycle: self })
    }

    /// Performs the one final shutdown, bounded by `timeout` in total.
    ///
    /// Returns [`ExitOutcome::ShutDownWhileClearWriting`] when the budget ran out
    /// while a clear still held the write slot (see the module docs).
    pub(crate) fn exit(&self, timeout: Duration) -> ExitOutcome {
        let deadline = Instant::now().checked_add(timeout);
        let (guard, clear_still_writing) = {
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
            (guard, state.clear_writing)
        };
        if !clear_still_writing {
            // Safe to write: no clear can truncate these records any more.
            guard.report_before_shutdown();
        }
        self.shutdown_requests.fetch_add(1, Ordering::SeqCst);
        let result = guard.shutdown(remaining(deadline, timeout));
        self.lock().phase = Phase::Stopped;
        self.changed.notify_all();
        if clear_still_writing {
            ExitOutcome::ShutDownWhileClearWriting { result }
        } else {
            ExitOutcome::ShutDown { result }
        }
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
        type Control = ();

        fn control(&self) {}

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
        let late_control = guard.control();
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
                    TIMEOUT,
                    move |control: &LogControl| {
                        // Hold the clear-side flush until exit has returned.
                        let _ = held_tx.send(());
                        let _ = release_rx.recv();
                        control.flush(TIMEOUT)
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

        let health = late_control.health();
        assert_eq!(health.lifecycle, BridgeLifecycle::Stopped);
        assert_eq!(health.state, BridgeHealthState::Unavailable);

        let contents = fs::read_to_string(&log_path)?;
        assert!(
            contents.contains("record before exit"),
            "the rejected clear must not truncate the final records"
        );
        assert_health_at_exit_record(&contents)?;
        assert!(
            !contents.contains("record after shutdown"),
            "the logger accepted a record after shutdown"
        );
        Ok(())
    }

    /// ATM-QA-002: pins the documented shape of the `[logging] health at exit` record.
    fn assert_health_at_exit_record(contents: &str) -> Result<(), TestError> {
        use serde_json::Value;
        let text =
            |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
        let record = contents
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find(|event| text(event, "message").as_deref() == Some("health at exit"))
            .ok_or_else(|| TestError("no health-at-exit record".to_owned()))?;
        assert_eq!(text(&record, "level").as_deref(), Some("Info"));
        assert_eq!(
            text(&record, "target").as_deref(),
            Some("app_lib.logging.lifecycle")
        );
        assert_eq!(text(&record, "action").as_deref(), Some("logging"));
        let health = record
            .get("fields")
            .and_then(|fields| text(fields, "health"))
            .ok_or_else(|| TestError("health field is not a JSON string".to_owned()))?;
        let health: Value = serde_json::from_str(&health).map_err(|e| TestError(e.to_string()))?;
        assert_eq!(
            health.get("schema_version").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(text(&health, "lifecycle").as_deref(), Some("running"));
        assert!(health.get("dropped_events").is_some_and(Value::is_object));
        Ok(())
    }

    /// Runs `exit(exit_timeout)` while a fake clear holds the write slot inside
    /// its directory listing; the slot is released `release_after` after exit starts.
    fn exit_during_truncation(
        exit_timeout: Duration,
        release_after: Duration,
    ) -> Result<(ExitOutcome, Duration, Arc<Calls>), TestError> {
        let scratch = ScratchDir::new("lifecycle-truncating")?;
        let log_path = scratch.path().join("app.log.jsonl");
        fs::write(&log_path, "active")?;
        let (lifecycle, calls) = installed_fake()?;

        let (writing_tx, writing_rx) = mpsc::channel::<()>();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let (outcome, elapsed, clear_result) = std::thread::scope(|scope| {
            let (lifecycle, log_path) = (&lifecycle, &log_path);
            let clearing = scope.spawn(move || {
                lifecycle.clear(
                    log_path,
                    TIMEOUT,
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
            let releaser = scope.spawn(move || {
                std::thread::sleep(release_after);
                let _ = release_tx.send(());
            });
            let started = Instant::now();
            let outcome = lifecycle.exit(exit_timeout);
            let elapsed = started.elapsed();
            let _ = releaser.join();
            (writing.map(|()| outcome), elapsed, clearing.join())
        });
        let outcome = outcome.map_err(|e| TestError(e.to_string()))?;
        let clear_result = clear_result.map_err(|_| TestError("clear panicked".to_owned()))?;
        if clear_result.is_err() {
            return Err(TestError(format!(
                "the in-progress clear must complete: {clear_result:?}"
            )));
        }
        Ok((outcome, elapsed, calls))
    }

    #[test]
    fn exit_waits_for_a_truncating_clear_that_finishes_within_its_budget() -> Result<(), TestError>
    {
        let release_after = Duration::from_millis(100);
        let (outcome, elapsed, calls) = exit_during_truncation(TIMEOUT, release_after)?;

        assert!(matches!(outcome, ExitOutcome::ShutDown { result: Ok(()) }));
        assert!(
            elapsed >= release_after,
            "exit must wait for the truncating clear"
        );
        assert!(elapsed < TIMEOUT, "exit stays bounded: {elapsed:?}");
        assert_eq!(calls.reports.load(Ordering::SeqCst), 1);
        let timeouts = shutdown_timeouts(&calls);
        assert_eq!(timeouts.len(), 1);
        assert!(timeouts.iter().all(|t| *t > Duration::ZERO));
        Ok(())
    }

    /// RSH-001: the clear is still truncating when the exit budget runs out.
    #[test]
    fn exit_reports_a_degraded_shutdown_when_a_clear_is_still_truncating_at_its_deadline(
    ) -> Result<(), TestError> {
        let exit_timeout = Duration::from_millis(200);
        // Released only after exit has returned.
        let (outcome, elapsed, calls) = exit_during_truncation(exit_timeout, exit_timeout * 3)?;

        assert!(
            matches!(
                outcome,
                ExitOutcome::ShutDownWhileClearWriting { result: Ok(()) }
            ),
            "degraded outcome expected, got {outcome:?}"
        );
        assert!(elapsed >= exit_timeout, "exit used its whole budget");
        assert!(elapsed < TIMEOUT, "exit stays bounded: {elapsed:?}");
        assert_eq!(
            calls.reports.load(Ordering::SeqCst),
            0,
            "no at-exit records while a clear may truncate them"
        );
        // Exactly one final shutdown, with what was left of the budget: nothing.
        assert_eq!(shutdown_timeouts(&calls), vec![Duration::ZERO]);
        Ok(())
    }

    /// RSH-002: a second clear gives up waiting for a stuck write slot.
    #[test]
    fn clear_times_out_waiting_for_a_held_write_slot() -> Result<(), TestError> {
        let scratch = ScratchDir::new("lifecycle-slot-timeout")?;
        let log_path = scratch.path().join("app.log.jsonl");
        fs::write(&log_path, "active")?;
        let (lifecycle, _calls) = installed_fake()?;
        let slot_timeout = Duration::from_millis(100);

        let (writing_tx, writing_rx) = mpsc::channel::<()>();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let (second, elapsed, first) = std::thread::scope(|scope| {
            let (lifecycle, log_path) = (&lifecycle, &log_path);
            let first = scope.spawn(move || {
                lifecycle.clear(
                    log_path,
                    TIMEOUT,
                    |()| Ok(()),
                    move |dir: &Path| {
                        let _ = writing_tx.send(());
                        let _ = release_rx.recv();
                        read_log_dir(dir)
                    },
                )
            });
            let writing = writing_rx.recv();
            let started = Instant::now();
            let second = lifecycle.clear(log_path, slot_timeout, |()| Ok(()), read_log_dir);
            let elapsed = started.elapsed();
            let _ = release_tx.send(());
            (writing.map(|()| second), elapsed, first.join())
        });
        let second = second.map_err(|e| TestError(e.to_string()))?;

        assert!(
            matches!(second, Err(ClearError::WriteSlotTimedOut { timeout }) if timeout == slot_timeout),
            "expected WriteSlotTimedOut, got {second:?}"
        );
        assert!(elapsed >= slot_timeout, "waited for the slot: {elapsed:?}");
        assert!(elapsed < TIMEOUT, "the wait is bounded: {elapsed:?}");
        let first = first.map_err(|_| TestError("first clear panicked".to_owned()))?;
        assert!(first.is_ok(), "the slot holder still completes: {first:?}");
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

        let result = lifecycle.clear(&log_path, TIMEOUT, |()| Ok(()), read_log_dir);

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
