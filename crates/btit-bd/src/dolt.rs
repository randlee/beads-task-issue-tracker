//! Dolt detection, the [`DoltMode`] switch, and the `DoltOperations` argument builders
//! for [`crate::backend::BdCli`].
//!
//! [`project_uses_dolt_for`] is the only backend that can say yes: br never uses Dolt
//! and a bd below 0.50 never does either. On bd 0.51 and later it follows bd's own
//! `metadata.json` backend rule. [`project_dolt_mode`] resolves how a Dolt project is
//! opened (embedded, server, proxied server) with bd's `GetDoltMode()` rules; any
//! mode-dependent behaviour in this crate is a `match` on [`DoltMode`], not a second
//! implementation. The argument builders and [`to_dolt_result`] are the pure pieces
//! `BdCli`'s `DoltOperations` impl composes with `inv.run_raw`.
//!
//! Rules are taken from bd at `610339cd7`, `internal/configfile/configfile.go`.

use std::path::Path;

use btit_types::{CliClient, CliOutput, DoltOpResult};

/// `metadata.json` backend values bd recognizes as non-Dolt (`BackendSQLite`,
/// `BackendPostgres`, `BackendMySQL`, `configfile.go:245-247`); `GetBackend()`
/// returns them unchanged and falls back to Dolt for every other value
/// (`configfile.go:297-311`).
const NON_DOLT_BACKENDS: [&str; 3] = ["sqlite", "postgres", "mysql"];

/// Pure core of `BeadsBackend::project_uses_dolt`: decides from the (possibly
/// unknown) CLI client info and the on-disk layout of `beads_dir`. Testable without
/// spawning `bd`.
///
/// - br never uses Dolt; bd < 0.50 never uses Dolt.
/// - bd 0.50.x: `.beads/.dolt` (legacy layout), or `metadata.json` declaring
///   `"backend":"dolt"` AND a `dolt/<name>/.dolt` directory (SQLite still existed).
/// - bd >= 0.51, an unknown client, or no probe: `.beads/.dolt` (legacy layout), or a
///   parsable `metadata.json` whose `backend` is not `sqlite`, `postgres` or `mysql`
///   (bd's `GetBackend()` default-to-Dolt rule). Server-mode and custom
///   `dolt_data_dir` projects are therefore detected without probing `dolt/`.
///
/// An absent, unreadable or unparsable `metadata.json` yields `false`.
#[must_use]
pub fn project_uses_dolt_for(info: Option<(CliClient, u32, u32, u32)>, beads_dir: &Path) -> bool {
    match info {
        Some((CliClient::Br, ..)) => false,
        Some((CliClient::Bd, 0, minor, _)) if minor < 50 => false,
        Some((CliClient::Bd, 0, 50, _)) => legacy_filesystem_probe(beads_dir),
        _ => {
            if beads_dir.join(".dolt").is_dir() {
                return true;
            }
            read_metadata(beads_dir).is_some_and(|meta| metadata_backend_is_dolt(&meta))
        }
    }
}

/// How bd opens a Dolt project, as persisted in its `metadata.json`
/// (`DoltModeEmbedded`, `DoltModeServer`, `DoltModeProxiedServer`,
/// `configfile.go:325-327`).
///
/// The single switch point for mode-dependent behaviour in `btit-bd`: callers
/// `match` on it instead of choosing between separate implementations.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DoltMode {
    /// Dolt runs in-process (`"embedded"`, bd's default for a standalone project).
    Embedded,
    /// bd connects to an external `dolt sql-server` (`"server"`).
    Server,
    /// bd connects through a beads team server proxy (`"proxied-server"`).
    ProxiedServer,
}

