//! Dolt detection and the `DoltOperations` argument builders for [`crate::backend::BdCli`].
//!
//! [`project_uses_dolt_for`] is the only backend that can say yes: br never uses Dolt
//! and a bd below 0.50 never does either. The argument builders and
//! [`to_dolt_result`] are the pure pieces `BdCli`'s `DoltOperations` impl composes
//! with `inv.run_raw`.

use std::path::Path;

use btit_types::{CliClient, CliOutput, DoltOpResult};

/// Pure core of `BeadsBackend::project_uses_dolt`: decides from the (possibly
/// unknown) CLI client info and the on-disk layout of `beads_dir`. Testable without
/// spawning `bd`.
///
/// - br never uses Dolt; bd < 0.50 never uses Dolt
/// - otherwise: `.beads/.dolt` (legacy layout), or `metadata.json` declaring
///   `"backend":"dolt"` AND a `dolt/<name>/.dolt` directory (bd 0.52+ layout)
#[must_use]
pub fn project_uses_dolt_for(info: Option<(CliClient, u32, u32, u32)>, beads_dir: &Path) -> bool {
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
                if content.contains("\"backend\":\"dolt\"")
                    || content.contains("\"backend\": \"dolt\"")
                {
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

/// `doctor --fix --yes` (`migration.rs:334`).
#[must_use]
pub(crate) fn doctor_fix_args() -> [&'static str; 3] {
    ["doctor", "--fix", "--yes"]
}

/// `migrate --to-dolt --yes` (`migration.rs:624`).
#[must_use]
pub(crate) fn migrate_to_dolt_args() -> [&'static str; 3] {
    ["migrate", "--to-dolt", "--yes"]
}

/// `init --prefix <prefix>` (`migration.rs:666,772`).
#[must_use]
pub(crate) fn init_args(prefix: &str) -> [&str; 3] {
    ["init", "--prefix", prefix]
}

/// `import -i <file>` (`migration.rs:880`).
#[must_use]
pub(crate) fn import_args(file: &str) -> [&str; 3] {
    ["import", "-i", file]
}

/// Maps a raw [`CliOutput`] to the transport-neutral [`DoltOpResult`] every
/// `DoltOperations` method returns: trimmed stdout as the success message, trimmed
/// stderr as the failure detail (what `migration.rs` reads from the process today).
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "takes CliOutput by value so `run_raw(..).map(to_dolt_result)` composes without a closure"
)]
pub fn to_dolt_result(out: CliOutput) -> DoltOpResult {
    DoltOpResult {
        success: out.success,
        message: out.stdout.trim().to_owned(),
        detail: out.stderr.trim().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Dolt-detection filesystem tests (moved from btit-app/src/cli.rs) ------
    //
    // `project_uses_dolt_false_without_beads_dir`, which exercises the wrapper
    // (`BdCli::project_uses_dolt`, which still spawns `bd --version`), moved to
    // `backend.rs`'s test module instead of here, since it tests `BdCli` rather than
    // this module's pure core.

    fn dolt_tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "beads_dolt_{}_{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap_or_else(|e| panic!("create_dir_all failed: {e}"));
        d
    }

    const BD_1: Option<(CliClient, u32, u32, u32)> = Some((CliClient::Bd, 1, 0, 4));

    #[test]
    fn project_uses_dolt_for_legacy_dolt_dir() {
        let beads = dolt_tmp("legacy");
        std::fs::create_dir_all(beads.join(".dolt"))
            .unwrap_or_else(|e| panic!("create_dir_all failed: {e}"));
        assert!(project_uses_dolt_for(BD_1, &beads));
        // Unknown client info falls through to the layout check
        assert!(project_uses_dolt_for(None, &beads));
        let _ = std::fs::remove_dir_all(&beads);
    }

    #[test]
    fn project_uses_dolt_for_nested_layout_needs_metadata_and_dolt_dir() {
        let beads = dolt_tmp("nested");
        std::fs::write(beads.join("metadata.json"), r#"{"backend": "dolt"}"#)
            .unwrap_or_else(|e| panic!("write failed: {e}"));
        // metadata says dolt but no dolt/ directory yet
        assert!(!project_uses_dolt_for(BD_1, &beads));
        std::fs::create_dir_all(beads.join("dolt").join("proj").join(".dolt"))
            .unwrap_or_else(|e| panic!("create_dir_all failed: {e}"));
        assert!(project_uses_dolt_for(BD_1, &beads));
        let _ = std::fs::remove_dir_all(&beads);
    }

    #[test]
    fn project_uses_dolt_for_sqlite_metadata_or_empty_dir_is_false() {
        let beads = dolt_tmp("sqlite");
        assert!(!project_uses_dolt_for(BD_1, &beads));
        std::fs::write(beads.join("metadata.json"), r#"{"backend":"sqlite"}"#)
            .unwrap_or_else(|e| panic!("write failed: {e}"));
        assert!(!project_uses_dolt_for(BD_1, &beads));
        std::fs::write(beads.join("metadata.json"), "not json")
            .unwrap_or_else(|e| panic!("write failed: {e}"));
        assert!(!project_uses_dolt_for(BD_1, &beads));
        let _ = std::fs::remove_dir_all(&beads);
    }

    #[test]
    fn project_uses_dolt_for_br_and_legacy_bd_never_true() {
        let beads = dolt_tmp("never");
        std::fs::create_dir_all(beads.join(".dolt"))
            .unwrap_or_else(|e| panic!("create_dir_all failed: {e}"));
        assert!(!project_uses_dolt_for(
            Some((CliClient::Br, 0, 1, 33)),
            &beads
        ));
        assert!(!project_uses_dolt_for(
            Some((CliClient::Bd, 0, 49, 6)),
            &beads
        ));
        assert!(project_uses_dolt_for(
            Some((CliClient::Bd, 0, 50, 0)),
            &beads
        ));
        let _ = std::fs::remove_dir_all(&beads);
    }

    // ---- DoltOperations argument-vector and mapping tests -----------------------

    #[test]
    fn doctor_fix_args_is_fixed() {
        assert_eq!(doctor_fix_args(), ["doctor", "--fix", "--yes"]);
    }

    #[test]
    fn migrate_to_dolt_args_is_fixed() {
        assert_eq!(migrate_to_dolt_args(), ["migrate", "--to-dolt", "--yes"]);
    }

    #[test]
    fn init_args_carries_prefix() {
        assert_eq!(init_args("proj"), ["init", "--prefix", "proj"]);
    }

    #[test]
    fn import_args_carries_file() {
        assert_eq!(
            import_args("issues.jsonl"),
            ["import", "-i", "issues.jsonl"]
        );
    }

    #[test]
    fn to_dolt_result_trims_stdout_and_stderr() {
        let out = CliOutput {
            status: Some(1),
            success: false,
            stdout: "  ok  \n".to_string(),
            stderr: "  boom  \n".to_string(),
        };
        assert_eq!(
            to_dolt_result(out),
            DoltOpResult {
                success: false,
                message: "ok".to_string(),
                detail: "boom".to_string(),
            }
        );
    }
}
