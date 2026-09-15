//! Backend for the Rust `br` CLI (`beads_rust`): `BeadsBackend`, `CliBackend`, `CloseSuggestions`.
//!
//! [`BrCli`] wraps a `Box<dyn CliInvoker>` (`btit-cli`) and delegates every
//! transport-neutral method to [`btit_cli::ops`]. br never uses Dolt
//! ([`btit_beads::backend::BeadsBackend::project_uses_dolt`] is always `false`, and
//! [`BrCli`] does not implement `DoltOperations`), always passes `--suggest-next` on
//! close ([`btit_beads::backend::CloseSuggestions::close_suggesting_next`]), and
//! offers only the seven common dependency relation types (no
//! `tracks`/`until`/`validates`).
//!
//! Skeleton registered in sprint b-4 with its final dependencies; the implementation
//! landed in b-6.

#![deny(missing_docs)]

mod backend;

#[doc(inline)]
pub use backend::{BrCli, BR_RELEASE_SOURCE};
