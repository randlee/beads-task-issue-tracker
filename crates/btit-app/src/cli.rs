use crate::config::get_cli_binary;
use btit_beads::error::BeadsError;
use btit_cli::command::new_command;
use btit_cli::path::get_extended_path;
use btit_cli::{run, CliInvoker, ProjectLocks};
use btit_types::{CliClient, CliOutput, CliProbe, CompatibilityInfo, ProjectRef};
// Pure detection, compatibility and gate logic lives in btit-beads (b-3); re-exported so
// config.rs, updates.rs, migration.rs, issue_commands.rs, polling.rs and lib.rs keep their paths.
pub(crate) use btit_beads::compat::cli_compatibility_warnings;
pub(crate) use btit_beads::detect::{
    cli_client_name, detect_cli_client, is_legacy_bd, parse_bd_version, MIN_SUPPORTED_BD_MAJOR,
};
pub(crate) use btit_beads::gates::{
    supports_daemon_flag_for, supports_delete_hard_flag_for, supports_list_all_flag_for,
    uses_dolt_backend_for, uses_jsonl_files_for,
};
use std::sync::{Arc, LazyLock, Mutex};

// Per-project mutex to prevent concurrent bd/Dolt access.
// bd 0.55 uses embedded Dolt which crashes (SIGSEGV) when two bd processes
// access the same database simultaneously. This serializes all bd calls per project.
// Transitional (b-4): shared with btit-cli's invocations through AppInvoker; removed in b-7.
pub(crate) static PROJECT_LOCKS: LazyLock<Arc<ProjectLocks>> =
    LazyLock::new(|| Arc::new(ProjectLocks::default()));

// Cached CLI client info — detected once on first use
// Stores: (client_type, major, minor, patch)
pub(crate) static CLI_CLIENT_INFO: LazyLock<Mutex<Option<(CliClient, u32, u32, u32)>>> =
    LazyLock::new(|| Mutex::new(None));