/// Resolves the [`DoltMode`] a project's `.beads` directory persists, following bd's
/// `GetDoltMode()` (`configfile.go:471-479`) and `HostImpliesServerMode()`
/// (`configfile.go:417-437`).
///
/// - `metadata.json` absent, unreadable or unparsable, or its `backend` is `sqlite`,
///   `postgres` or `mysql` → `None` (not a Dolt project).
/// - `dolt_mode` non-empty, compared case-insensitively: `server` → [`DoltMode::Server`],
///   `proxied-server` → [`DoltMode::ProxiedServer`], anything else →
///   [`DoltMode::Embedded`] (bd's `IsDoltServerMode` treats every other explicit
///   value as not-server).
/// - `dolt_mode` missing or empty: a non-local `dolt_server_host` →
///   [`DoltMode::Server`], otherwise [`DoltMode::Embedded`].
///
/// Only the project-local persisted mode is reported: bd's runtime overrides
/// (`BEADS_DOLT_SERVER_MODE`, `BEADS_DOLT_SHARED_SERVER`, `BEADS_DOLT_SERVER_HOST`)
/// and the `config.yaml` `dolt.mode`/`dolt.host` fallbacks are not consulted. A
/// `dolt-server.port` file or a running `dolt sql-server` is never evidence of server
/// mode.
#[must_use]
pub fn project_dolt_mode(beads_dir: &Path) -> Option<DoltMode> {
    let meta = read_metadata(beads_dir)?;
    if !metadata_backend_is_dolt(&meta) {
        return None;
    }
    let explicit = string_field(&meta, "dolt_mode");
    if explicit.is_empty() {
        return Some(if is_local_host(string_field(&meta, "dolt_server_host")) {
            DoltMode::Embedded
        } else {
            DoltMode::Server
        });
    }
    Some(match explicit.to_lowercase().as_str() {
        "server" => DoltMode::Server,
        "proxied-server" => DoltMode::ProxiedServer,
        _ => DoltMode::Embedded,
    })
}

/// The bd 0.50.x rule, unchanged from before bd 0.51: `.beads/.dolt`, or
/// `metadata.json` declaring `"backend":"dolt"` and a `dolt/<name>/.dolt` directory.
fn legacy_filesystem_probe(beads_dir: &Path) -> bool {
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

/// `beads_dir/metadata.json` parsed as JSON; `None` when it is absent, unreadable
/// or not valid JSON.
fn read_metadata(beads_dir: &Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(beads_dir.join("metadata.json")).ok()?;
    serde_json::from_str::<serde_json::Value>(&text).ok()
}

/// bd's `GetBackend() == BackendDolt`: every `backend` value other than `sqlite`,
/// `postgres` and `mysql`, including a missing key, is Dolt (`configfile.go:297-311`).
fn metadata_backend_is_dolt(meta: &serde_json::Value) -> bool {
    meta.get("backend")
        .and_then(serde_json::Value::as_str)
        .is_none_or(|backend| !NON_DOLT_BACKENDS.contains(&backend))
}

/// A string field of `metadata.json`, or `""` when it is missing or not a string
/// (bd's `omitempty` string fields read as empty).
fn string_field<'a>(meta: &'a serde_json::Value, key: &str) -> &'a str {
    meta.get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
}

