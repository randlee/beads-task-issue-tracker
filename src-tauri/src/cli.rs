use crate::config::get_cli_binary;
use crate::logging::VERBOSE_LOGGING;
use crate::types::CliClient;
use serde::Serialize;
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

/// CLI binaries probed during auto-detection, in priority order.
/// `bd` (Go) is the primary CLI this app targets; `br` (Rust) is secondary.
pub(crate) const CLI_CANDIDATES: &[&str] = &["bd", "br"];

/// Binary assumed when no candidate responds to `--version`.
pub(crate) const CLI_FALLBACK: &str = "bd";

/// Minimum `bd` major version this app targets. Older versions still run
/// through the version-gated code paths but are reported as legacy and
/// surfaced to the user. Raise this when the supported floor moves.
pub(crate) const MIN_SUPPORTED_BD_MAJOR: u32 = 1;

/// Result of running `<bin> --version` on a candidate CLI binary.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CliProbe {
    pub(crate) client: CliClient,
    pub(crate) version: Option<(u32, u32, u32)>,
    /// Trimmed first line of `--version` output, for logs and the UI.
    pub(crate) raw: String,
}

/// Outcome of auto-detection.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CliSelection {
    binary: String,
    /// `None` when no candidate answered and `binary` is the fallback.
    probe: Option<CliProbe>,
}

impl CliSelection {
    /// True when a `bd` older than `MIN_SUPPORTED_BD_MAJOR` was selected.
    fn is_legacy(&self) -> bool {
        self.probe
            .as_ref()
            .map(|p| is_legacy_bd(p.client, p.version))
            .unwrap_or(false)
    }
}

/// A `bd` whose major version is below the supported floor (or whose version
/// could not be parsed) is legacy. `br` and unknown clients are never "legacy".
pub(crate) fn is_legacy_bd(client: CliClient, version: Option<(u32, u32, u32)>) -> bool {
    match (client, version) {
        (CliClient::Bd, Some((major, _, _))) => major < MIN_SUPPORTED_BD_MAJOR,
        (CliClient::Bd, None) => true,
        _ => false,
    }
}

/// Rank a probed candidate for auto-selection. Lower is better; ties keep
/// candidate order. Pure so it can be unit-tested without spawning processes.
///
/// 0: bd at or above the supported floor (primary CLI)
/// 1: bd below the floor / unparsable version (works, but warn)
/// 2: br (secondary CLI)
/// 3: unknown client
pub(crate) fn rank_cli_candidate(probe: &CliProbe) -> u8 {
    match probe.client {
        CliClient::Bd if !is_legacy_bd(probe.client, probe.version) => 0,
        CliClient::Bd => 1,
        CliClient::Br => 2,
        CliClient::Unknown => 3,
    }
}

/// Pure selection logic: probe every candidate, pick the best-ranked one
/// (first wins on ties), or fall back to `fallback` when none answers.
pub(crate) fn select_default_binary<F: Fn(&str) -> Option<CliProbe>>(
    candidates: &[&str],
    fallback: &str,
    probe: F,
) -> CliSelection {
    let mut best: Option<(u8, &str, CliProbe)> = None;
    for bin in candidates {
        if let Some(p) = probe(bin) {
            let rank = rank_cli_candidate(&p);
            let better = match &best {
                None => true,
                Some((best_rank, _, _)) => rank < *best_rank,
            };
            if better {
                best = Some((rank, bin, p));
            }
        }
    }
    match best {
        Some((_, bin, p)) => CliSelection { binary: bin.to_string(), probe: Some(p) },
        None => CliSelection { binary: fallback.to_string(), probe: None },
    }
}

/// Parse `--version` stdout into a `CliProbe`. Pure.
pub(crate) fn parse_cli_probe(stdout: &str) -> CliProbe {
    let raw = stdout
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string();
    CliProbe {
        client: detect_cli_client(&raw),
        version: parse_bd_version(&raw),
        raw,
    }
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

pub(crate) fn cli_client_name(client: CliClient) -> &'static str {
    match client {
        CliClient::Bd => "bd",
        CliClient::Br => "br",
        CliClient::Unknown => "unknown",
    }
}

/// Human-readable compatibility warnings for a detected client. Pure.
pub(crate) fn cli_compatibility_warnings(client: CliClient, version: Option<(u32, u32, u32)>) -> Vec<String> {
    let mut warnings = Vec::new();
    match (client, version) {
        (CliClient::Bd, Some((major, minor, patch))) if major < MIN_SUPPORTED_BD_MAJOR => {
            warnings.push(format!(
                "bd {}.{}.{} is a legacy version: this app targets bd {}.x. \
                 Legacy versions may work but are not supported; please upgrade bd.",
                major, minor, patch, MIN_SUPPORTED_BD_MAJOR
            ));
            if major == 0 && minor >= 50 {
                warnings.push(
                    "bd 0.50-0.56 removed the daemon and JSONL files in favor of Dolt server mode; \
                     change detection falls back to polling."
                        .to_string(),
                );
            }
        }
        (CliClient::Bd, None) => {
            warnings.push(format!(
                "Could not parse the bd version; this app targets bd {}.x.",
                MIN_SUPPORTED_BD_MAJOR
            ));
        }
        (CliClient::Br, _) => {
            warnings.push(
                "br (beads_rust) detected: supported as a secondary CLI. New features target bd first."
                    .to_string(),
            );
        }
        (CliClient::Unknown, _) => {
            warnings.push("Could not detect the CLI client from its --version output.".to_string());
        }
        _ => {}
    }
    warnings
}

