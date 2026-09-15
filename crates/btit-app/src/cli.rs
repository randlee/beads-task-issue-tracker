use crate::config::get_cli_binary;
use crate::logging::VERBOSE_LOGGING;
use btit_types::{CliClient, CliProbe, CompatibilityInfo};
// Pure detection, compatibility and gate logic lives in btit-beads (b-3); re-exported so
// config.rs, updates.rs, migration.rs, issue_commands.rs, polling.rs and lib.rs keep their paths.
pub(crate) use btit_beads::compat::cli_compatibility_warnings;
pub(crate) use btit_beads::detect::{
    cli_client_name, detect_cli_client, is_legacy_bd, parse_bd_version, parse_cli_probe,
    select_default_binary, CLI_CANDIDATES, CLI_FALLBACK, MIN_SUPPORTED_BD_MAJOR,
};
pub(crate) use btit_beads::gates::{
    supports_daemon_flag_for, supports_delete_hard_flag_for, supports_list_all_flag_for,
    uses_dolt_backend_for, uses_jsonl_files_for,
};
use std::collections::HashMap;
use std::env;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::{LazyLock, Mutex};

// Per-project mutex to prevent concurrent bd/Dolt access.
// bd 0.55 uses embedded Dolt which crashes (SIGSEGV) when two bd processes
// access the same database simultaneously. This serializes all bd calls per project.
pub(crate) static BD_PROJECT_LOCKS: LazyLock<Mutex<HashMap<String, std::sync::Arc<Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// Cached CLI client info — detected once on first use
// Stores: (client_type, major, minor, patch)
pub(crate) static CLI_CLIENT_INFO: LazyLock<Mutex<Option<(CliClient, u32, u32, u32)>>> =
    LazyLock::new(|| Mutex::new(None));

pub(crate) fn get_extended_path() -> String {
    let current_path = env::var("PATH").unwrap_or_default();

    #[cfg(target_os = "windows")]
    {
        let userprofile = env::var("USERPROFILE").unwrap_or_default();
        let localappdata = env::var("LOCALAPPDATA").unwrap_or_default();
        let mut extra_paths = vec![
            format!(r"{}\AppData\Local\bin", userprofile),
            format!(r"{}\.local\bin", userprofile),
            format!(r"{}\Programs", localappdata),
            // `go install` and `cargo install` targets (bd / br)
            format!(r"{}\go\bin", userprofile),
            format!(r"{}\.cargo\bin", userprofile),
        ];
        extra_paths.extend(current_path.split(';').map(String::from));
        extra_paths.join(";")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = env::var("HOME").unwrap_or_default();
        let gopath = env::var("GOPATH").unwrap_or_else(|_| format!("{}/go", home));
        let mut extra_paths = vec![
            "/opt/homebrew/bin".to_string(),
            "/usr/local/bin".to_string(),
            "/home/linuxbrew/.linuxbrew/bin".to_string(),
            "/usr/bin".to_string(),
            "/bin".to_string(),
            format!("{}/.local/bin", home),
            format!("{}/bin", home),
            // `go install` and `cargo install` targets (bd / br)
            format!("{}/bin", gopath),
            format!("{}/.cargo/bin", home),
        ];
        extra_paths.extend(current_path.split(':').map(String::from));
        extra_paths.join(":")
    }
}

/// Creates a Command with platform-specific flags.
/// On Windows, sets CREATE_NO_WINDOW to prevent console popups.
pub(crate) fn new_command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}

/// Run `bin --version` and parse it. `None` when the binary is missing,
/// not executable, or exits non-zero.
/// Runs with the extended PATH so GUI launches (Finder/Dock, minimal PATH)
/// can still resolve Homebrew / Go / Cargo installs, and from the temp dir so
/// bd never auto-migrates a project as a side effect of the probe.
pub(crate) fn probe_cli_binary(bin: &str) -> Option<CliProbe> {
    let output = new_command(bin)
        .arg("--version")
        .current_dir(std::env::temp_dir())
        .env("PATH", get_extended_path())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(parse_cli_probe(&String::from_utf8_lossy(&output.stdout)))
}

/// Directories the CLI probe searches, in order, without duplicates.
/// Uses the platform PATH separator (`;` on Windows, `:` elsewhere).
pub(crate) fn extended_path_entries() -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    std::env::split_paths(&get_extended_path())
        .map(|p| p.to_string_lossy().to_string())
        .filter(|p| !p.is_empty())
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

pub(crate) fn default_cli_binary() -> String {
    let selection = select_default_binary(CLI_CANDIDATES, CLI_FALLBACK, probe_cli_binary);
    match selection.probe() {
        Some(p) => {
            log::info!(
                "[cli_detect] Auto-selected {} ({} {})",
                selection.binary(),
                cli_client_name(p.client),
                p.raw
            );
            if selection.is_legacy() {
                log::warn!(
                    "[cli_detect] {} is below the supported bd {}.x floor; running in legacy mode",
                    p.raw,
                    MIN_SUPPORTED_BD_MAJOR
                );
            }
        }
        None => log::warn!(
            "[cli_detect] No CLI found (tried {}); defaulting to {}. Searched: {}",
            CLI_CANDIDATES.join(", "),
            selection.binary(),
            extended_path_entries().join(if cfg!(windows) { ";" } else { ":" })
        ),
    }
    selection.binary().to_string()
}


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

