// crates/sc-observability-log/tests/api_freeze.rs — frozen after a-1 merges
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration test: helper fns are not covered by clippy.toml allow-*-in-tests"
)]
#![allow(
    clippy::items_after_statements,
    clippy::match_same_arms,
    reason = "signature lock: derive-probe fns sit beside the assertions they serve, and one arm per variant keeps each match exhaustive by name"
)]

use sc_observability_log::{
    BridgeOptions, DropCause, DroppedEvents, FlushError, InitError, LogGuard, LoggerConfig,
    ShutdownError,
};
use std::time::Duration;

#[test]
fn a1_public_api_is_frozen() {
    let _: fn(LoggerConfig, BridgeOptions) -> Result<LogGuard, InitError> =
        sc_observability_log::init;
    let _: fn(&LogGuard, Duration) -> Result<(), FlushError> = LogGuard::flush;
    let _: fn(LogGuard, Duration) -> Result<(), ShutdownError> = LogGuard::shutdown;
    let _: fn(&LogGuard) -> DroppedEvents = LogGuard::dropped_events;
    let _: for<'a> fn(&'a LogGuard) -> Option<&'a std::path::Path> = LogGuard::active_log_path;
    let _: fn(&DroppedEvents, DropCause) -> u64 = DroppedEvents::get;
    let _: fn(&DroppedEvents) -> u64 = DroppedEvents::total;
    let _: fn(&InitError) -> sc_observability_log::ErrorCode = InitError::code;
    let _: fn(&InitError) -> sc_observability_log::Remediation = InitError::remediation;
    let _: fn(&FlushError) -> sc_observability_log::ErrorCode = FlushError::code;
    let _: fn(&FlushError) -> sc_observability_log::Remediation = FlushError::remediation;
    let _: fn(&ShutdownError) -> sc_observability_log::ErrorCode = ShutdownError::code;
    let _: fn(&ShutdownError) -> sc_observability_log::Remediation = ShutdownError::remediation;
    // Derives: removing one breaks compilation.
    fn dropped_events_derives<T: std::fmt::Debug + Clone + Copy + Default + PartialEq + Eq>() {}
    fn drop_cause_derives<T: std::fmt::Debug + Clone + Copy + PartialEq + Eq + std::hash::Hash>() {}
    dropped_events_derives::<DroppedEvents>();
    drop_cause_derives::<DropCause>();
    let _ = |a: sc_observability_log::ActionName| BridgeOptions {
        default_action: a,
        parse_bracket_action: true,
    };
    let _: Duration = sc_observability_log::DEFAULT_DROP_SHUTDOWN_TIMEOUT;
    let _: &[sc_observability_log::ErrorCode] = sc_observability_log::error_codes::ALL;
    // Exhaustive matches: adding, removing or reshaping a variant breaks this test.
    let _ = |e: InitError| match e {
        InitError::AlreadyInitialized => (),
        InitError::ForeignLoggerInstalled { source: _ } => (),
        InitError::IdentityResolution { source: _ } => (),
        InitError::Logger { source: _ } => (),
    };
    let _ = |e: FlushError| match e {
        FlushError::TimedOut { timeout: _ } | FlushError::Logger { source: _ } => (),
        FlushError::HelperSpawn { source: _ } | FlushError::HelperLost => (),
    };
    let _ = |e: ShutdownError| match e {
        ShutdownError::TimedOut { timeout: _ } | ShutdownError::FinalFlush { source: _ } => (),
        ShutdownError::HelperSpawn { source: _ } | ShutdownError::HelperLost => (),
    };
    let _ = |c: DropCause| match c {
        DropCause::QueueFull | DropCause::InvalidEvent | DropCause::WriterDegraded => (),
        DropCause::ShutdownTimedOut | DropCause::NotInstalled => (),
        DropCause::LoggerPanicked | DropCause::ReentrantEmit => (),
    };
    let _: [DropCause; 7] = DropCause::ALL;
}