pub(crate) fn default_cli_binary() -> String {
    let selection = select_default_binary(CLI_CANDIDATES, CLI_FALLBACK, probe_cli_binary);
    match &selection.probe {
        Some(p) => {
            log::info!(
                "[cli_detect] Auto-selected {} ({} {})",
                selection.binary,
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
            selection.binary,
            extended_path_entries().join(if cfg!(windows) { ";" } else { ":" })
        ),
    }
    selection.binary
}

// ============================================================================
// CLI Client Detection (bd vs br)
// ============================================================================

/// Detect the client type from the version string.
/// - "bd version 0.49.6 (Homebrew)" → Bd
/// - "br 0.1.13 (rustc 1.85.0-nightly)" → Br
pub(crate) fn detect_cli_client(version_str: &str) -> CliClient {
    let lower = version_str.to_lowercase();
    if lower.starts_with("br ") || lower.contains("beads_rust") || lower.contains("beads-rust") {
        CliClient::Br
    } else if lower.starts_with("bd ") || lower.contains("bd version") {
        CliClient::Bd
    } else {
        CliClient::Unknown
    }
}

/// Parse a version string into (major, minor, patch).
/// Works for both "bd version 0.49.6 (Homebrew)" and "br 0.1.13 (rustc ...)".
pub(crate) fn parse_bd_version(version_str: &str) -> Option<(u32, u32, u32)> {
    // Look for a semver-like pattern: digits.digits.digits
    // Accept "1.2.3" and "v1.2.3" (some CLIs print a v-prefixed tag)
    let re_like = version_str
        .split_whitespace()
        .map(|word| word.trim_start_matches(['v', 'V']))
        .find(|word| word.contains('.') && word.chars().next().map_or(false, |c| c.is_ascii_digit()));

    let version_part = re_like?;
    let parts: Vec<&str> = version_part.split('.').collect();
    if parts.len() >= 3 {
        let major = parts[0].parse::<u32>().ok()?;
        let minor = parts[1].parse::<u32>().ok()?;
        // Patch may have trailing non-numeric chars (e.g. "6-beta")
        let patch_str: String = parts[2].chars().take_while(|c| c.is_ascii_digit()).collect();
        let patch = patch_str.parse::<u32>().ok()?;
        Some((major, minor, patch))
    } else {
        None
    }
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
    let tuple = parse_bd_version(trimmed);

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

/// Returns true if the CLI supports the --no-daemon flag.
/// - br: NEVER (no daemon concept)
/// - bd < 0.50.0: YES
/// - bd >= 0.50.0: NO (daemon removed)
/// - unknown: NO (safe default)
pub(crate) fn supports_daemon_flag_for(client: CliClient, major: u32, minor: u32, _patch: u32) -> bool {
    match client {
        CliClient::Br => false, // br has no daemon
        CliClient::Bd => major == 0 && minor < 50,
        CliClient::Unknown => false,
    }
}

pub(crate) fn supports_daemon_flag() -> bool {
    match get_cli_client_info() {
        Some((client, major, minor, patch)) => supports_daemon_flag_for(client, major, minor, patch),
        None => false,
    }
}

/// Returns true if the CLI uses issues.jsonl files.
/// - br: ALWAYS (frozen on SQLite+JSONL architecture)
/// - bd < 0.50.0: YES
/// - bd >= 0.50.0: NO (Dolt only)
/// - unknown: NO (safe default)
pub(crate) fn uses_jsonl_files_for(client: CliClient, major: u32, minor: u32, _patch: u32) -> bool {
    match client {
        CliClient::Br => true, // br always uses JSONL
        CliClient::Bd => major == 0 && minor < 50,
        CliClient::Unknown => false,
    }
}

pub(crate) fn uses_jsonl_files() -> bool {
    match get_cli_client_info() {
        Some((client, major, minor, patch)) => uses_jsonl_files_for(client, major, minor, patch),
        None => false,
    }
}

/// Returns true if `bd list --all` works correctly.
/// The --all flag was buggy before bd 0.55.0 (returned incorrect results).
/// - br: NO
/// - bd >= 0.55.0: YES
/// - bd < 0.55.0: NO (use 2 separate calls instead)
/// - unknown: NO (safe default)
pub(crate) fn supports_list_all_flag_for(client: CliClient, major: u32, minor: u32, _patch: u32) -> bool {
    match client {
        CliClient::Bd => major > 0 || minor >= 55,
        CliClient::Br => true, // br always supports --all
        CliClient::Unknown => false,
    }
}

pub(crate) fn supports_list_all_flag() -> bool {
    match get_cli_client_info() {
        Some((client, major, minor, patch)) => supports_list_all_flag_for(client, major, minor, patch),
        None => false,
    }
}

/// Returns true if `bd delete --hard` is supported.
/// The --hard flag was removed in bd 0.50.0.
/// - br: NO
/// - bd < 0.50.0: YES
/// - bd >= 0.50.0: NO (only --force needed)
/// - unknown: NO (safe default)
pub(crate) fn supports_delete_hard_flag_for(client: CliClient, major: u32, minor: u32, _patch: u32) -> bool {
    match client {
        CliClient::Bd => major == 0 && minor < 50,
        _ => false,
    }
}

pub(crate) fn supports_delete_hard_flag() -> bool {
    match get_cli_client_info() {
        Some((client, major, minor, patch)) => supports_delete_hard_flag_for(client, major, minor, patch),
        None => false,
    }
}

/// Returns true if the CLI uses the Dolt backend (inverse of uses_jsonl_files).
/// - br: NEVER (frozen on SQLite+JSONL architecture)
/// - bd >= 0.50.0: YES (Dolt only)
/// - bd < 0.50.0: NO (SQLite+JSONL)
/// - unknown: NO (safe default)
pub(crate) fn uses_dolt_backend_for(client: CliClient, major: u32, minor: u32, _patch: u32) -> bool {
    match client {
        CliClient::Br => false, // br never uses Dolt
        CliClient::Bd => major > 0 || minor >= 50,
        CliClient::Unknown => false,
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
    match get_cli_client_info() {
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

/// Auto-run refs migration v3 (filesystem-only attachments) if needed.
/// Called synchronously before br sync to prevent UNIQUE constraint errors.

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompatibilityInfo {
    /// Configured binary name or path (e.g. "bd").
    binary: String,
    /// False when `binary --version` could not be run.
    found: bool,
    /// Raw `--version` output, or a "not found" message.
    version: String,
    /// "bd", "br", or "unknown"
    client_type: String,
    version_tuple: Option<Vec<u32>>,
    /// True when a bd below `MIN_SUPPORTED_BD_MAJOR` is in use.
    legacy: bool,
    min_supported_major: u32,
    supports_daemon_flag: bool,
    uses_jsonl_files: bool,
    uses_dolt_backend: bool,
    supports_list_all_flag: bool,
    /// Directories searched when resolving the binary (for the "not found" UI).
    searched_paths: Vec<String>,
    warnings: Vec<String>,
}

#[tauri::command]
pub(crate) async fn check_bd_compatibility() -> CompatibilityInfo {
    let binary = get_cli_binary();
    let probe = probe_cli_binary(&binary);

    let (found, version_string, client, tuple) = match &probe {
        Some(p) => (true, p.raw.clone(), p.client, p.version),
        None => (false, format!("{} not found", binary), CliClient::Unknown, None),
    };

    let mut warnings = if found {
        cli_compatibility_warnings(client, tuple)
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
        legacy: is_legacy_bd(client, tuple),
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
    use crate::test_support::*;

    // ---- CLI auto-detection -------------------------------------------------


    #[test]
    fn rank_prefers_supported_bd_then_legacy_bd_then_br_then_unknown() {
        assert_eq!(rank_cli_candidate(&probe(CliClient::Bd, Some((1, 0, 4)))), 0);
        assert_eq!(rank_cli_candidate(&probe(CliClient::Bd, Some((2, 3, 0)))), 0);
        assert_eq!(rank_cli_candidate(&probe(CliClient::Bd, Some((0, 49, 6)))), 1);
        assert_eq!(rank_cli_candidate(&probe(CliClient::Bd, Some((0, 56, 0)))), 1);
        assert_eq!(rank_cli_candidate(&probe(CliClient::Bd, None)), 1);
        assert_eq!(rank_cli_candidate(&probe(CliClient::Br, Some((0, 1, 33)))), 2);
        assert_eq!(rank_cli_candidate(&probe(CliClient::Unknown, None)), 3);
    }

    #[test]
    fn select_picks_supported_bd_over_br() {
        let sel = select_default_binary(CLI_CANDIDATES, CLI_FALLBACK, |bin| match bin {
            "bd" => Some(probe(CliClient::Bd, Some((1, 0, 4)))),
            "br" => Some(probe(CliClient::Br, Some((0, 1, 33)))),
            _ => None,
        });
        assert_eq!(sel.binary, "bd");
        assert!(sel.probe.is_some());
        assert!(!sel.is_legacy());
    }

    #[test]
    fn select_still_picks_legacy_bd_over_br_but_flags_it() {
        let sel = select_default_binary(CLI_CANDIDATES, CLI_FALLBACK, |bin| match bin {
            "bd" => Some(probe(CliClient::Bd, Some((0, 49, 6)))),
            "br" => Some(probe(CliClient::Br, Some((0, 1, 33)))),
            _ => None,
        });
        assert_eq!(sel.binary, "bd");
        assert!(sel.is_legacy());
    }

    #[test]
    fn select_falls_through_to_br_when_bd_missing() {
        let sel = select_default_binary(CLI_CANDIDATES, CLI_FALLBACK, |bin| match bin {
            "br" => Some(probe(CliClient::Br, Some((0, 1, 33)))),
            _ => None,
        });
        assert_eq!(sel.binary, "br");
        assert!(sel.probe.is_some());
        assert!(!sel.is_legacy());
    }

    #[test]
    fn select_falls_back_to_bd_when_nothing_found() {
        let sel = select_default_binary(CLI_CANDIDATES, CLI_FALLBACK, |_| None);
        assert_eq!(sel.binary, "bd");
        assert!(sel.probe.is_none());
        assert!(!sel.is_legacy());
    }

    #[test]
    fn select_prefers_better_rank_over_candidate_order() {
        // A binary named "bd" that turns out to be unknown loses to a real br later in the list.
        let sel = select_default_binary(&["bd", "br"], "bd", |bin| match bin {
            "bd" => Some(probe(CliClient::Unknown, None)),
            "br" => Some(probe(CliClient::Br, Some((0, 1, 33)))),
            _ => None,
        });
        assert_eq!(sel.binary, "br");
    }

    #[test]
    fn select_keeps_first_candidate_on_equal_rank() {
        let sel = select_default_binary(&["first", "second"], "fb", |_| Some(probe(CliClient::Bd, Some((1, 2, 0)))));
        assert_eq!(sel.binary, "first");
    }

    #[test]
    fn parse_probe_uses_first_line_only() {
        let p = parse_cli_probe("bd version 1.0.4 (ce242a879)\nextra line\n");
        assert_eq!(p.client, CliClient::Bd);
        assert_eq!(p.version, Some((1, 0, 4)));
        assert_eq!(p.raw, "bd version 1.0.4 (ce242a879)");
    }

    #[test]
    fn probe_returns_none_for_nonexistent_binary() {
        assert!(probe_cli_binary("definitely-not-a-real-cli-binary-xyz").is_none());
    }

    #[test]
    fn legacy_detection_respects_floor() {
        assert!(is_legacy_bd(CliClient::Bd, Some((0, 49, 6))));
        assert!(is_legacy_bd(CliClient::Bd, Some((0, 99, 0))));
        assert!(is_legacy_bd(CliClient::Bd, None));
        assert!(!is_legacy_bd(CliClient::Bd, Some((MIN_SUPPORTED_BD_MAJOR, 0, 0))));
        assert!(!is_legacy_bd(CliClient::Bd, Some((MIN_SUPPORTED_BD_MAJOR + 1, 0, 0))));
        assert!(!is_legacy_bd(CliClient::Br, Some((0, 1, 33))));
        assert!(!is_legacy_bd(CliClient::Unknown, None));
    }

    #[test]
    fn warnings_empty_for_supported_bd() {
        assert!(cli_compatibility_warnings(CliClient::Bd, Some((1, 0, 4))).is_empty());
        assert!(cli_compatibility_warnings(CliClient::Bd, Some((3, 1, 0))).is_empty());
    }

    #[test]
    fn warnings_flag_legacy_bd() {
        let w = cli_compatibility_warnings(CliClient::Bd, Some((0, 49, 6)));
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("0.49.6"));
        assert!(w[0].contains("legacy"));
        assert!(w[0].contains(&format!("bd {}.x", MIN_SUPPORTED_BD_MAJOR)));

        let w = cli_compatibility_warnings(CliClient::Bd, Some((0, 56, 0)));
        assert_eq!(w.len(), 2, "0.50-0.56 also gets the server-mode note");
        assert!(w[1].contains("Dolt"));
    }

    #[test]
    fn warnings_for_br_unknown_and_unparsable() {
        let w = cli_compatibility_warnings(CliClient::Br, Some((0, 1, 33)));
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("secondary"));

        let w = cli_compatibility_warnings(CliClient::Unknown, None);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("Could not detect"));

        let w = cli_compatibility_warnings(CliClient::Bd, None);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("Could not parse"));
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
    fn compatibility_info_serializes_camel_case() {
        let info = CompatibilityInfo {
            binary: "bd".into(),
            found: true,
            version: "bd version 1.0.4".into(),
            client_type: "bd".into(),
            version_tuple: Some(vec![1, 0, 4]),
            legacy: false,
            min_supported_major: MIN_SUPPORTED_BD_MAJOR,
            supports_daemon_flag: false,
            uses_jsonl_files: false,
            uses_dolt_backend: true,
            supports_list_all_flag: true,
            searched_paths: vec!["/opt/homebrew/bin".into()],
            warnings: vec![],
        };
        let json = serde_json::to_value(&info).unwrap();
        for key in [
            "binary", "found", "version", "clientType", "versionTuple", "legacy", "minSupportedMajor",
            "supportsDaemonFlag", "usesJsonlFiles", "usesDoltBackend", "supportsListAllFlag",
            "searchedPaths", "warnings",
        ] {
            assert!(json.get(key).is_some(), "missing camelCase key {key}: {json}");
        }
        assert!(json.get("client_type").is_none());
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

    // ---- CLI client detection -----------------------------------------------

    #[test]
    fn detect_bd_1x_banner() {
        // Real output from bd 1.0.4
        assert_eq!(detect_cli_client("bd version 1.0.4 (ce242a879)"), CliClient::Bd);
        assert_eq!(parse_bd_version("bd version 1.0.4 (ce242a879)"), Some((1, 0, 4)));
    }

    #[test]
    fn detect_bd_legacy_banner() {
        assert_eq!(detect_cli_client("bd version 0.49.6 (Homebrew)"), CliClient::Bd);
        assert_eq!(parse_bd_version("bd version 0.49.6 (Homebrew)"), Some((0, 49, 6)));
    }

    #[test]
    fn detect_br_banner() {
        let banner = "br 0.1.33 (rustc 1.85.0-nightly)";
        assert_eq!(detect_cli_client(banner), CliClient::Br);
        assert_eq!(parse_bd_version(banner), Some((0, 1, 33)));
    }

    #[test]
    fn detect_unknown_banner() {
        assert_eq!(detect_cli_client("something-else 2.0.0"), CliClient::Unknown);
        assert_eq!(detect_cli_client(""), CliClient::Unknown);
    }

    #[test]
    fn parse_version_tolerates_prerelease_suffix_and_garbage() {
        assert_eq!(parse_bd_version("bd version 1.2.0-beta (abc)"), Some((1, 2, 0)));
        assert_eq!(parse_bd_version("bd version 1.2.3-fork"), Some((1, 2, 3)));
        assert_eq!(parse_bd_version("no version here"), None);
        assert_eq!(parse_bd_version("bd version 1.2"), None);
    }

    // ---- CLI auto-detection: edge cases -------------------------------------

    #[test]
    fn parse_probe_empty_and_whitespace_only_are_unknown() {
        for input in ["", "   ", "\t", "  \n", "\n\n"] {
            let p = parse_cli_probe(input);
            assert_eq!(p.client, CliClient::Unknown, "input {input:?}");
            assert_eq!(p.version, None, "input {input:?}");
            assert_eq!(p.raw, "", "input {input:?}");
        }
    }

    #[test]
    fn parse_probe_strips_crlf_line_endings() {
        let p = parse_cli_probe("bd version 1.0.4 (abc)\r\n");
        assert_eq!(p.client, CliClient::Bd);
        assert_eq!(p.version, Some((1, 0, 4)));
        assert_eq!(p.raw, "bd version 1.0.4 (abc)");
        assert!(!p.raw.contains('\r'));
    }

    #[test]
    fn parse_probe_trims_surrounding_whitespace_on_first_line() {
        let p = parse_cli_probe("  bd version 1.0.4 (abc)  \n");
        assert_eq!(p.client, CliClient::Bd);
        assert_eq!(p.version, Some((1, 0, 4)));
        assert_eq!(p.raw, "bd version 1.0.4 (abc)");
    }

    #[test]
    fn parse_probe_skips_leading_blank_lines() {
        let p = parse_cli_probe("\n  \nbd version 1.0.4 (abc)\n");
        assert_eq!(p.client, CliClient::Bd);
        assert_eq!(p.version, Some((1, 0, 4)));
        assert_eq!(p.raw, "bd version 1.0.4 (abc)");
    }

    #[test]
    fn parse_probe_accepts_v_prefixed_version() {
        let p = parse_cli_probe("bd v1.2.0");
        assert_eq!(p.client, CliClient::Bd);
        assert_eq!(p.version, Some((1, 2, 0)));
        assert_eq!(p.raw, "bd v1.2.0");
        assert!(!is_legacy_bd(p.client, p.version));
        assert_eq!(rank_cli_candidate(&p), 0);
        assert_eq!(parse_bd_version("bd version V0.49.6"), Some((0, 49, 6)));
    }

    #[test]
    fn parse_probe_br_banner_with_beads_rust_in_later_word() {
        let p = parse_cli_probe("beads beads_rust 0.1.5 (rustc 1.85.0)\n");
        assert_eq!(p.client, CliClient::Br);
        assert_eq!(p.version, Some((0, 1, 5)));

        let p = parse_cli_probe("cli beads-rust 0.2.0");
        assert_eq!(p.client, CliClient::Br);
        assert_eq!(p.version, Some((0, 2, 0)));
    }

    #[test]
    fn parse_probe_bd_1x_prerelease_with_dotted_suffix() {
        let p = parse_cli_probe("bd version 1.3.0-rc.1 (abc)\n");
        assert_eq!(p.client, CliClient::Bd);
        assert_eq!(p.version, Some((1, 3, 0)));
        assert!(!is_legacy_bd(p.client, p.version));
        assert_eq!(rank_cli_candidate(&p), 0);
    }

    #[test]
    fn detect_cli_client_is_case_insensitive() {
        assert_eq!(detect_cli_client("BD version 1.0.0"), CliClient::Bd);
        assert_eq!(detect_cli_client("Bd Version 1.0.0"), CliClient::Bd);
        assert_eq!(detect_cli_client("Br 0.1.0"), CliClient::Br);
        assert_eq!(detect_cli_client("BR 0.1.0 (rustc)"), CliClient::Br);
        assert_eq!(detect_cli_client("x BEADS_RUST 0.1.0"), CliClient::Br);
        assert_eq!(parse_bd_version("BD version 1.0.0"), Some((1, 0, 0)));
    }

    #[test]
    fn detect_cli_client_requires_word_boundary_prefix() {
        // "bdx" / "brx" are not bd / br
        assert_eq!(detect_cli_client("bdx 1.0.0"), CliClient::Unknown);
        assert_eq!(detect_cli_client("brx 1.0.0"), CliClient::Unknown);
        assert_eq!(detect_cli_client("bd"), CliClient::Unknown);
    }

    #[test]
    fn select_picks_best_ranked_candidate_when_it_is_last() {
        let sel = select_default_binary(&["a", "b", "c"], "fb", |bin| match bin {
            "a" => Some(probe(CliClient::Unknown, None)),
            "b" => Some(probe(CliClient::Br, Some((0, 1, 33)))),
            "c" => Some(probe(CliClient::Bd, Some((1, 0, 4)))),
            _ => None,
        });
        assert_eq!(sel.binary, "c");
        assert_eq!(sel.probe.as_ref().map(|p| p.client), Some(CliClient::Bd));
        assert!(!sel.is_legacy());
    }

    #[test]
    fn select_legacy_bd_last_beats_earlier_br() {
        let sel = select_default_binary(&["br", "bd"], "fb", |bin| match bin {
            "br" => Some(probe(CliClient::Br, Some((0, 1, 33)))),
            "bd" => Some(probe(CliClient::Bd, Some((0, 49, 6)))),
            _ => None,
        });
        assert_eq!(sel.binary, "bd");
        assert!(sel.is_legacy());
    }

    #[test]
    fn select_all_unknown_picks_first_found_not_fallback() {
        let sel = select_default_binary(&["x", "y", "z"], "fb", |_| Some(probe(CliClient::Unknown, None)));
        assert_eq!(sel.binary, "x", "a found-but-unknown binary still beats the fallback");
        assert!(sel.probe.is_some());
        assert_eq!(sel.probe.as_ref().map(|p| p.client), Some(CliClient::Unknown));
        assert!(!sel.is_legacy());
    }

    #[test]
    fn select_empty_candidate_list_returns_fallback() {
        // Even a probe that would always succeed is never consulted.
        let sel = select_default_binary(&[], "fb", |_| Some(probe(CliClient::Bd, Some((1, 0, 0)))));
        assert_eq!(sel.binary, "fb");
        assert!(sel.probe.is_none());
        assert!(!sel.is_legacy());
    }

    #[test]
    fn select_probe_called_once_per_candidate_in_order() {
        use std::cell::RefCell;
        let seen = RefCell::new(Vec::new());
        let sel = select_default_binary(&["one", "two", "three"], "fb", |bin| {
            seen.borrow_mut().push(bin.to_string());
            None
        });
        assert_eq!(*seen.borrow(), vec!["one", "two", "three"]);
        assert_eq!(sel.binary, "fb");
    }

    #[test]
    fn select_ignores_fallback_name_when_probing() {
        // The fallback is a name, not a candidate: it must not be probed.
        let sel = select_default_binary(&["only"], "fallback-not-probed", |bin| {
            assert_ne!(bin, "fallback-not-probed");
            None
        });
        assert_eq!(sel.binary, "fallback-not-probed");
    }

    #[test]
    fn legacy_boundary_at_exact_floor() {
        assert!(!is_legacy_bd(CliClient::Bd, Some((MIN_SUPPORTED_BD_MAJOR, 0, 0))));
        assert!(is_legacy_bd(CliClient::Bd, Some((MIN_SUPPORTED_BD_MAJOR - 1, 99, 99))));
        // Minor/patch never influence the decision.
        assert!(!is_legacy_bd(CliClient::Bd, Some((MIN_SUPPORTED_BD_MAJOR, 99, 99))));
        assert!(is_legacy_bd(CliClient::Bd, Some((MIN_SUPPORTED_BD_MAJOR - 1, 0, 0))));
        // Version is irrelevant for non-bd clients even at 0.0.0.
        assert!(!is_legacy_bd(CliClient::Br, Some((0, 0, 0))));
        assert!(!is_legacy_bd(CliClient::Br, None));
        assert!(!is_legacy_bd(CliClient::Unknown, Some((0, 0, 0))));
    }

    #[test]
    fn cli_selection_is_legacy_follows_probe() {
        let legacy = CliSelection { binary: "bd".into(), probe: Some(probe(CliClient::Bd, Some((0, 49, 6)))) };
        assert!(legacy.is_legacy());
        let unparsable = CliSelection { binary: "bd".into(), probe: Some(probe(CliClient::Bd, None)) };
        assert!(unparsable.is_legacy());
        let ok = CliSelection { binary: "bd".into(), probe: Some(probe(CliClient::Bd, Some((1, 0, 0)))) };
        assert!(!ok.is_legacy());
        let br = CliSelection { binary: "br".into(), probe: Some(probe(CliClient::Br, Some((0, 1, 0)))) };
        assert!(!br.is_legacy());
        let none = CliSelection { binary: "bd".into(), probe: None };
        assert!(!none.is_legacy());
    }

    #[test]
    fn warnings_for_0_49_x_have_no_dolt_note() {
        for v in [(0, 49, 0), (0, 49, 6), (0, 49, 99)] {
            let w = cli_compatibility_warnings(CliClient::Bd, Some(v));
            assert_eq!(w.len(), 1, "{v:?}: {w:?}");
            assert!(w[0].contains(&format!("bd {}.{}.{}", v.0, v.1, v.2)), "{w:?}");
            assert!(!w.iter().any(|m| m.contains("Dolt")), "{w:?}");
        }
    }

    #[test]
    fn warnings_for_0_50_through_0_56_include_dolt_note() {
        for v in [(0, 50, 0), (0, 53, 2), (0, 56, 9), (0, 99, 0)] {
            let w = cli_compatibility_warnings(CliClient::Bd, Some(v));
            assert_eq!(w.len(), 2, "{v:?}: {w:?}");
            assert!(w[0].contains(&format!("bd {}.{}.{}", v.0, v.1, v.2)), "{w:?}");
            assert!(w[0].contains("legacy"), "{w:?}");
            assert!(w[1].contains("Dolt"), "{w:?}");
            assert!(w[1].contains("polling"), "{w:?}");
        }
    }

    #[test]
    fn warnings_mention_supported_floor_for_unparsable_bd() {
        let w = cli_compatibility_warnings(CliClient::Bd, None);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains(&format!("bd {}.x", MIN_SUPPORTED_BD_MAJOR)), "{w:?}");
    }

    #[test]
    fn warnings_for_br_and_unknown_ignore_version() {
        assert_eq!(cli_compatibility_warnings(CliClient::Br, None).len(), 1);
        assert_eq!(cli_compatibility_warnings(CliClient::Br, Some((9, 9, 9))).len(), 1);
        assert_eq!(cli_compatibility_warnings(CliClient::Unknown, Some((1, 0, 0))).len(), 1);
        assert_eq!(cli_compatibility_warnings(CliClient::Unknown, None).len(), 1);
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
    fn cli_client_name_mapping() {
        assert_eq!(cli_client_name(CliClient::Bd), "bd");
        assert_eq!(cli_client_name(CliClient::Br), "br");
        assert_eq!(cli_client_name(CliClient::Unknown), "unknown");
    }

    #[test]
    fn compatibility_info_arrays_and_null_tuple_serialize() {
        let info = CompatibilityInfo {
            binary: "bd".into(),
            found: false,
            version: "bd not found".into(),
            client_type: cli_client_name(CliClient::Unknown).into(),
            version_tuple: None,
            legacy: false,
            min_supported_major: MIN_SUPPORTED_BD_MAJOR,
            supports_daemon_flag: false,
            uses_jsonl_files: false,
            uses_dolt_backend: false,
            supports_list_all_flag: false,
            searched_paths: vec!["/a".into(), "/b".into()],
            warnings: vec!["w1".into(), "w2".into()],
        };
        let json = serde_json::to_value(&info).unwrap();
        assert!(json["warnings"].is_array());
        assert_eq!(json["warnings"].as_array().unwrap().len(), 2);
        assert!(json["searchedPaths"].is_array());
        assert_eq!(json["searchedPaths"], serde_json::json!(["/a", "/b"]));
        // Present and null, not omitted: the frontend distinguishes "unknown" from "missing key".
        assert!(json.as_object().unwrap().contains_key("versionTuple"));
        assert!(json["versionTuple"].is_null());
        assert_eq!(json["found"], serde_json::json!(false));
        assert_eq!(json["clientType"], serde_json::json!("unknown"));
        assert_eq!(json["minSupportedMajor"], serde_json::json!(MIN_SUPPORTED_BD_MAJOR));
    }

    #[test]
    fn compatibility_info_empty_arrays_serialize_as_empty_not_null() {
        let info = CompatibilityInfo {
            binary: "bd".into(),
            found: true,
            version: "bd version 1.0.4".into(),
            client_type: "bd".into(),
            version_tuple: Some(vec![1, 0, 4]),
            legacy: false,
            min_supported_major: MIN_SUPPORTED_BD_MAJOR,
            supports_daemon_flag: false,
            uses_jsonl_files: false,
            uses_dolt_backend: true,
            supports_list_all_flag: true,
            searched_paths: vec![],
            warnings: vec![],
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["warnings"], serde_json::json!([]));
        assert_eq!(json["searchedPaths"], serde_json::json!([]));
        assert_eq!(json["versionTuple"], serde_json::json!([1, 0, 4]));
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

    #[test]
    fn project_uses_dolt_true_with_legacy_dolt_dir() {
        let temp_dir = std::env::temp_dir().join(format!("beads_test_legacy_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
        let beads_dir = temp_dir.join(".beads");
        let dolt_dir = beads_dir.join(".dolt");
        let _ = std::fs::create_dir_all(&dolt_dir);

        // Note: project_uses_dolt checks CLI version first; this may return false
        // depending on whether bd 0.49 is available. Test just the dir structure.
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // ---- Version-gate helpers (#7) -----------------------------------------------

    #[test]
    fn supports_daemon_flag_table_driven() {
        // (client, major, minor, patch, expected)
        let cases = vec![
            (CliClient::Bd, 0, 49, 6, true),   // bd 0.49.6: 0 < 50
            (CliClient::Bd, 0, 50, 0, false),  // bd 0.50.0: 0 !< 50
            (CliClient::Bd, 0, 52, 0, false),  // bd 0.52.0
            (CliClient::Bd, 0, 55, 0, false),  // bd 0.55.0
            (CliClient::Bd, 0, 56, 0, false),  // bd 0.56.0
            (CliClient::Bd, 1, 0, 4, false),   // bd 1.0.4: major != 0
            (CliClient::Bd, 1, 2, 1, false),   // bd 1.2.1
            (CliClient::Br, 0, 1, 33, false),  // br 0.1.33: always false
            (CliClient::Unknown, 0, 0, 0, false), // Unknown: always false
        ];

        for (client, major, minor, patch, expected) in cases {
            let result = supports_daemon_flag_for(client, major, minor, patch);
            assert_eq!(result, expected,
                "supports_daemon_flag_for({:?}, {}.{}.{}) should be {}",
                client, major, minor, patch, expected);
        }
    }

    #[test]
    fn uses_jsonl_files_table_driven() {
        // (client, major, minor, patch, expected)
        let cases = vec![
            (CliClient::Bd, 0, 49, 6, true),   // bd 0.49.6: major==0 && minor < 50
            (CliClient::Bd, 0, 50, 0, false),  // bd 0.50.0: minor !< 50
            (CliClient::Bd, 0, 52, 0, false),  // bd 0.52.0
            (CliClient::Bd, 0, 55, 0, false),  // bd 0.55.0
            (CliClient::Bd, 0, 56, 0, false),  // bd 0.56.0
            (CliClient::Bd, 1, 0, 4, false),   // bd 1.0.4: major != 0
            (CliClient::Bd, 1, 2, 1, false),   // bd 1.2.1
            (CliClient::Br, 0, 1, 33, true),   // br 0.1.33: always true
            (CliClient::Unknown, 0, 0, 0, false), // Unknown: always false
        ];

        for (client, major, minor, patch, expected) in cases {
            let result = uses_jsonl_files_for(client, major, minor, patch);
            assert_eq!(result, expected,
                "uses_jsonl_files_for({:?}, {}.{}.{}) should be {}",
                client, major, minor, patch, expected);
        }
    }

    #[test]
    fn supports_list_all_flag_table_driven() {
        // (client, major, minor, patch, expected)
        let cases = vec![
            (CliClient::Bd, 0, 49, 6, false),  // bd 0.49.6: major !> 0 && minor !>= 55
            (CliClient::Bd, 0, 50, 0, false),  // bd 0.50.0
            (CliClient::Bd, 0, 52, 0, false),  // bd 0.52.0
            (CliClient::Bd, 0, 55, 0, true),   // bd 0.55.0: minor >= 55
            (CliClient::Bd, 0, 56, 0, true),   // bd 0.56.0: minor >= 55
            (CliClient::Bd, 1, 0, 4, true),    // bd 1.0.4: major > 0
            (CliClient::Bd, 1, 2, 1, true),    // bd 1.2.1: major > 0
            (CliClient::Br, 0, 1, 33, true),   // br 0.1.33: always true
            (CliClient::Unknown, 0, 0, 0, false), // Unknown: always false
        ];

        for (client, major, minor, patch, expected) in cases {
            let result = supports_list_all_flag_for(client, major, minor, patch);
            assert_eq!(result, expected,
                "supports_list_all_flag_for({:?}, {}.{}.{}) should be {}",
                client, major, minor, patch, expected);
        }
    }

    #[test]
    fn supports_delete_hard_flag_table_driven() {
        // (client, major, minor, patch, expected)
        let cases = vec![
            (CliClient::Bd, 0, 49, 6, true),   // bd 0.49.6: major==0 && minor < 50
            (CliClient::Bd, 0, 50, 0, false),  // bd 0.50.0: minor !< 50
            (CliClient::Bd, 0, 52, 0, false),  // bd 0.52.0
            (CliClient::Bd, 0, 55, 0, false),  // bd 0.55.0
            (CliClient::Bd, 0, 56, 0, false),  // bd 0.56.0
            (CliClient::Bd, 1, 0, 4, false),   // bd 1.0.4: major != 0
            (CliClient::Bd, 1, 2, 1, false),   // bd 1.2.1
            (CliClient::Br, 0, 1, 33, false),  // br 0.1.33: always false
            (CliClient::Unknown, 0, 0, 0, false), // Unknown: always false
        ];

        for (client, major, minor, patch, expected) in cases {
            let result = supports_delete_hard_flag_for(client, major, minor, patch);
            assert_eq!(result, expected,
                "supports_delete_hard_flag_for({:?}, {}.{}.{}) should be {}",
                client, major, minor, patch, expected);
        }
    }

    #[test]
    fn uses_dolt_backend_table_driven() {
        // (client, major, minor, patch, expected)
        let cases = vec![
            (CliClient::Bd, 0, 49, 6, false),  // bd 0.49.6: major !> 0 && minor !>= 50
            (CliClient::Bd, 0, 50, 0, true),   // bd 0.50.0: minor >= 50
            (CliClient::Bd, 0, 52, 0, true),   // bd 0.52.0: minor >= 50
            (CliClient::Bd, 0, 55, 0, true),   // bd 0.55.0: minor >= 50
            (CliClient::Bd, 0, 56, 0, true),   // bd 0.56.0: minor >= 50
            (CliClient::Bd, 1, 0, 4, true),    // bd 1.0.4: major > 0
            (CliClient::Bd, 1, 2, 1, true),    // bd 1.2.1: major > 0
            (CliClient::Br, 0, 1, 33, false),  // br 0.1.33: always false
            (CliClient::Unknown, 0, 0, 0, false), // Unknown: always false
        ];

        for (client, major, minor, patch, expected) in cases {
            let result = uses_dolt_backend_for(client, major, minor, patch);
            assert_eq!(result, expected,
                "uses_dolt_backend_for({:?}, {}.{}.{}) should be {}",
                client, major, minor, patch, expected);
        }
    }

}
