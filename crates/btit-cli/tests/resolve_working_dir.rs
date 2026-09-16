//! `run::resolve_working_dir` precedence: `cwd`, then `BEADS_PATH`, then the current directory.
//!
//! A single test in its own binary, because it mutates the process environment.

use std::ffi::OsString;

use btit_cli::run::resolve_working_dir;
use btit_types::{CliClient, ProjectRef};

/// Restores `BEADS_PATH` to the value it had when the guard was created (or removes
/// it if it was unset) on drop, so a panicking assertion mid-test cannot leak the
/// mutated environment variable into later test binaries.
struct BeadsPathGuard {
    previous: Option<OsString>,
}

impl BeadsPathGuard {
    fn set(value: &str) -> Self {
        let previous = std::env::var_os("BEADS_PATH");
        std::env::set_var("BEADS_PATH", value);
        Self { previous }
    }
}

impl Drop for BeadsPathGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(v) => std::env::set_var("BEADS_PATH", v),
            None => std::env::remove_var("BEADS_PATH"),
        }
    }
}

#[test]
fn resolve_working_dir_precedence() {
    let current = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let guard = BeadsPathGuard::set("/from/beads-path");
    assert_eq!(
        resolve_working_dir(
            &ProjectRef::local(Some("/from/cwd".to_string())),
            CliClient::Bd
        )
        .unwrap(),
        "/from/cwd"
    );
    assert_eq!(
        resolve_working_dir(&ProjectRef::local(None), CliClient::Bd).unwrap(),
        "/from/beads-path"
    );
    drop(guard);

    assert_eq!(
        resolve_working_dir(&ProjectRef::local(None), CliClient::Unknown).unwrap(),
        current
    );
}
