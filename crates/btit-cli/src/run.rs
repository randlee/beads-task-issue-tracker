//! Invocation functions: argv assembly, the `--json` call under the project lock,
//! raw calls, and the `--version` spawn.
//!
//! [`json_argv`] is the single assembler of a JSON invocation's argv and
//! [`json_invocation`] the single step from a cached probe to that argv. Both are
//! pure, so [`CliRunner`](crate::CliRunner) spawns, and the test-support
//! `RecordingInvoker` records, exactly the same vector.
//!
//! Every spawn sets `PATH` to [`get_extended_path`]; project invocations run in the
//! project's working directory with `BEADS_PATH` set to it. Spawns block until the
//! child exits (`Command::output()`, no timeout).

use std::env;
use std::process::Output;
use std::sync::atomic::Ordering;
use std::sync::PoisonError;

use btit_beads::error::BeadsError;
use btit_beads::gates::capabilities_for;
use btit_beads::logging::VERBOSE_LOGGING;
use btit_beads::{log_debug, log_error, log_info};
use btit_types::{CliClient, CliOutput, CliProbe, ProjectRef};

use crate::command::new_command;
use crate::locks::ProjectLocks;
use crate::path::get_extended_path;

/// The working directory for `project`: its `cwd`, else `BEADS_PATH`, else the
/// process current directory, else `"."`.
///
/// # Errors
///
/// [`BeadsError::Unsupported`] for a project reference that is not a local directory
/// (none exists today; `ProjectRef` is `#[non_exhaustive]`), attributed to `client`.
pub fn resolve_working_dir(project: &ProjectRef, client: CliClient) -> Result<String, BeadsError> {
    match project {
        ProjectRef::Local { cwd } => Ok(cwd
            .clone()
            .or_else(|| env::var("BEADS_PATH").ok())
            .unwrap_or_else(|| {
                env::current_dir()
                    .map_or_else(|_| ".".to_string(), |p| p.to_string_lossy().to_string())
            })),
        _ => Err(BeadsError::Unsupported {
            operation: "non-local project reference",
            client,
        }),
    }
}

/// The argv of one JSON invocation: the words of `command` (so `"comments add"` is two
/// words), then `args`, then `--no-daemon` when `no_daemon`, then `--json`.
#[must_use]
pub fn json_argv(command: &str, args: &[String], no_daemon: bool) -> Vec<String> {
    let mut full_args: Vec<String> = command.split_whitespace().map(str::to_owned).collect();
    full_args.extend(args.iter().cloned());
    if no_daemon {
        full_args.push("--no-daemon".to_owned());
    }
    full_args.push("--json".to_owned());
    full_args
}

/// Client (for [`resolve_working_dir`]) and argv for one JSON invocation, from the
/// cached probe.
///
/// `None` yields [`CliClient::Unknown`] and no `--no-daemon`; `Some(p)` adds
/// `--no-daemon` when `capabilities_for(p.client, p.version).supports_daemon_flag`.
#[must_use]
pub fn json_invocation(
    info: Option<&CliProbe>,
    command: &str,
    args: &[String],
) -> (CliClient, Vec<String>) {
    let client = info.map_or(CliClient::Unknown, |p| p.client);
    let no_daemon =
        info.is_some_and(|p| capabilities_for(p.client, p.version).supports_daemon_flag);
    (client, json_argv(command, args, no_daemon))
}

