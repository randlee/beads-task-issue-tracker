//! The `--version` probe and default-binary auto-detection.
//!
//! Both spawn `<binary> --version` from the system temp directory with the extended
//! `PATH` ([`run::probe_version_output`](crate::run::probe_version_output)); the
//! selection and parsing rules are the pure ones in `btit_beads::detect`.

use btit_beads::detect::{
    cli_client_name, parse_cli_probe, select_default_binary, CLI_CANDIDATES, CLI_FALLBACK,
    MIN_SUPPORTED_BD_MAJOR,
};
use btit_types::CliProbe;

use crate::path::extended_path_entries;
use crate::run::probe_version_output;

/// Run `bin --version` and parse it. `None` when the binary is missing,
/// not executable, or exits non-zero.
/// Runs with the extended PATH so GUI launches (Finder/Dock, minimal PATH)
/// can still resolve Homebrew / Go / Cargo installs, and from the temp dir so
/// bd never auto-migrates a project as a side effect of the probe.
#[must_use]
pub fn probe_cli_binary(bin: &str) -> Option<CliProbe> {
    let output = probe_version_output(bin).ok()?;
    if !output.success {
        return None;
    }
    Some(parse_cli_probe(&output.stdout))
}

/// Auto-detects the CLI binary: probes every candidate (`bd`, then `br`) and returns
/// the best-ranked one, or `bd` when none answers.
///
/// Logs the choice (and a legacy warning for a bd below the supported floor) through
/// `log` directly, not the gated macros, because it runs before logging is configured.
#[must_use]
pub fn default_cli_binary() -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_returns_none_for_nonexistent_binary() {
        assert!(probe_cli_binary("definitely-not-a-real-cli-binary-xyz").is_none());
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
}
