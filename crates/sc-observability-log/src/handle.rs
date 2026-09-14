//! Process-global logger slot, drop accounting, the guarded submission core, and
//! the bounded flush / shutdown helpers.
//!
//! Every lock acquisition recovers from poisoning with `PoisonError::into_inner`:
//! the slot holds only an `Option<Arc<Installed>>`, which has no invariant a
//! panic can break.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, PoisonError, RwLock};
use std::time::{Duration, Instant};

use sc_observability::TryLogError;
use sc_observability_types::LevelFilter;

use crate::__private::EventParts;
use crate::{BridgeLifecycle, DropCause, DroppedEvents, FlushError, ShutdownError, health};

/// A rejected submission: every failure of the guarded core maps to exactly one [`DropCause`].
///
/// Implemented by `DropCause` (the facade and the macros, which discard the
/// result) and by `SubmitError` (`LogControl::submit`, which returns it).
pub(crate) trait Rejection: Sized {
    /// The single counter this rejection increments.
    fn drop_cause(&self) -> DropCause;
    /// The rejection for a call made while this thread is already inside the guard.
    fn reentrant() -> Self;
    /// The rejection for a panic caught inside the guard.
    fn panicked() -> Self;
}

impl Rejection for DropCause {
    fn drop_cause(&self) -> DropCause {
        *self
    }

    fn reentrant() -> Self {
        DropCause::ReentrantEmit
    }

    fn panicked() -> Self {
        DropCause::LoggerPanicked
    }
}

/// Everything the emit path needs, shared behind one `Arc`.
pub(crate) struct Installed {
    pub(crate) logger: sc_observability::Logger,
    pub(crate) service: sc_observability_types::ServiceName,
    pub(crate) identity: sc_observability_types::ProcessIdentity,
    pub(crate) options: crate::BridgeOptions,
}

pub(crate) static SLOT: RwLock<Option<Arc<Installed>>> = RwLock::new(None);
pub(crate) static INSTALLED: AtomicBool = AtomicBool::new(false);
/// Encoded `LevelFilter`: 0 = Off, 1 = Error, 2 = Warn, 3 = Info, 4 = Debug, 5 = Trace.
pub(crate) static THRESHOLD: AtomicU8 = AtomicU8::new(THRESHOLD_OFF);

pub(crate) const THRESHOLD_OFF: u8 = 0;

/// Encoded [`BridgeLifecycle`]; `Stopped` until `init` succeeds (no guard exists before).
static LIFECYCLE: AtomicU8 = AtomicU8::new(LIFECYCLE_STOPPED);
const LIFECYCLE_RUNNING: u8 = 0;
const LIFECYCLE_SHUTTING_DOWN: u8 = 1;
const LIFECYCLE_STOPPED: u8 = 2;
const LIFECYCLE_SHUTDOWN_TIMED_OUT: u8 = 3;

/// Publishes a lifecycle transition; read lock-free by health snapshots.
pub(crate) fn set_lifecycle(lifecycle: BridgeLifecycle) {
    let encoded = match lifecycle {
        BridgeLifecycle::Running => LIFECYCLE_RUNNING,
        BridgeLifecycle::ShuttingDown => LIFECYCLE_SHUTTING_DOWN,
        BridgeLifecycle::ShutdownTimedOut => LIFECYCLE_SHUTDOWN_TIMED_OUT,
        BridgeLifecycle::Stopped => LIFECYCLE_STOPPED,
    };
    LIFECYCLE.store(encoded, Ordering::SeqCst);
}

/// `ShuttingDown` -> `ShutdownTimedOut`, unless the detached helper already reached `Stopped`.
fn mark_shutdown_timed_out() {
    let _ = LIFECYCLE.compare_exchange(
        LIFECYCLE_SHUTTING_DOWN,
        LIFECYCLE_SHUTDOWN_TIMED_OUT,
        Ordering::SeqCst,
        Ordering::SeqCst,
    );
}

/// Current lifecycle phase.
pub(crate) fn lifecycle() -> BridgeLifecycle {
    match LIFECYCLE.load(Ordering::SeqCst) {
        LIFECYCLE_RUNNING => BridgeLifecycle::Running,
        LIFECYCLE_SHUTTING_DOWN => BridgeLifecycle::ShuttingDown,
        LIFECYCLE_SHUTDOWN_TIMED_OUT => BridgeLifecycle::ShutdownTimedOut,
        _ => BridgeLifecycle::Stopped,
    }
}

