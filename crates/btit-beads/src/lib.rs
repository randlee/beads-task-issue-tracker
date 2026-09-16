//! Beads backend contract, error type and the pure beads logic shared by every backend.
//!
//! This crate sits one level above `btit-types` and holds:
//!
//! - [`backend`]: the backend contract, [`BeadsBackend`] (transport-neutral),
//!   [`CliBackend`] (what only a spawned CLI has), and the backend-specific
//!   [`DoltOperations`] and [`CloseSuggestions`] traits, reached through optional
//!   accessors. No implementation lives here.
//! - [`error`]: [`BeadsError`], the only error type that crosses crate boundaries.
//! - [`detect`], [`compat`], [`gates`], [`issues`], [`parse`]: pure logic (inputs to
//!   outputs, plus gated log lines) for CLI detection, compatibility warnings,
//!   version gates, issue normalization and tolerant JSON parsing.
//! - [`logging`]: the `LOGGING_ENABLED`/`VERBOSE_LOGGING` gate and the exported
//!   `log_info!`, `log_warn!`, `log_error!` and `log_debug!` macros.
//!
//! The crate performs no process spawning and no filesystem access, and holds no
//! process-global client state: the two logging atomics are its only statics. The
//! contract is documented in `docs/backend-contract.md`.

#![deny(missing_docs)]

// Declared first so the exported `log_*!` macros are in textual scope for every
// module below, exactly as `#[macro_use] mod logging;` provided them in the app.
#[macro_use]
pub mod logging;

pub mod backend;
pub mod compat;
pub mod detect;
pub mod error;
pub mod gates;
pub mod issues;
pub mod parse;

#[cfg(test)]
#[expect(
    clippy::uninlined_format_args,
    reason = "fixtures moved verbatim from btit-app in b-3"
)]
mod test_support;

#[doc(inline)]
pub use backend::{BeadsBackend, CliBackend, CloseSuggestions, DoltOperations};
#[doc(inline)]
pub use error::{BeadsError, ExpectedShape, ParseTarget};