/// Spawns `binary` with exactly `argv` in `working_dir`, holding that directory's
/// project lock, and returns stdout.
///
/// Logs `[bd] <binary> <argv> | cwd: <dir>` before the spawn, `[bd] OK | <n> bytes`
/// after it, and a 500-character output preview when verbose logging is on.
///
/// # Errors
///
/// - [`BeadsError::Spawn`] (`operation: None`) when the process cannot be started.
/// - [`BeadsError::SchemaMigration`] when it fails with `no such column: spec_id` on stderr.
/// - [`BeadsError::CommandFailed`] for any other unsuccessful exit.
pub fn spawn_json(
    binary: &str,
    locks: &ProjectLocks,
    working_dir: &str,
    argv: &[String],
) -> Result<String, BeadsError> {
    log_info!("[bd] {} {} | cwd: {}", binary, argv.join(" "), working_dir);

    // Acquire per-project lock to prevent concurrent Dolt access (causes SIGSEGV).
    let project_lock = locks.guard(working_dir);
    let _guard = project_lock.lock().unwrap_or_else(PoisonError::into_inner);

    let output = new_command(binary)
        .args(argv)
        .current_dir(working_dir)
        .env("PATH", get_extended_path())
        .env("BEADS_PATH", working_dir)
        .output()
        .map_err(|e| {
            log_error!("[bd] Failed to execute {}: {}", binary, e);
            BeadsError::Spawn {
                binary: binary.to_string(),
                operation: None,
                source: e,
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log_error!(
            "[bd] Command failed | status: {} | stderr: {}",
            output.status,
            stderr
        );

        // Detect schema migration failure (bd 0.49.4 migration bug)
        if stderr.contains("no such column: spec_id") {
            log_error!("[bd] Schema migration failure detected - database needs repair");
            return Err(BeadsError::SchemaMigration {
                binary: binary.to_string(),
            });
        }

        return Err(BeadsError::CommandFailed {
            binary: binary.to_string(),
            status: output.status.code(),
            status_display: output.status.to_string(),
            stderr: stderr.to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    log_info!("[bd] OK | {} bytes", stdout.len());

    // Log output preview only if verbose mode is enabled
    if VERBOSE_LOGGING.load(Ordering::Relaxed) {
        let preview: String = stdout.chars().take(500).collect();
        log_debug!("[bd] Output: {}", preview);
    }

    Ok(stdout)
}

/// `<binary> <command…> <args…> [--no-daemon] --json` in `working_dir` under its
/// project lock: [`spawn_json`] over [`json_argv`].
///
/// # Errors
///
/// As [`spawn_json`].
pub fn run_json(
    binary: &str,
    no_daemon: bool,
    locks: &ProjectLocks,
    working_dir: &str,
    command: &str,
    args: &[String],
) -> Result<String, BeadsError> {
    spawn_json(
        binary,
        locks,
        working_dir,
        &json_argv(command, args, no_daemon),
    )
}

/// `<binary> <args…>` in `working_dir` with `PATH`/`BEADS_PATH`, no `--json`, no lock.
///
/// Any exit status is returned as `Ok`; the caller decides what a failure means.
///
/// # Errors
///
/// [`BeadsError::Spawn`] (`operation` = the first argument) when the process cannot be started.
pub fn run_raw(binary: &str, working_dir: &str, args: &[&str]) -> Result<CliOutput, BeadsError> {
    new_command(binary)
        .args(args)
        .current_dir(working_dir)
        .env("PATH", get_extended_path())
        .env("BEADS_PATH", working_dir)
        .output()
        .map(|output| cli_output(&output))
        .map_err(|source| BeadsError::Spawn {
            binary: binary.to_string(),
            operation: args.first().map(ToString::to_string),
            source,
        })
}

/// `<binary> --version`, run from the system temp directory (so bd never auto-migrates a
/// project as a side effect of the probe) with the extended `PATH`.
///
/// # Errors
///
/// [`BeadsError::Spawn`] (`operation: "--version"`) when the process cannot be started.
pub fn probe_version_output(binary: &str) -> Result<CliOutput, BeadsError> {
    new_command(binary)
        .arg("--version")
        .current_dir(env::temp_dir())
        .env("PATH", get_extended_path())
        .output()
        .map(|output| cli_output(&output))
        .map_err(|source| BeadsError::Spawn {
            binary: binary.to_string(),
            operation: Some("--version".to_string()),
            source,
        })
}

/// Lossy UTF-8 view of a finished process.
fn cli_output(output: &Output) -> CliOutput {
    CliOutput {
        status: output.status.code(),
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use btit_types::CliVersion;

    fn probe(client: CliClient, version: (u32, u32, u32)) -> CliProbe {
        CliProbe {
            client,
            version: Some(CliVersion::from(version)),
            raw: String::new(),
        }
    }

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn json_invocation_literal_outputs() {
        let bd = probe(CliClient::Bd, (0, 49, 6));
        let br = probe(CliClient::Br, (0, 1, 33));
        assert_eq!(
            json_invocation(Some(&bd), "ready", &[]),
            (CliClient::Bd, strings(&["ready", "--no-daemon", "--json"]))
        );
        assert_eq!(
            json_invocation(Some(&br), "ready", &[]),
            (CliClient::Br, strings(&["ready", "--json"]))
        );
        assert_eq!(
            json_invocation(None, "ready", &[]),
            (CliClient::Unknown, strings(&["ready", "--json"]))
        );
    }

    #[test]
    fn json_argv_splits_subcommand_and_appends_flags_last() {
        let args = strings(&["bd-1", "text with spaces"]);
        assert_eq!(
            json_argv("comments add", &args, false),
            strings(&["comments", "add", "bd-1", "text with spaces", "--json"])
        );
        assert_eq!(
            json_argv("comments add", &args, true),
            strings(&[
                "comments",
                "add",
                "bd-1",
                "text with spaces",
                "--no-daemon",
                "--json"
            ])
        );
    }

    #[test]
    fn run_raw_maps_spawn_failure_to_operation() {
        let dir = env::temp_dir();
        let wd = dir.to_string_lossy();
        let err = run_raw(
            "definitely-not-a-real-cli-binary-xyz",
            &wd,
            &["sync", "--no-daemon"],
        )
        .unwrap_err();
        assert!(matches!(
            &err,
            BeadsError::Spawn { binary, operation: Some(op), .. }
                if binary == "definitely-not-a-real-cli-binary-xyz" && op == "sync"
        ));
        assert!(err
            .to_string()
            .starts_with("Failed to run definitely-not-a-real-cli-binary-xyz sync: "));
    }

    #[test]
    fn spawn_json_maps_spawn_failure_like_execute_bd() {
        let dir = env::temp_dir();
        let wd = dir.to_string_lossy();
        let locks = ProjectLocks::new();
        let err = run_json(
            "definitely-not-a-real-cli-binary-xyz",
            false,
            &locks,
            &wd,
            "ready",
            &[],
        )
        .unwrap_err();
        assert!(matches!(
            &err,
            BeadsError::Spawn {
                operation: None,
                ..
            }
        ));
        assert!(err
            .to_string()
            .starts_with("Failed to execute definitely-not-a-real-cli-binary-xyz: "));
    }

    #[test]
    fn probe_version_output_maps_spawn_failure() {
        let err = probe_version_output("definitely-not-a-real-cli-binary-xyz").unwrap_err();
        assert!(matches!(&err, BeadsError::Spawn { operation: Some(op), .. } if op == "--version"));
    }
}