static QUEUE_FULL: AtomicU64 = AtomicU64::new(0);
static INVALID_EVENT: AtomicU64 = AtomicU64::new(0);
static WRITER_DEGRADED: AtomicU64 = AtomicU64::new(0);
static SHUTDOWN_TIMED_OUT: AtomicU64 = AtomicU64::new(0);
static NOT_INSTALLED: AtomicU64 = AtomicU64::new(0);
static LOGGER_PANICKED: AtomicU64 = AtomicU64::new(0);
static REENTRANT_EMIT: AtomicU64 = AtomicU64::new(0);

fn counter(cause: DropCause) -> &'static AtomicU64 {
    match cause {
        DropCause::QueueFull => &QUEUE_FULL,
        DropCause::InvalidEvent => &INVALID_EVENT,
        DropCause::WriterDegraded => &WRITER_DEGRADED,
        DropCause::ShutdownTimedOut => &SHUTDOWN_TIMED_OUT,
        DropCause::NotInstalled => &NOT_INSTALLED,
        DropCause::LoggerPanicked => &LOGGER_PANICKED,
        DropCause::ReentrantEmit => &REENTRANT_EMIT,
    }
}

/// Counts one dropped event. Wrapped by `__private::record_drop`.
pub(crate) fn record_drop(cause: DropCause) {
    counter(cause).fetch_add(1, Ordering::Relaxed);
}

/// Reads the current value of one drop counter.
pub(crate) fn drop_count(cause: DropCause) -> u64 {
    counter(cause).load(Ordering::Relaxed)
}

/// Snapshot of every drop counter.
pub(crate) fn dropped_events() -> DroppedEvents {
    DroppedEvents::from_counter(drop_count)
}

/// Encodes a `LevelFilter` for `THRESHOLD`.
pub(crate) fn encode_threshold(level: LevelFilter) -> u8 {
    match level {
        LevelFilter::Off => THRESHOLD_OFF,
        LevelFilter::Error => 1,
        LevelFilter::Warn => 2,
        LevelFilter::Info => 3,
        LevelFilter::Debug => 4,
        LevelFilter::Trace => 5,
    }
}

/// Maps a `LevelFilter` to the `log` facade's filter.
pub(crate) fn to_log_level_filter(level: LevelFilter) -> log::LevelFilter {
    match level {
        LevelFilter::Off => log::LevelFilter::Off,
        LevelFilter::Error => log::LevelFilter::Error,
        LevelFilter::Warn => log::LevelFilter::Warn,
        LevelFilter::Info => log::LevelFilter::Info,
        LevelFilter::Debug => log::LevelFilter::Debug,
        LevelFilter::Trace => log::LevelFilter::Trace,
    }
}

/// Encoded rank of an event level, comparable with `THRESHOLD`.
pub(crate) fn level_rank(level: sc_observability_types::Level) -> u8 {
    match level {
        sc_observability_types::Level::Error => 1,
        sc_observability_types::Level::Warn => 2,
        sc_observability_types::Level::Info => 3,
        sc_observability_types::Level::Debug => 4,
        sc_observability_types::Level::Trace => 5,
    }
}

/// Lock-free threshold check.
pub(crate) fn level_enabled(level: sc_observability_types::Level) -> bool {
    level_rank(level) <= THRESHOLD.load(Ordering::Relaxed)
}

thread_local! {
    static IN_EMIT: Cell<bool> = const { Cell::new(false) };
}

/// Per-thread reentrancy guard. Only `try_with` is used: `with` can panic.
pub(crate) struct EmitScope(());

impl EmitScope {
    /// Enters the scope, or returns `None` when this thread is already inside `emit`.
    pub(crate) fn enter() -> Option<Self> {
        match IN_EMIT.try_with(|flag| flag.replace(true)) {
            Ok(false) => Some(Self(())),
            // Reentrant, or TLS unavailable (impossible for a const Cell<bool>): treated as reentrant.
            Ok(true) | Err(_) => None,
        }
    }
}

impl Drop for EmitScope {
    fn drop(&mut self) {
        // Also runs while a panic unwinds, so the flag is always released.
        let _ = IN_EMIT.try_with(|flag| flag.set(false));
    }
}

