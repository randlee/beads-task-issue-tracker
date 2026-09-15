//! `run::resolve_working_dir` precedence: `cwd`, then `BEADS_PATH`, then the current directory.
//!
//! A single test in its own binary, because it mutates the process environment.

use btit_cli::run::resolve_working_dir;
use btit_types::{CliClient, ProjectRef};

#[test]
fn resolve_working_dir_precedence() {
    let current = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();

    std::env::set_var("BEADS_PATH", "/from/beads-path");
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

    std::env::remove_var("BEADS_PATH");
    assert_eq!(
        resolve_working_dir(&ProjectRef::local(None), CliClient::Unknown).unwrap(),
        current
    );
}
