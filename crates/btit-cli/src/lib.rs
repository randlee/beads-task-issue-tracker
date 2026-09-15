//! Process transport shared by the `bd` and `br` backends.
//!
//! This crate spawns the beads CLI and owns everything both CLIs share:
//!
//! - [`path`]: the extended `PATH` every spawn uses, so GUI launches (Finder/Dock,
//!   minimal `PATH`) still resolve Homebrew, Go and Cargo installs.
//! - [`command`]: [`command::new_command`], a `std::process::Command` with
//!   `CREATE_NO_WINDOW` on Windows.
//! - [`locks`]: [`ProjectLocks`], the per-working-directory lock that serializes CLI
//!   calls against one project.
//! - [`run`]: the pure argv assembly ([`run::json_argv`], [`run::json_invocation`]) and
//!   the invocation functions (`--json` under the project lock, raw, `--version`).
//! - [`probe`]: the `--version` probe and default-binary auto-detection.
//! - [`runner`]: the [`CliInvoker`] trait and [`CliRunner`], one configured binary with
//!   its lazily probed, cached version.
//! - [`ops`]: one function per issue operation, generic over `&dyn CliInvoker`.
//! - `testing` (feature `test-support`): `RecordingInvoker`, a scripted invoker that
//!   spawns nothing and records every argv.
//!
//! The crate holds no process-global client state: a [`CliRunner`] owns its binary and
//! probe cache, and the [`ProjectLocks`] it uses is passed in at construction. Every
//! spawn blocks the calling thread until the child exits (`Command::output()`, no
//! timeout), exactly as the app did before the crate split.

#![deny(missing_docs)]

pub mod command;
pub mod locks;
pub mod ops;
pub mod path;
pub mod probe;
pub mod run;
pub mod runner;
#[cfg(feature = "test-support")]
pub mod testing;

#[doc(inline)]
pub use locks::ProjectLocks;
#[doc(inline)]
pub use runner::{CliInvoker, CliRunner, ProbeState};