/// bd's `IsLocalHostString` (`configfile.go:444-450`): empty, `localhost`,
/// `127.0.0.1`, `::1`, `[::1]` or `0.0.0.0` after trimming and lowercasing.
fn is_local_host(host: &str) -> bool {
    matches!(
        host.trim().to_lowercase().as_str(),
        "" | "localhost" | "127.0.0.1" | "::1" | "[::1]" | "0.0.0.0"
    )
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
    // Every test calls the pure `_for` core with a fixed client tuple, so none spawns
    // `bd` (b-10, B10).

    fn dolt_tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "beads_dolt_{}_{}_{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap_or_else(|e| panic!("create_dir_all failed: {e}"));
        d
    }

    const BD_1: Option<(CliClient, u32, u32, u32)> = Some((CliClient::Bd, 1, 0, 4));
    const BD_0_50: Option<(CliClient, u32, u32, u32)> = Some((CliClient::Bd, 0, 50, 3));

    fn write_metadata(beads: &Path, text: &str) {
        std::fs::write(beads.join("metadata.json"), text)
            .unwrap_or_else(|e| panic!("write failed: {e}"));
    }

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
    fn project_uses_dolt_for_bd_1x_reads_metadata_backend() {
        struct Case {
            metadata: Option<&'static str>,
            legacy_dolt_dir: bool,
            info: Option<(CliClient, u32, u32, u32)>,
            expected: bool,
        }
        let cases = [
            Case {
                metadata: Some(r#"{"backend":"dolt"}"#),
                legacy_dolt_dir: false,
                info: BD_1,
                expected: true,
            },
            Case {
                metadata: Some("{}"),
                legacy_dolt_dir: false,
                info: BD_1,
                expected: true,
            },
            Case {
                metadata: Some(r#"{"dolt_mode":"server"}"#),
                legacy_dolt_dir: false,
                info: BD_1,
                expected: true,
            },
            Case {
                metadata: Some(r#"{"backend":"sqlite"}"#),
                legacy_dolt_dir: false,
                info: BD_1,
                expected: false,
            },
            Case {
                metadata: Some(r#"{"backend":"postgres"}"#),
                legacy_dolt_dir: false,
                info: BD_1,
                expected: false,
            },
            Case {
                metadata: None,
                legacy_dolt_dir: false,
                info: BD_1,
                expected: false,
            },
            Case {
                metadata: Some("not json"),
                legacy_dolt_dir: false,
                info: BD_1,
                expected: false,
            },
            Case {
                metadata: None,
                legacy_dolt_dir: true,
                info: BD_1,
                expected: true,
            },
            // Deliberate deviation (sprint-b-10 Deliverable 2): no probe and an
            // unknown client take the bd >= 0.51 metadata rule.
            Case {
                metadata: Some(r#"{"backend":"dolt"}"#),
                legacy_dolt_dir: false,
                info: None,
                expected: true,
            },
            Case {
                metadata: Some(r#"{"backend":"dolt"}"#),
                legacy_dolt_dir: false,
                info: Some((CliClient::Unknown, 9, 9, 9)),
                expected: true,
            },
        ];
        for (row, case) in cases.iter().enumerate() {
            let beads = dolt_tmp(&format!("bd1x_{row}"));
            if let Some(text) = case.metadata {
                write_metadata(&beads, text);
            }
            if case.legacy_dolt_dir {
                std::fs::create_dir_all(beads.join(".dolt"))
                    .unwrap_or_else(|e| panic!("create_dir_all failed: {e}"));
            }
            assert_eq!(
                project_uses_dolt_for(case.info, &beads),
                case.expected,
                "row {row}: metadata {:?}, .dolt dir {}, info {:?}",
                case.metadata,
                case.legacy_dolt_dir,
                case.info
            );
            let _ = std::fs::remove_dir_all(&beads);
        }
    }

    #[test]
    fn project_uses_dolt_for_bd_0_50_keeps_filesystem_probe() {
        let beads = dolt_tmp("bd050");
        // empty dir
        assert!(!project_uses_dolt_for(BD_0_50, &beads));
        // sqlite metadata
        write_metadata(&beads, r#"{"backend":"sqlite"}"#);
        assert!(!project_uses_dolt_for(BD_0_50, &beads));
        // unparsable metadata
        write_metadata(&beads, "not json");
        assert!(!project_uses_dolt_for(BD_0_50, &beads));
        // `{}` defaults to Dolt on bd >= 0.51 only; 0.50 still needs the database dir
        write_metadata(&beads, "{}");
        assert!(!project_uses_dolt_for(BD_0_50, &beads));
        // metadata says dolt but no dolt/ directory yet
        write_metadata(&beads, r#"{"backend": "dolt"}"#);
        assert!(!project_uses_dolt_for(BD_0_50, &beads));
        std::fs::create_dir_all(beads.join("dolt").join("proj").join(".dolt"))
            .unwrap_or_else(|e| panic!("create_dir_all failed: {e}"));
        assert!(project_uses_dolt_for(BD_0_50, &beads));
        let _ = std::fs::remove_dir_all(&beads);
    }

    #[test]
    fn project_uses_dolt_for_is_false_for_dir_without_beads_layout() {
        let project = dolt_tmp("no_beads");
        assert!(!project_uses_dolt_for(BD_1, &project.join(".beads")));
        let _ = std::fs::remove_dir_all(&project);
    }

    #[test]
    fn project_uses_dolt_for_legacy_dolt_dir_by_client() {
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
