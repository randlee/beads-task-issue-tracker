//! Backend for the Go `bd` CLI (steveyegge/beads): `BeadsBackend`, `CliBackend`, `DoltOperations`.
//!
//! [`backend::BdCli`] wraps a `Box<dyn CliInvoker>` (a `btit_cli::CliRunner` in
//! production, `btit_cli::testing::RecordingInvoker` in tests) and delegates issue
//! operations to `btit_cli::ops`. [`dolt`] holds the only backend-specific logic: the
//! Dolt-detection core ([`dolt::project_uses_dolt_for`], the only backend that can say
//! yes), the [`dolt::DoltMode`] switch resolved by [`dolt::project_dolt_mode`], and the
//! `DoltOperations` argument builders.

#![deny(missing_docs)]

pub mod backend;
pub mod dolt;

#[doc(inline)]
pub use backend::{BdCli, BD_RELEASE_SOURCE};
#[doc(inline)]
pub use dolt::{project_dolt_mode, project_uses_dolt_for, DoltMode};