/// The guarded submission core: reentrancy guard plus panic containment.
///
/// This is the single outermost boundary of every submission, shared by the
/// `log` facade, the event macros / `#[instrument]` and `LogControl::submit`:
/// exactly one call per record. A rejection is counted under its one
/// [`DropCause`] *before* it is returned, so a caller that discards the result
/// (the facade and the macros) still leaves exactly-once drop accounting.
///
/// The closure must reach the logger only through the unguarded cores
/// [`submit_installed`] / [`submit_to`]; calling a guarded entry point from
/// inside the closure would classify the record as `DropCause::ReentrantEmit`.
pub(crate) fn submit_guarded<E: Rejection>(
    submit: impl FnOnce() -> Result<(), E>,
) -> Result<(), E> {
    let Some(_scope) = EmitScope::enter() else {
        let rejection = E::reentrant();
        record_drop(rejection.drop_cause());
        return Err(rejection);
    };
    // AssertUnwindSafe: the closure reaches Logger, which holds dyn LogSink / dyn Redactor /
    // dyn ProcessIdentityResolver; none is RefUnwindSafe (E0277 without the wrapper). After a
    // caught panic nothing reads logger state except try_log itself, which reports its
    // poisoned mutexes by panicking again.
    let result =
        catch_unwind(AssertUnwindSafe(submit)).unwrap_or_else(|_payload| Err(E::panicked()));
    if let Err(rejection) = &result {
        record_drop(rejection.drop_cause());
    }
    result
}

/// Unguarded submit core: event assembly, ambient trace context and `Logger::try_log`.
///
/// Never call it outside a [`submit_guarded`] closure: it neither contains panics
/// nor detects reentrancy.
pub(crate) fn submit_to(installed: &Installed, parts: EventParts) -> Result<(), DropCause> {
    let mut event = crate::mapping::assemble_event(
        parts,
        &installed.service,
        &installed.identity,
        &installed.options.default_action,
    );
    // Ambient `#[instrument]` context of the emitting thread; bridge records included.
    event.trace = crate::context::current_trace();
    installed
        .logger
        .try_log(event)
        .map_err(|error| match error {
            TryLogError::QueueFull(_) => DropCause::QueueFull,
            TryLogError::InvalidEvent(_) => DropCause::InvalidEvent,
            TryLogError::WriterDegraded(_) => DropCause::WriterDegraded,
            TryLogError::ShutdownTimedOut(_) => DropCause::ShutdownTimedOut,
        })
}

/// Unguarded submit core for pre-built parts: slot read, then [`submit_to`].
///
/// Same contract as [`submit_to`]: call it only inside a [`submit_guarded`] closure.
pub(crate) fn submit_installed(parts: EventParts) -> Result<(), DropCause> {
    let installed = current_installed().ok_or(DropCause::NotInstalled)?;
    submit_to(&installed, parts)
}

/// Crate-private failure of [`run_bounded`], mapped 1:1 into `FlushError` / `ShutdownError`.
#[derive(Debug)]
pub(crate) enum BoundedError {
    /// The work did not finish within the timeout; the helper is detached.
    TimedOut,
    /// The helper thread could not be started.
    Spawn { source: std::io::Error },
    /// The channel disconnected without a value: the work closure panicked.
    WorkerLost,
}

/// Helpers whose caller timed out and whose work has not finished (flush and shutdown).
///
/// Incremented by the caller when its timeout elapses, decremented by the helper
/// when its work returns or unwinds. Read by health snapshots
/// (`BridgeHealthReport.helpers.detached`).
static DETACHED_HELPERS: AtomicU32 = AtomicU32::new(0);

/// Set while a flush helper runs: at most one flush helper per installed bridge.
static FLUSH_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// Per-helper state shared by the caller and the helper of one [`run_bounded`] call.
const HELPER_RUNNING: u8 = 0;
/// The caller timed out and counted the helper in its detached counter.
const HELPER_DETACHED: u8 = 1;
/// The helper's work returned or unwound.
const HELPER_DONE: u8 = 2;

/// Marks the helper done when its work returns or unwinds, and uncounts it if detached.
struct HelperExit {
    state: Arc<AtomicU8>,
    detached: &'static AtomicU32,
}