pub(crate) fn execute_bd(command: &str, args: &[String], cwd: Option<&str>) -> Result<String, String> {
    let working_dir = cwd
        .map(String::from)
        .or_else(|| env::var("BEADS_PATH").ok())
        .unwrap_or_else(|| {
            env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });

    // Split command by spaces to handle subcommands like "comments add"
    let mut full_args: Vec<&str> = command.split_whitespace().collect();
    for arg in args {
        full_args.push(arg);
    }
    if supports_daemon_flag() {
        full_args.push("--no-daemon");
    }
    full_args.push("--json");

    let binary = get_cli_binary();
    log_info!("[bd] {} {} | cwd: {}", binary, full_args.join(" "), working_dir);

    // Acquire per-project lock to prevent concurrent Dolt access (causes SIGSEGV).
    let project_lock = {
        let mut locks = BD_PROJECT_LOCKS.lock().unwrap();
        locks.entry(working_dir.clone())
            .or_insert_with(|| std::sync::Arc::new(Mutex::new(())))
            .clone()
    };
    let _guard = project_lock.lock().unwrap();

    let output = new_command(&binary)
        .args(&full_args)
        .current_dir(&working_dir)
        .env("PATH", get_extended_path())
        .env("BEADS_PATH", &working_dir)
        .output()
        .map_err(|e| {
            log_error!("[bd] Failed to execute {}: {}", binary, e);
            format!("Failed to execute {}: {}", binary, e)
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log_error!("[bd] Command failed | status: {} | stderr: {}", output.status, stderr);

        // Detect schema migration failure (bd 0.49.4 migration bug)
        if stderr.contains("no such column: spec_id") {
            log_error!("[bd] Schema migration failure detected - database needs repair");
            return Err("SCHEMA_MIGRATION_ERROR: Database schema is incompatible. Please use the repair function to fix this issue.".to_string());
        }

        if !stderr.is_empty() {
            return Err(stderr.to_string());
        }
        return Err(format!("bd command failed with status: {}", output.status));
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

#[tauri::command]
pub(crate) async fn check_bd_compatibility() -> CompatibilityInfo {
    let binary = get_cli_binary();
    let probe = probe_cli_binary(&binary);

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
        searched_paths: extended_path_entries(),
        warnings,
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    // ---- CLI auto-detection -------------------------------------------------

    #[test]
    fn probe_returns_none_for_nonexistent_binary() {
        assert!(probe_cli_binary("definitely-not-a-real-cli-binary-xyz").is_none());
    }


    #[test]
    fn extended_path_entries_are_nonempty_and_deduplicated() {
        let entries = extended_path_entries();
        assert!(!entries.is_empty());
        assert!(entries.iter().all(|e| !e.is_empty()));
        let mut sorted = entries.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), entries.len(), "duplicates present: {entries:?}");
        // Split on the platform separator, not a hardcoded one
        let sep = if cfg!(windows) { ';' } else { ':' };
        assert!(entries.iter().all(|e| !e.contains(sep)), "entry contains separator: {entries:?}");
    }

    #[test]
    fn extended_path_includes_gui_launch_locations() {
        let path = get_extended_path();
        #[cfg(not(target_os = "windows"))]
        {
            for needle in ["/opt/homebrew/bin", "/usr/local/bin", "/.cargo/bin", "/.local/bin"] {
                assert!(path.contains(needle), "extended PATH missing {needle}: {path}");
            }
            assert!(path.contains("go/bin") || env::var("GOPATH").is_ok(), "extended PATH missing go bin dir: {path}");
            let ambient = env::var("PATH").unwrap_or_default();
            assert!(ambient.is_empty() || path.ends_with(&ambient));
        }
        #[cfg(target_os = "windows")]
        {
            for needle in [r"\go\bin", r"\.cargo\bin", r"\.local\bin"] {
                assert!(path.contains(needle), "extended PATH missing {needle}: {path}");
            }
        }
    }

    #[test]
    fn extended_path_entries_contain_platform_install_dirs_in_order() {
        let entries = extended_path_entries();
        assert!(entries.iter().all(|e| !e.is_empty()), "empty entry: {entries:?}");

        #[cfg(not(target_os = "windows"))]
        {
            let idx = |needle: &str| {
                entries
                    .iter()
                    .position(|e| e == needle)
                    .unwrap_or_else(|| panic!("missing {needle}: {entries:?}"))
            };
            let homebrew = idx("/opt/homebrew/bin");
            let usr_local = idx("/usr/local/bin");
            let usr_bin = idx("/usr/bin");
            assert!(homebrew < usr_local, "order not preserved: {entries:?}");
            assert!(usr_local < usr_bin, "order not preserved: {entries:?}");
            assert!(entries.iter().any(|e| e.ends_with("/.cargo/bin")), "{entries:?}");
            assert!(
                entries.iter().any(|e| e.ends_with("/bin") && (e.contains("/go/") || env::var("GOPATH").is_ok())),
                "missing go bin dir: {entries:?}"
            );
        }
        #[cfg(target_os = "windows")]
        {
            assert!(entries.iter().any(|e| e.ends_with(r"\go\bin")), "{entries:?}");
            assert!(entries.iter().any(|e| e.ends_with(r"\.cargo\bin")), "{entries:?}");
            assert!(entries.iter().any(|e| e.ends_with(r"\.local\bin")), "{entries:?}");
            let go = entries.iter().position(|e| e.ends_with(r"\go\bin")).unwrap();
            let cargo = entries.iter().position(|e| e.ends_with(r"\.cargo\bin")).unwrap();
            assert!(go < cargo, "order not preserved: {entries:?}");
        }
    }

    #[test]
    fn extended_path_entries_precede_ambient_path() {
        // The extra install dirs must come before whatever PATH the process
        // inherited, so a GUI launch with a minimal PATH still finds bd.
        let entries = extended_path_entries();
        #[cfg(not(target_os = "windows"))]
        assert_eq!(entries.first().map(String::as_str), Some("/opt/homebrew/bin"));
        #[cfg(target_os = "windows")]
        assert!(entries.first().map(|e| e.ends_with(r"\AppData\Local\bin")).unwrap_or(false), "{entries:?}");
    }


    #[test]
    fn probe_returns_none_for_nonexistent_relative_path() {
        // A candidate containing a path separator bypasses PATH lookup entirely,
        // so this is None on every platform regardless of what is installed.
        assert!(probe_cli_binary("./definitely/not/here").is_none());
        #[cfg(target_os = "windows")]
        assert!(probe_cli_binary(r".\definitely\not\here.exe").is_none());
        #[cfg(not(target_os = "windows"))]
        assert!(probe_cli_binary("/definitely/not/here").is_none());
    }

    #[test]
    fn probe_returns_none_for_empty_binary_name() {
        assert!(probe_cli_binary("").is_none());
    }
    // ---- Filesystem-local helpers (#6) ------------------------------------------

    #[test]
    fn project_uses_dolt_false_without_beads_dir() {
        let temp_dir = std::env::temp_dir().join(format!("beads_test_no_beads_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let result = project_uses_dolt(&temp_dir);
        assert!(!result);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    fn dolt_tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "beads_dolt_{}_{}",
            name,
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const BD_1: Option<(CliClient, u32, u32, u32)> = Some((CliClient::Bd, 1, 0, 4));

    #[test]
    fn project_uses_dolt_for_legacy_dolt_dir() {
        let beads = dolt_tmp("legacy");
        std::fs::create_dir_all(beads.join(".dolt")).unwrap();
        assert!(project_uses_dolt_for(BD_1, &beads));
        // Unknown client info falls through to the layout check
        assert!(project_uses_dolt_for(None, &beads));
        let _ = std::fs::remove_dir_all(&beads);
    }

    #[test]
    fn project_uses_dolt_for_nested_layout_needs_metadata_and_dolt_dir() {
        let beads = dolt_tmp("nested");
        std::fs::write(beads.join("metadata.json"), r#"{"backend": "dolt"}"#).unwrap();
        // metadata says dolt but no dolt/ directory yet
        assert!(!project_uses_dolt_for(BD_1, &beads));
        std::fs::create_dir_all(beads.join("dolt").join("proj").join(".dolt")).unwrap();
        assert!(project_uses_dolt_for(BD_1, &beads));
        let _ = std::fs::remove_dir_all(&beads);
    }

    #[test]
    fn project_uses_dolt_for_sqlite_metadata_or_empty_dir_is_false() {
        let beads = dolt_tmp("sqlite");
        assert!(!project_uses_dolt_for(BD_1, &beads));
        std::fs::write(beads.join("metadata.json"), r#"{"backend":"sqlite"}"#).unwrap();
        assert!(!project_uses_dolt_for(BD_1, &beads));
        std::fs::write(beads.join("metadata.json"), "not json").unwrap();
        assert!(!project_uses_dolt_for(BD_1, &beads));
        let _ = std::fs::remove_dir_all(&beads);
    }

    #[test]
    fn project_uses_dolt_for_br_and_legacy_bd_never_true() {
        let beads = dolt_tmp("never");
        std::fs::create_dir_all(beads.join(".dolt")).unwrap();
        assert!(!project_uses_dolt_for(Some((CliClient::Br, 0, 1, 33)), &beads));
        assert!(!project_uses_dolt_for(Some((CliClient::Bd, 0, 49, 6)), &beads));
        assert!(project_uses_dolt_for(Some((CliClient::Bd, 0, 50, 0)), &beads));
        let _ = std::fs::remove_dir_all(&beads);
    }

}
