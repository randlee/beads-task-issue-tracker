//! Procedural macros for `sc-observability-log`.
//!
//! This crate is an implementation detail of `sc-observability-log`, which
//! depends on it through an exact `=` version pin. Its expansions target the
//! `#[doc(hidden)] sc_observability_log::__private` module, which is outside
//! semver, so the two crates always move in lockstep.
//!
//! No macros are exported yet: the tracing-compatible event macros arrive in
//! sprint a-2 and `#[instrument]` in sprint a-3. Depend on
//! `sc-observability-log` rather than on this crate directly.