impl Drop for HelperExit {
    fn drop(&mut self) {
        // Also runs while a panic in the work unwinds.
        if self.state.swap(HELPER_DONE, Ordering::SeqCst) == HELPER_DETACHED {
            self.detached.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

/// Exclusive claim on a single-flight flag; released on drop, including during unwinding.
struct Flight {
    flag: &'static AtomicBool,
}

impl Flight {
    /// Claims `flag` without blocking, or returns `None` while another claim is alive.
    fn claim(flag: &'static AtomicBool) -> Option<Self> {
        flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| Self { flag })
    }
}

impl Drop for Flight {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

/// Health of the bounded-operation helper threads.
pub(crate) fn helper_health() -> crate::HelperHealth {
    crate::HelperHealth {
        flush_in_flight: FLUSH_IN_FLIGHT.load(Ordering::SeqCst),
        detached: DETACHED_HELPERS.load(Ordering::SeqCst),
    }
}

/// Runs `work` on a named helper thread and waits at most `timeout` for its result.
///
/// A helper left running past `timeout` is counted in the process-wide detached
/// counter until its work finishes.
pub(crate) fn run_bounded<T: Send + 'static>(
    timeout: Duration,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, BoundedError> {
    run_bounded_in(&DETACHED_HELPERS, timeout, work)
}

/// [`run_bounded`] against an explicit detached counter (unit tests use their own).
fn run_bounded_in<T: Send + 'static>(
    detached: &'static AtomicU32,
    timeout: Duration,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, BoundedError> {
    let (tx, rx) = mpsc::sync_channel(1);
    let state = Arc::new(AtomicU8::new(HELPER_RUNNING));
    let exit = HelperExit {
        state: Arc::clone(&state),
        detached,
    };
    std::thread::Builder::new()
        .name("sc-observability-log-helper".to_owned())
        .spawn(move || {
            let exit = exit;
            let value = work();
            // Done before the result is observable, so a caller that received it never
            // sees this helper as still running.
            drop(exit);
            let _ = tx.send(value);
        })
        .map_err(|source| BoundedError::Spawn { source })?;
    match rx.recv_timeout(timeout) {
        Ok(value) => Ok(value),
        Err(RecvTimeoutError::Disconnected) => Err(BoundedError::WorkerLost),
        Err(RecvTimeoutError::Timeout) => {
            // Count first, then publish: the helper's decrement can never precede this increment.
            detached.fetch_add(1, Ordering::SeqCst);
            if state
                .compare_exchange(
                    HELPER_RUNNING,
                    HELPER_DETACHED,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_ok()
            {
                return Err(BoundedError::TimedOut);
            }
            // The work finished at the deadline: its result is one non-blocking send away.
            detached.fetch_sub(1, Ordering::SeqCst);
            rx.recv()
                .map_err(|mpsc::RecvError| BoundedError::WorkerLost)
        }
    }
}

const UNWRAP_BACKOFF_START: Duration = Duration::from_millis(1);
const UNWRAP_BACKOFF_MAX: Duration = Duration::from_millis(50);

/// Retries `Arc::try_unwrap` with a sleep backoff until `deadline` (`None`: no deadline).
///
/// At the deadline the clone held by this call is dropped and `None` is returned.
pub(crate) fn take_sole<T>(mut shared: Arc<T>, deadline: Option<Instant>) -> Option<T> {
    let mut backoff = UNWRAP_BACKOFF_START;
    loop {
        match Arc::try_unwrap(shared) {
            Ok(value) => return Some(value),
            Err(still_shared) => {
                let now = Instant::now();
                let remaining = match deadline {
                    Some(d) if now >= d => return None, // drops this clone
                    Some(d) => d.saturating_duration_since(now),
                    None => UNWRAP_BACKOFF_MAX,
                };
                shared = still_shared;
                std::thread::sleep(backoff.min(remaining));
                backoff = backoff.saturating_mul(2).min(UNWRAP_BACKOFF_MAX);
            }
        }
    }
}

/// Crate-private outcome of the shutdown helper.
#[derive(Debug)]
pub(crate) enum ShutdownStep {
    /// Sole ownership was not gained (unreachable: the helper waits without a deadline).
    StillShared,
    /// The final flush failed; the logger was still shut down.
    FinalFlush {
        source: sc_observability_types::FlushError,
    },
}

/// Shutdown helper. `Logger::shutdown(self)` consumes the logger (runtime.rs:224), so the
/// helper first gains sole ownership.
///
/// The caller waits at most `timeout`. The helper itself has no deadline: an
/// `Arc<Installed>` clone held past `timeout` (a detached `flush` helper, or a
/// submission blocked in a sink or redactor) makes the caller return
/// `ShutdownError::TimedOut` while the detached helper keeps waiting. When it
/// completes late it stores the final health report and publishes
/// `BridgeLifecycle::Stopped`, so the late completion is observable.
pub(crate) fn shutdown_installed(
    installed: Arc<Installed>,
    timeout: Duration,
) -> Result<(), ShutdownError> {
    let outcome = run_bounded(timeout, move || {
        let Some(sole) = take_sole(installed, None) else {
            return Err(ShutdownStep::StillShared);
        };
        let flushed = sole.logger.flush();
        let stopped = sole.logger.shutdown();
        // Final health for post-shutdown snapshots (writer state `Stopped`).
        if let Some(report) = health::read_report(&stopped) {
            health::store_final_report(report);
        }
        set_lifecycle(BridgeLifecycle::Stopped);
        flushed.map_err(|source| ShutdownStep::FinalFlush { source })
    });
    match outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(ShutdownStep::StillShared)) | Err(BoundedError::TimedOut) => {
            Err(ShutdownError::TimedOut { timeout })
        }
        Ok(Err(ShutdownStep::FinalFlush { source })) => Err(ShutdownError::FinalFlush { source }),
        Err(BoundedError::Spawn { source }) => Err(ShutdownError::HelperSpawn { source }),
        Err(BoundedError::WorkerLost) => Err(ShutdownError::HelperLost),
    }
}

/// `LogGuard::shutdown` and `Drop for LogGuard` both call this once.
///
/// The lifecycle is `ShuttingDown` from the first statement. When this returns
/// it is `Stopped` for every result except `ShutdownError::TimedOut`, which
/// leaves `ShutdownTimedOut` until the detached helper completes and publishes
/// `Stopped` (see [`shutdown_installed`]).
pub(crate) fn shutdown_sequence(timeout: Duration) -> Result<(), ShutdownError> {
    set_lifecycle(BridgeLifecycle::ShuttingDown);
    THRESHOLD.store(THRESHOLD_OFF, Ordering::SeqCst);
    log::set_max_level(log::LevelFilter::Off);
    let taken = SLOT.write().unwrap_or_else(PoisonError::into_inner).take();
    let result = match taken {
        Some(installed) => shutdown_installed(installed, timeout),
        None => Ok(()),
    };
    if matches!(result, Err(ShutdownError::TimedOut { .. })) {
        mark_shutdown_timed_out();
    } else {
        set_lifecycle(BridgeLifecycle::Stopped);
    }
    result
}

/// Clones the installed `Arc` out of the slot; the read lock is held only for the clone.
pub(crate) fn current_installed() -> Option<Arc<Installed>> {
    SLOT.read().unwrap_or_else(PoisonError::into_inner).clone()
}

/// `LogGuard::flush` / `LogControl::flush`: the helper owns an `Arc` clone until
/// sc-observability's flush returns.
///
/// An empty slot means shutdown has taken the logger: `FlushError::ShutDown`. A
/// flush whose helper already holds its clone when shutdown starts is awaited by
/// the shutdown's `take_sole`, within the shutdown's own timeout.
///
/// At most one flush helper runs at a time. While one is running (also after its
/// caller timed out and detached it), a new flush spawns nothing and returns
/// `FlushError::InProgress`, so a stuck sink cannot accumulate helper threads.
pub(crate) fn flush_installed(timeout: Duration) -> Result<(), FlushError> {
    let Some(installed) = current_installed() else {
        return Err(FlushError::ShutDown);
    };
    let Some(flight) = Flight::claim(&FLUSH_IN_FLIGHT) else {
        return Err(FlushError::InProgress);
    };
    let flush = move || {
        // Released when the flush returns or unwinds, before the result is sent.
        let _flight = flight;
        installed.logger.flush()
    };
    match run_bounded(timeout, flush) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(source)) => Err(FlushError::Logger { source }),
        Err(BoundedError::TimedOut) => Err(FlushError::TimedOut { timeout }),
        Err(BoundedError::Spawn { source }) => Err(FlushError::HelperSpawn { source }),
        Err(BoundedError::WorkerLost) => Err(FlushError::HelperLost),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_bounded_times_out_on_blocked_work() {
        let started = Instant::now();
        let result = run_bounded(Duration::from_millis(100), || {
            std::thread::sleep(Duration::from_secs(2));
        });
        assert!(matches!(result, Err(BoundedError::TimedOut)));
        assert!(started.elapsed() < Duration::from_millis(600));
    }

    #[test]
    fn run_bounded_reports_worker_lost_on_panic() {
        let result: Result<(), BoundedError> = run_bounded(Duration::from_secs(5), || {
            panic!("worker panic (expected by this test)");
        });
        assert!(matches!(result, Err(BoundedError::WorkerLost)));
    }

    /// Signals its channel when dropped: the work closure has returned.
    struct SignalOnDrop(mpsc::SyncSender<()>);

    impl Drop for SignalOnDrop {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }

    /// A helper is counted as detached from its caller's timeout until its work finishes.
    #[test]
    fn run_bounded_counts_a_detached_helper_until_it_finishes() {
        static DETACHED: AtomicU32 = AtomicU32::new(0);
        let (release_tx, release_rx) = mpsc::sync_channel::<()>(0);
        let (done_tx, done_rx) = mpsc::sync_channel::<()>(1);
        let result = run_bounded_in(&DETACHED, Duration::from_millis(20), move || {
            let _done = SignalOnDrop(done_tx);
            release_rx.recv().unwrap();
        });
        assert!(matches!(result, Err(BoundedError::TimedOut)));
        assert_eq!(DETACHED.load(Ordering::SeqCst), 1);
        release_tx.send(()).unwrap();
        done_rx.recv().unwrap();
        // The work has returned; the helper's exit guard runs right after the closure.
        let deadline = Instant::now() + Duration::from_secs(10);
        while DETACHED.load(Ordering::SeqCst) != 0 {
            assert!(Instant::now() < deadline, "detached helper never uncounted");
            std::thread::yield_now();
        }
        assert_eq!(
            run_bounded_in(&DETACHED, Duration::from_secs(5), || 3).ok(),
            Some(3)
        );
        assert_eq!(DETACHED.load(Ordering::SeqCst), 0);
    }

    /// A panicking detached helper is uncounted by its unwinding exit guard.
    #[test]
    fn run_bounded_uncounts_a_detached_helper_that_panics() {
        static DETACHED: AtomicU32 = AtomicU32::new(0);
        let (release_tx, release_rx) = mpsc::sync_channel::<()>(0);
        let result: Result<(), BoundedError> =
            run_bounded_in(&DETACHED, Duration::from_millis(20), move || {
                release_rx.recv().unwrap();
                panic!("detached helper panic (expected by this test)");
            });
        assert!(matches!(result, Err(BoundedError::TimedOut)));
        assert_eq!(DETACHED.load(Ordering::SeqCst), 1);
        release_tx.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while DETACHED.load(Ordering::SeqCst) != 0 {
            assert!(
                Instant::now() < deadline,
                "panicking helper never uncounted"
            );
            std::thread::yield_now();
        }
    }

    #[test]
    fn flight_claim_is_exclusive_and_released_on_drop_or_panic() {
        static FLAG: AtomicBool = AtomicBool::new(false);
        let first = Flight::claim(&FLAG).unwrap();
        assert!(Flight::claim(&FLAG).is_none());
        drop(first);
        let second = Flight::claim(&FLAG).unwrap();
        let unwound = std::thread::spawn(move || {
            let _held = second;
            panic!("flight holder panic (expected by this test)");
        })
        .join();
        assert!(unwound.is_err());
        assert!(!FLAG.load(Ordering::SeqCst));
        assert!(Flight::claim(&FLAG).is_some());
    }

    #[test]
    fn run_bounded_returns_value() {
        assert!(matches!(run_bounded(Duration::from_secs(5), || 7), Ok(7)));
    }

    #[test]
    fn take_sole_succeeds_once_clone_is_released() {
        let shared = Arc::new(5_u32);
        let clone = Arc::clone(&shared);
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            drop(clone);
        });
        let deadline = Instant::now().checked_add(Duration::from_secs(5));
        assert_eq!(take_sole(shared, deadline), Some(5));
        releaser.join().unwrap();
    }

    #[test]
    fn take_sole_gives_up_at_deadline() {
        let shared = Arc::new(5_u32);
        let held = Arc::clone(&shared);
        let started = Instant::now();
        let deadline = started.checked_add(Duration::from_millis(100));
        assert_eq!(take_sole(shared, deadline), None);
        assert!(started.elapsed() < Duration::from_millis(400));
        assert_eq!(Arc::strong_count(&held), 1);
    }

    #[test]
    fn threshold_encoding_orders_levels() {
        assert_eq!(encode_threshold(LevelFilter::Off), THRESHOLD_OFF);
        assert!(
            level_rank(sc_observability_types::Level::Error)
                <= encode_threshold(LevelFilter::Error)
        );
        assert!(
            level_rank(sc_observability_types::Level::Warn) > encode_threshold(LevelFilter::Error)
        );
        assert!(
            level_rank(sc_observability_types::Level::Trace)
                <= encode_threshold(LevelFilter::Trace)
        );
        assert_eq!(
            to_log_level_filter(LevelFilter::Debug),
            log::LevelFilter::Debug
        );
        for (filter, log_filter) in [
            (LevelFilter::Off, log::LevelFilter::Off),
            (LevelFilter::Error, log::LevelFilter::Error),
            (LevelFilter::Warn, log::LevelFilter::Warn),
            (LevelFilter::Info, log::LevelFilter::Info),
            (LevelFilter::Debug, log::LevelFilter::Debug),
            (LevelFilter::Trace, log::LevelFilter::Trace),
        ] {
            // The encoding matches the log facade's own ordering (Off = 0 .. Trace = 5).
            assert_eq!(usize::from(encode_threshold(filter)), log_filter as usize);
        }
    }

    /// The only unit test that calls `submit_guarded` or `record_drop`, so its counter deltas cannot race.
    #[test]
    fn emit_core_counts_panics_and_reentry() {
        // 1. A nested call inside the closure is counted once as ReentrantEmit.
        let reentrant_before = drop_count(DropCause::ReentrantEmit);
        let panicked_before = drop_count(DropCause::LoggerPanicked);
        let _ = submit_guarded(|| {
            assert_eq!(
                submit_guarded(|| Ok::<(), DropCause>(())),
                Err(DropCause::ReentrantEmit)
            );
            Ok::<(), DropCause>(())
        });
        assert_eq!(drop_count(DropCause::ReentrantEmit), reentrant_before + 1);
        assert_eq!(drop_count(DropCause::LoggerPanicked), panicked_before);

        // 2. A panicking closure is counted once as LoggerPanicked, and the panic hook's
        //    own submit_guarded call (same thread, still inside emit) once as ReentrantEmit.
        std::panic::set_hook(Box::new(|_| {
            let _ = submit_guarded(|| Ok::<(), DropCause>(()));
        }));
        let panicked: Result<(), DropCause> =
            submit_guarded(|| panic!("simulated sc-observability panic"));
        assert_eq!(panicked, Err(DropCause::LoggerPanicked));
        // 3. Remove the hook.
        let _ = std::panic::take_hook();
        assert_eq!(drop_count(DropCause::LoggerPanicked), panicked_before + 1);
        assert_eq!(drop_count(DropCause::ReentrantEmit), reentrant_before + 2);

        // 4. The EmitScope was released during unwinding: a fresh call is not counted.
        assert_eq!(submit_guarded(|| Ok::<(), DropCause>(())), Ok(()));
        assert_eq!(drop_count(DropCause::ReentrantEmit), reentrant_before + 2);
        assert_eq!(drop_count(DropCause::LoggerPanicked), panicked_before + 1);

        // A closure error is counted under its own cause.
        let invalid_before = drop_count(DropCause::InvalidEvent);
        assert_eq!(
            submit_guarded(|| Err(DropCause::InvalidEvent)),
            Err(DropCause::InvalidEvent)
        );
        assert_eq!(drop_count(DropCause::InvalidEvent), invalid_before + 1);
        record_drop(DropCause::QueueFull);
        let snapshot = dropped_events();
        assert_eq!(
            snapshot.total(),
            DropCause::ALL
                .iter()
                .map(|cause| snapshot.get(*cause))
                .sum::<u64>()
        );
    }
}