/// Detect and cache the CLI client type and version. Runs `binary --version` once.
pub(crate) fn get_cli_client_info() -> Option<(CliClient, u32, u32, u32)> {
    let mut cached = CLI_CLIENT_INFO.lock().unwrap();
    if let Some(info) = *cached {
        return Some(info);
    }

    let binary = get_cli_binary();
    // Run from temp dir to avoid bd auto-migrating projects in cwd
    let output = new_command(&binary)
        .arg("--version")
        .current_dir(std::env::temp_dir())
        .env("PATH", get_extended_path())
        .output()
        .ok()?;

    if !output.status.success() {
        log_warn!("[cli_detect] Failed to get version from {}", binary);
        return None;
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let trimmed = version_str.trim();
    let client = detect_cli_client(trimmed);
    let tuple: Option<(u32, u32, u32)> = parse_bd_version(trimmed).map(Into::into);

    if let Some((major, minor, patch)) = tuple {
        let info = (client, major, minor, patch);
        let client_name = match client {
            CliClient::Bd => "bd",
            CliClient::Br => "br",
            CliClient::Unknown => "unknown",
        };
        log_info!("[cli_detect] Detected {} client v{}.{}.{}", client_name, major, minor, patch);
        *cached = Some(info);
        Some(info)
    } else {
        log_warn!("[cli_detect] Could not parse version from: {}", trimmed);
        None
    }
}


pub(crate) fn supports_daemon_flag() -> bool {
    match get_cli_client_info() {
        Some((client, major, minor, patch)) => supports_daemon_flag_for(client, major, minor, patch),
        None => false,
    }
}


pub(crate) fn uses_jsonl_files() -> bool {
    match get_cli_client_info() {
        Some((client, major, minor, patch)) => uses_jsonl_files_for(client, major, minor, patch),
        None => false,
    }
}


pub(crate) fn supports_list_all_flag() -> bool {
    match get_cli_client_info() {
        Some((client, major, minor, patch)) => supports_list_all_flag_for(client, major, minor, patch),
        None => false,
    }
}


pub(crate) fn supports_delete_hard_flag() -> bool {
    match get_cli_client_info() {
        Some((client, major, minor, patch)) => supports_delete_hard_flag_for(client, major, minor, patch),
        None => false,
    }
}


pub(crate) fn uses_dolt_backend() -> bool {
    match get_cli_client_info() {
        Some((client, major, minor, patch)) => uses_dolt_backend_for(client, major, minor, patch),
        None => false,
    }
}

/// Returns true if a specific project uses the Dolt backend.
/// Checks for the presence of `.beads/.dolt/` directory in the project.
/// - br: NEVER (frozen on SQLite+JSONL architecture)
/// - bd < 0.50.0: NEVER (CLI doesn't support Dolt)
/// - bd >= 0.50.0: checks if `.dolt/` directory exists inside the beads dir
pub(crate) fn project_uses_dolt(beads_dir: &std::path::Path) -> bool {
    project_uses_dolt_for(get_cli_client_info(), beads_dir)
}

/// Pure core of `project_uses_dolt`: decides from the (possibly unknown) CLI client
/// info and the on-disk layout of `beads_dir`. Testable without spawning `bd`.
/// - br never uses Dolt; bd < 0.50 never uses Dolt
/// - otherwise: `.beads/.dolt` (legacy layout), or `metadata.json` declaring
///   `"backend":"dolt"` AND a `dolt/<name>/.dolt` directory (bd 0.52+ layout)
pub(crate) fn project_uses_dolt_for(
    info: Option<(CliClient, u32, u32, u32)>,
    beads_dir: &std::path::Path,
) -> bool {
    match info {
        Some((CliClient::Br, _, _, _)) => false,
        Some((CliClient::Bd, major, minor, _)) if major == 0 && minor < 50 => false,
        _ => {
            // Check .beads/.dolt (legacy) or .beads/dolt/<name>/.dolt (bd 0.52+)
            if beads_dir.join(".dolt").is_dir() {
                return true;
            }
            // Check metadata.json for backend: "dolt"
            let metadata_path = beads_dir.join("metadata.json");
            if let Ok(content) = std::fs::read_to_string(&metadata_path) {
                if content.contains("\"backend\":\"dolt\"") || content.contains("\"backend\": \"dolt\"") {
                    // Verify dolt database actually exists
                    let dolt_dir = beads_dir.join("dolt");
                    if dolt_dir.is_dir() {
                        // Check if any subdirectory has .dolt
                        if let Ok(entries) = std::fs::read_dir(&dolt_dir) {
                            for entry in entries.flatten() {
                                if entry.path().join(".dolt").is_dir() {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            false
        }
    }
}

/// Reset the cached client info (called when CLI binary path changes).
pub(crate) fn reset_bd_version_cache() {
    let mut cached = CLI_CLIENT_INFO.lock().unwrap();
    *cached = None;
}

/// `CliInvoker` over the app's process-global client state (`CLI_BINARY`, `CLI_CLIENT_INFO`,
/// `PROJECT_LOCKS`), so `btit_cli::ops` run through the same statics as before.
/// Transitional (b-4); deleted by b-7 when the backend slot replaces the statics.
pub(crate) struct AppInvoker;

impl CliInvoker for AppInvoker {
    fn binary(&self) -> String {
        get_cli_binary()
    }

    fn probe(&self) -> Option<CliProbe> {
        btit_cli::probe::probe_cli_binary(&get_cli_binary())
    }

    fn client_info(&self) -> Option<CliProbe> {
        get_cli_client_info().map(|(client, major, minor, patch)| CliProbe {
            client,
            version: Some((major, minor, patch).into()),
            raw: String::new(),
        })
    }

    fn run_json(&self, project: &ProjectRef, command: &str, args: &[String]) -> Result<String, BeadsError> {
        // The client only labels the error of a non-local project, so no extra probe is spent on it;
        // `supports_daemon_flag()` is the one `get_cli_client_info()` read `execute_bd` made.
        let wd = run::resolve_working_dir(project, CliClient::Unknown)?;
        run::run_json(&get_cli_binary(), supports_daemon_flag(), &PROJECT_LOCKS, &wd, command, args)
    }

    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> {
        let wd = run::resolve_working_dir(project, CliClient::Unknown)?;
        run::run_raw(&get_cli_binary(), &wd, args)
    }
}

/// Transitional wrapper over `btit_cli::run::run_json` through [`AppInvoker`] (b-4); deleted by b-7.
#[expect(dead_code, reason = "b-4 transitional wrapper: every former caller now runs btit_cli::ops through AppInvoker")]
pub(crate) fn execute_bd(command: &str, args: &[String], cwd: Option<&str>) -> Result<String, String> {
    AppInvoker.run_json(&ProjectRef::local(cwd.map(String::from)), command, args).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn check_bd_compatibility() -> CompatibilityInfo {
    let binary = get_cli_binary();
    let probe = btit_cli::probe::probe_cli_binary(&binary);

    let (found, version_string, client, tuple) = match &probe {
        Some(p) => (true, p.raw.clone(), p.client, p.version.map(Into::into)),
        None => (false, format!("{} not found", binary), CliClient::Unknown, None),
    };

    let mut warnings = if found {
        cli_compatibility_warnings(client, tuple.map(Into::into))
    } else {
        vec![format!(
            "{} was not found on PATH or is not executable. Install bd {}.x or point Settings at the binary.",
            binary, MIN_SUPPORTED_BD_MAJOR
        )]
    };
    if found && client == CliClient::Unknown {
        warnings.push(format!("--version output was: {}", version_string));
    }

    // Keep the feature-flag cache coherent with what we just observed.
    if let (Some(p), Some((major, minor, patch))) = (&probe, tuple) {
        *CLI_CLIENT_INFO.lock().unwrap_or_else(|e| e.into_inner()) = Some((p.client, major, minor, patch));
    }

    CompatibilityInfo {
        binary,
        found,
        version: version_string,
        client_type: cli_client_name(client).to_string(),
        version_tuple: tuple.map(|(a, b, c)| vec![a, b, c]),
        legacy: is_legacy_bd(client, tuple.map(Into::into)),
        min_supported_major: MIN_SUPPORTED_BD_MAJOR,
        supports_daemon_flag: supports_daemon_flag(),
        uses_jsonl_files: uses_jsonl_files(),
        uses_dolt_backend: uses_dolt_backend(),
        supports_list_all_flag: supports_list_all_flag(),
        searched_paths: btit_cli::path::extended_path_entries(),
        warnings,
    }
}



// The seven `project_uses_dolt`/`project_uses_dolt_for` tests moved to
// `crates/btit-bd` (b-5): `BdCli::project_uses_dolt` wraps this file's
// `project_uses_dolt_for` unchanged, and `crate::dolt::project_uses_dolt_for` in
// btit-bd is the copy those tests now exercise directly. This function's body stays
// here, untouched, until b-7 deletes the app's copy.
