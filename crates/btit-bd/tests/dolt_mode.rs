//! `DoltMode` verification table (sprint b-10 Deliverable 7) and signature pin.
//!
//! Each row writes a temporary `.beads` directory and checks
//! [`project_dolt_mode`] against bd's `GetDoltMode()` rules
//! (`../beads` `610339cd7`, `internal/configfile/configfile.go:417-479`). No `bd`
//! is spawned.

use std::path::{Path, PathBuf};

use btit_bd::dolt::{project_dolt_mode, project_uses_dolt_for, DoltMode};
use btit_types::CliClient;

#[test]
fn project_dolt_mode_signature() {
    let _: fn(&Path) -> Option<DoltMode> = project_dolt_mode;
    let _: fn(&Path) -> Option<DoltMode> = btit_bd::project_dolt_mode;
}

/// A `.beads` path unique to this process and row (created by the test itself, so
/// only `#[test]` bodies panic on I/O failure).
fn beads_tmp(row: usize) -> PathBuf {
    std::env::temp_dir()
        .join(format!(
            "beads_dolt_mode_{}_{}_{}",
            row,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
        .join(".beads")
}

struct Row {
    metadata: Option<&'static str>,
    /// Maintainer's vault layout: a stale `dolt-server.port` and an `embeddeddolt/iron/` dir.
    vault_extras: bool,
    expected: Option<DoltMode>,
}

#[test]
fn project_dolt_mode_follows_bd_get_dolt_mode() {
    let rows = [
        Row {
            metadata: Some(r#"{"backend":"dolt","dolt_mode":"embedded","dolt_database":"iron"}"#),
            vault_extras: true,
            expected: Some(DoltMode::Embedded),
        },
        Row {
            metadata: Some(r#"{"backend":"dolt","dolt_mode":"server"}"#),
            vault_extras: false,
            expected: Some(DoltMode::Server),
        },
        Row {
            metadata: Some(r#"{"backend":"dolt","dolt_mode":"SERVER"}"#),
            vault_extras: false,
            expected: Some(DoltMode::Server),
        },
        Row {
            metadata: Some(r#"{"backend":"dolt","dolt_mode":"proxied-server"}"#),
            vault_extras: false,
            expected: Some(DoltMode::ProxiedServer),
        },
        Row {
            metadata: Some(r#"{"backend":"dolt","dolt_mode":"bogus"}"#),
            vault_extras: false,
            expected: Some(DoltMode::Embedded),
        },
        Row {
            metadata: Some(r#"{"backend":"dolt"}"#),
            vault_extras: false,
            expected: Some(DoltMode::Embedded),
        },
        Row {
            metadata: Some("{}"),
            vault_extras: false,
            expected: Some(DoltMode::Embedded),
        },
        Row {
            metadata: Some(r#"{"backend":"dolt","dolt_server_host":"db.example.com"}"#),
            vault_extras: false,
            expected: Some(DoltMode::Server),
        },
        Row {
            metadata: Some(r#"{"backend":"dolt","dolt_server_host":"127.0.0.1"}"#),
            vault_extras: false,
            expected: Some(DoltMode::Embedded),
        },
        Row {
            metadata: Some(
                r#"{"backend":"dolt","dolt_mode":"embedded","dolt_server_host":"db.example.com"}"#,
            ),
            vault_extras: false,
            expected: Some(DoltMode::Embedded),
        },
        Row {
            metadata: Some(r#"{"backend":"sqlite","dolt_mode":"server"}"#),
            vault_extras: false,
            expected: None,
        },
        Row {
            metadata: None,
            vault_extras: false,
            expected: None,
        },
        Row {
            metadata: Some("not json"),
            vault_extras: false,
            expected: None,
        },
    ];

    for (index, row) in rows.iter().enumerate() {
        let beads = beads_tmp(index);
        std::fs::create_dir_all(&beads).unwrap_or_else(|e| panic!("create_dir_all failed: {e}"));
        if let Some(text) = row.metadata {
            std::fs::write(beads.join("metadata.json"), text)
                .unwrap_or_else(|e| panic!("write failed: {e}"));
        }
        if row.vault_extras {
            std::fs::write(beads.join("dolt-server.port"), "3308")
                .unwrap_or_else(|e| panic!("write failed: {e}"));
            std::fs::create_dir_all(beads.join("embeddeddolt").join("iron"))
                .unwrap_or_else(|e| panic!("create_dir_all failed: {e}"));
        }

        assert_eq!(
            project_dolt_mode(&beads),
            row.expected,
            "row {index}: metadata {:?}, vault extras {}",
            row.metadata,
            row.vault_extras
        );

        if let Some(project) = beads.parent() {
            let _ = std::fs::remove_dir_all(project);
        }
    }
}

/// The maintainer's vault (`.beads/metadata.json` with `dolt_mode: embedded`, a stale
/// `dolt-server.port` and `embeddeddolt/iron/`) is Dolt on bd 1.0.4 and embedded, so
/// the app no longer takes the legacy `bd sync` path for it (B3).
#[test]
fn vault_layout_is_dolt_and_embedded() {
    let beads = beads_tmp(usize::MAX);
    std::fs::create_dir_all(beads.join("embeddeddolt").join("iron"))
        .unwrap_or_else(|e| panic!("create_dir_all failed: {e}"));
    std::fs::write(
        beads.join("metadata.json"),
        r#"{"database":"dolt","backend":"dolt","dolt_mode":"embedded","dolt_database":"iron"}"#,
    )
    .unwrap_or_else(|e| panic!("write failed: {e}"));
    std::fs::write(beads.join("dolt-server.port"), "3308")
        .unwrap_or_else(|e| panic!("write failed: {e}"));

    assert!(project_uses_dolt_for(
        Some((CliClient::Bd, 1, 0, 4)),
        &beads
    ));
    assert_eq!(project_dolt_mode(&beads), Some(DoltMode::Embedded));

    if let Some(project) = beads.parent() {
        let _ = std::fs::remove_dir_all(project);
    }
}
