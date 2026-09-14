//! Compile-only proof that the event macros and `#[instrument]` need only the
//! `sc-observability-log` dependency: this package declares no other
//! `[dependencies]`, so any expansion path outside `::sc_observability_log`,
//! `::core` or `::std` fails to resolve.

use sc_observability_log::{Level, debug, error, event, info, instrument, trace, warn};

include!("../../sc-observability-log/tests/compat/events.rs");
include!("../../sc-observability-log/tests/compat/instrument.rs");

/// Consumer fixture for the bridge control surface (review finding R-A4-005).
///
/// A status provider as a non-owning consumer writes it: it receives only a
/// [`LogControl`](sc_observability_log::LogControl), never the `LogGuard`, and
/// reaches bounded flush and health through public items of
/// `sc-observability-log` alone (no `__private`, no `sc_observability::Logger`,
/// no other dependency). `tests/control_consumer.rs` runs it against an
/// installed bridge.
pub mod status {
    use std::time::Duration;

    use sc_observability_log::{BridgeHealthReport, FailureReport, LogControl};

    /// One status reading: a bounded flush result and the health report, both plain data.
    #[derive(Debug, Clone, PartialEq)]
    pub struct StatusReading {
        /// `Ok` when the flush completed; otherwise its serializable report.
        pub flush: Result<(), FailureReport>,
        /// The health report taken after the flush.
        pub health: BridgeHealthReport,
    }

    /// Flushes through `control` (bounded by `timeout`) and reads health.
    #[must_use]
    pub fn read_status(control: &LogControl, timeout: Duration) -> StatusReading {
        StatusReading {
            flush: control.flush(timeout).map_err(|error| error.report()),
            health: control.health(),
        }
    }
}
