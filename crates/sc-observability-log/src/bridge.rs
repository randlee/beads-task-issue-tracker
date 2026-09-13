//! The `log::Log` implementation installed by [`init`](crate::init).

use std::sync::atomic::Ordering;

use crate::handle::{self, THRESHOLD};
use crate::{DropCause, mapping};

/// Maps every enabled `log` record to a `LogEvent` and submits it through
/// [`__private::emit`](crate::__private::emit).
#[derive(Debug)]
pub(crate) struct Bridge;

impl log::Log for Bridge {
    /// Compares the record level against the threshold derived from `LoggerConfig.level`.
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        let rank = metadata.level() as usize;
        rank <= usize::from(THRESHOLD.load(Ordering::Relaxed))
    }

    /// Never blocks on I/O or queue capacity and never panics.
    ///
    /// Records below the threshold return at once. A record logged from inside
    /// the logger itself (a panic hook, sink or redactor running inside `emit`)
    /// is dropped and counted as `DropCause::ReentrantEmit`; a target the
    /// sanitizer cannot label is counted as `DropCause::InvalidEvent`.
    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let Some(installed) = handle::current_installed() else {
            handle::record_drop(DropCause::NotInstalled);
            return;
        };
        let parts = mapping::record_to_parts(record, &installed.options);
        drop(installed);
        match parts {
            Ok(parts) => crate::__private::emit(parts),
            Err(_label_error) => handle::record_drop(DropCause::InvalidEvent),
        }
    }

    /// No-op by design: `log::Log::flush` has no timeout or error channel, and
    /// sc-observability's flush is unbounded (`maintenance.rs:137-164`), so
    /// delegating would let any `log::logger().flush()` caller hang. Use
    /// [`LogGuard::flush`](crate::LogGuard::flush) for a bounded flush.
    fn flush(&self) {}
}
