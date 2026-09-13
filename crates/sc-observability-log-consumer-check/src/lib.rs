//! Compile-only proof that the event macros need only the `sc-observability-log`
//! dependency: this package declares no other `[dependencies]`, so any expansion
//! path outside `::sc_observability_log`, `::core` or `::std` fails to resolve.

use sc_observability_log::{Level, debug, error, event, info, trace, warn};

include!("../../sc-observability-log/tests/compat/events.rs");
