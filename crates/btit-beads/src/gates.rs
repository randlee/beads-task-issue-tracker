//! Version gates: which CLI flags and storage layouts a client/version has.
//!
//! The five `_for` cores keep their `(CliClient, u32, u32, u32)` signatures;
//! [`capabilities_for`] bundles them into a [`BackendCapabilities`].

use btit_types::{BackendCapabilities, CliClient, CliVersion};

/// Returns true if the CLI supports the --no-daemon flag.
/// - br: NEVER (no daemon concept)
/// - bd < 0.50.0: YES
/// - bd >= 0.50.0: NO (daemon removed)
/// - unknown: NO (safe default)
#[must_use]
#[expect(
    clippy::match_same_arms,
    reason = "moved verbatim from btit-app in b-3; behaviour and body edits belong to b-9"
)]
pub fn supports_daemon_flag_for(client: CliClient, major: u32, minor: u32, _patch: u32) -> bool {
    match client {
        CliClient::Br => false, // br has no daemon
        CliClient::Bd => major == 0 && minor < 50,
        CliClient::Unknown => false,
    }
}

/// Returns true if the CLI uses issues.jsonl files.
/// - br: ALWAYS (frozen on SQLite+JSONL architecture)
/// - bd < 0.51.0: YES
/// - bd >= 0.51.0: NO (Dolt only; the tombstone system replaced JSONL in 0.51.0, B5)
/// - unknown: NO (safe default)
#[must_use]
pub fn uses_jsonl_files_for(client: CliClient, major: u32, minor: u32, _patch: u32) -> bool {
    match client {
        CliClient::Br => true, // br always uses JSONL
        CliClient::Bd => major == 0 && minor < 51,
        CliClient::Unknown => false,
    }
}

/// Returns true if `bd list --all` works correctly.
/// The --all flag was buggy before bd 0.55.0 (returned incorrect results).
/// - br: returns true (unverified against `beads_rust`; OQ-4)
/// - bd >= 0.55.0: YES
/// - bd < 0.55.0: NO (use 2 separate calls instead)
/// - unknown: NO (safe default)
#[must_use]
pub fn supports_list_all_flag_for(client: CliClient, major: u32, minor: u32, _patch: u32) -> bool {
    match client {
        CliClient::Bd => major > 0 || minor >= 55,
        CliClient::Br => true, // br always supports --all
        CliClient::Unknown => false,
    }
}

/// Returns true if `bd delete --hard` is supported.
/// The --hard flag was removed in bd 0.51.0 (the tombstone system replaced it, B4).
/// - br: NO
/// - bd < 0.51.0: YES
/// - bd >= 0.51.0: NO (only --force needed)
/// - unknown: NO (safe default)
#[must_use]
pub fn supports_delete_hard_flag_for(
    client: CliClient,
    major: u32,
    minor: u32,
    _patch: u32,
) -> bool {
    match client {
        CliClient::Bd => major == 0 && minor < 51,
        _ => false,
    }
}

/// Returns true if the CLI uses the Dolt backend (inverse of uses_jsonl_files).
/// - br: NEVER (frozen on SQLite+JSONL architecture)
/// - bd >= 0.51.0: YES (Dolt only, B5)
/// - bd < 0.51.0: NO (SQLite+JSONL)
/// - unknown: NO (safe default)
#[must_use]
#[expect(
    clippy::match_same_arms,
    clippy::doc_markdown,
    reason = "moved verbatim from btit-app in b-3; behaviour and body edits belong to b-9"
)]
pub fn uses_dolt_backend_for(client: CliClient, major: u32, minor: u32, _patch: u32) -> bool {
    match client {
        CliClient::Br => false, // br never uses Dolt
        CliClient::Bd => major > 0 || minor >= 51,
        CliClient::Unknown => false,
    }
}

/// All five version gates for `client` at `version`. A `None` version (probe failed or
/// did not parse) yields all `false`, matching the wrappers' `None => false` arms
/// (cli.rs:380-466 at `a18c724`).
#[must_use]
pub fn capabilities_for(client: CliClient, version: Option<CliVersion>) -> BackendCapabilities {
    match version {
        None => BackendCapabilities::default(), // today's `None => false` arms
        Some(v) => {
            let (ma, mi, pa) = v.into();
            BackendCapabilities {
                supports_daemon_flag: supports_daemon_flag_for(client, ma, mi, pa),
                uses_jsonl_files: uses_jsonl_files_for(client, ma, mi, pa),
                uses_dolt_backend: uses_dolt_backend_for(client, ma, mi, pa),
                supports_list_all_flag: supports_list_all_flag_for(client, ma, mi, pa),
                supports_delete_hard_flag: supports_delete_hard_flag_for(client, ma, mi, pa),
            }
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::uninlined_format_args,
    reason = "tests moved verbatim from btit-app in b-3; bodies change only for CliVersion conversions"
)]
mod tests {
    use super::*;

    // ---- Version-gate helpers (#7) -----------------------------------------------

    #[test]
    fn supports_daemon_flag_table_driven() {
        // (client, major, minor, patch, expected)
        let cases = vec![
            (CliClient::Bd, 0, 49, 6, true),      // bd 0.49.6: 0 < 50
            (CliClient::Bd, 0, 50, 0, false),     // bd 0.50.0: 0 !< 50
            (CliClient::Bd, 0, 52, 0, false),     // bd 0.52.0
            (CliClient::Bd, 0, 55, 0, false),     // bd 0.55.0
            (CliClient::Bd, 0, 56, 0, false),     // bd 0.56.0
            (CliClient::Bd, 1, 0, 4, false),      // bd 1.0.4: major != 0
            (CliClient::Bd, 1, 2, 1, false),      // bd 1.2.1
            (CliClient::Br, 0, 1, 33, false),     // br 0.1.33: always false
            (CliClient::Unknown, 0, 0, 0, false), // Unknown: always false
        ];

        for (client, major, minor, patch, expected) in cases {
            let result = supports_daemon_flag_for(client, major, minor, patch);
            assert_eq!(
                result, expected,
                "supports_daemon_flag_for({:?}, {}.{}.{}) should be {}",
                client, major, minor, patch, expected
            );
        }
    }

    #[test]
    fn uses_jsonl_files_table_driven() {
        // (client, major, minor, patch, expected)
        let cases = vec![
            (CliClient::Bd, 0, 49, 6, true),  // bd 0.49.6: major==0 && minor < 51
            (CliClient::Bd, 0, 50, 0, true),  // bd 0.50.0: minor < 51 (B5)
            (CliClient::Bd, 0, 50, 3, true),  // bd 0.50.3: minor < 51 (B5)
            (CliClient::Bd, 0, 51, 0, false), // bd 0.51.0: minor !< 51 (B5)
            (CliClient::Bd, 0, 52, 0, false), // bd 0.52.0
            (CliClient::Bd, 0, 55, 0, false), // bd 0.55.0
            (CliClient::Bd, 0, 56, 0, false), // bd 0.56.0
            (CliClient::Bd, 1, 0, 4, false),  // bd 1.0.4: major != 0
            (CliClient::Bd, 1, 2, 1, false),  // bd 1.2.1
            (CliClient::Br, 0, 1, 33, true),  // br 0.1.33: always true
            (CliClient::Unknown, 0, 0, 0, false), // Unknown: always false
        ];

        for (client, major, minor, patch, expected) in cases {
            let result = uses_jsonl_files_for(client, major, minor, patch);
            assert_eq!(
                result, expected,
                "uses_jsonl_files_for({:?}, {}.{}.{}) should be {}",
                client, major, minor, patch, expected
            );
        }
    }

    #[test]
    fn supports_list_all_flag_table_driven() {
        // (client, major, minor, patch, expected)
        let cases = vec![
            (CliClient::Bd, 0, 49, 6, false), // bd 0.49.6: major !> 0 && minor !>= 55
            (CliClient::Bd, 0, 50, 0, false), // bd 0.50.0
            (CliClient::Bd, 0, 52, 0, false), // bd 0.52.0
            (CliClient::Bd, 0, 55, 0, true),  // bd 0.55.0: minor >= 55
            (CliClient::Bd, 0, 56, 0, true),  // bd 0.56.0: minor >= 55
            (CliClient::Bd, 1, 0, 4, true),   // bd 1.0.4: major > 0
            (CliClient::Bd, 1, 2, 1, true),   // bd 1.2.1: major > 0
            (CliClient::Br, 0, 1, 33, true),  // br 0.1.33: always true
            (CliClient::Unknown, 0, 0, 0, false), // Unknown: always false
        ];

        for (client, major, minor, patch, expected) in cases {
            let result = supports_list_all_flag_for(client, major, minor, patch);
            assert_eq!(
                result, expected,
                "supports_list_all_flag_for({:?}, {}.{}.{}) should be {}",
                client, major, minor, patch, expected
            );
        }
    }

    #[test]
    fn supports_delete_hard_flag_table_driven() {
        // (client, major, minor, patch, expected)
        let cases = vec![
            (CliClient::Bd, 0, 49, 6, true),  // bd 0.49.6: major==0 && minor < 51
            (CliClient::Bd, 0, 50, 0, true),  // bd 0.50.0: minor < 51 (B4)
            (CliClient::Bd, 0, 50, 3, true),  // bd 0.50.3: minor < 51 (B4)
            (CliClient::Bd, 0, 51, 0, false), // bd 0.51.0: minor !< 51 (B4)
            (CliClient::Bd, 0, 52, 0, false), // bd 0.52.0
            (CliClient::Bd, 0, 55, 0, false), // bd 0.55.0
            (CliClient::Bd, 0, 56, 0, false), // bd 0.56.0
            (CliClient::Bd, 1, 0, 4, false),  // bd 1.0.4: major != 0
            (CliClient::Bd, 1, 2, 1, false),  // bd 1.2.1
            (CliClient::Br, 0, 1, 33, false), // br 0.1.33: always false
            (CliClient::Unknown, 0, 0, 0, false), // Unknown: always false
        ];

        for (client, major, minor, patch, expected) in cases {
            let result = supports_delete_hard_flag_for(client, major, minor, patch);
            assert_eq!(
                result, expected,
                "supports_delete_hard_flag_for({:?}, {}.{}.{}) should be {}",
                client, major, minor, patch, expected
            );
        }
    }

    #[test]
    fn uses_dolt_backend_table_driven() {
        // (client, major, minor, patch, expected)
        let cases = vec![
            (CliClient::Bd, 0, 49, 6, false), // bd 0.49.6: major !> 0 && minor !>= 51
            (CliClient::Bd, 0, 50, 0, false), // bd 0.50.0: minor !>= 51 (B5)
            (CliClient::Bd, 0, 50, 3, false), // bd 0.50.3: minor !>= 51 (B5)
            (CliClient::Bd, 0, 51, 0, true),  // bd 0.51.0: minor >= 51 (B5)
            (CliClient::Bd, 0, 52, 0, true),  // bd 0.52.0: minor >= 51
            (CliClient::Bd, 0, 55, 0, true),  // bd 0.55.0: minor >= 51
            (CliClient::Bd, 0, 56, 0, true),  // bd 0.56.0: minor >= 51
            (CliClient::Bd, 1, 0, 4, true),   // bd 1.0.4: major > 0
            (CliClient::Bd, 1, 2, 1, true),   // bd 1.2.1: major > 0
            (CliClient::Br, 0, 1, 33, false), // br 0.1.33: always false
            (CliClient::Unknown, 0, 0, 0, false), // Unknown: always false
        ];

        for (client, major, minor, patch, expected) in cases {
            let result = uses_dolt_backend_for(client, major, minor, patch);
            assert_eq!(
                result, expected,
                "uses_dolt_backend_for({:?}, {}.{}.{}) should be {}",
                client, major, minor, patch, expected
            );
        }
    }

    /// Literal expectations taken from the `_for` cores (originally `cli.rs:372-459` at
    /// `a18c724`; the `(Bd, 0.50.0)` row reflects the B4/B5 cutoff moves to 0.51.0), so a
    /// wiring mistake in `capabilities_for` cannot hide behind derived expectations.
    /// Field order: `supports_daemon_flag`, `uses_jsonl_files`, `uses_dolt_backend`,
    /// `supports_list_all_flag`, `supports_delete_hard_flag`.
    const A18C724_CAPABILITIES: [(CliClient, Option<CliVersion>, BackendCapabilities); 7] = [
        (
            CliClient::Bd,
            Some(CliVersion {
                major: 1,
                minor: 0,
                patch: 4,
            }),
            BackendCapabilities {
                supports_daemon_flag: false,
                uses_jsonl_files: false,
                uses_dolt_backend: true,
                supports_list_all_flag: true,
                supports_delete_hard_flag: false,
            },
        ),
        (
            CliClient::Bd,
            Some(CliVersion {
                major: 0,
                minor: 54,
                patch: 0,
            }),
            BackendCapabilities {
                supports_daemon_flag: false,
                uses_jsonl_files: false,
                uses_dolt_backend: true,
                supports_list_all_flag: false,
                supports_delete_hard_flag: false,
            },
        ),
        (
            CliClient::Bd,
            Some(CliVersion {
                major: 0,
                minor: 50,
                patch: 0,
            }),
            BackendCapabilities {
                supports_daemon_flag: false,
                uses_jsonl_files: true,
                uses_dolt_backend: false,
                supports_list_all_flag: false,
                supports_delete_hard_flag: true,
            },
        ),
        (
            CliClient::Bd,
            Some(CliVersion {
                major: 0,
                minor: 49,
                patch: 6,
            }),
            BackendCapabilities {
                supports_daemon_flag: true,
                uses_jsonl_files: true,
                uses_dolt_backend: false,
                supports_list_all_flag: false,
                supports_delete_hard_flag: true,
            },
        ),
        (
            CliClient::Br,
            Some(CliVersion {
                major: 0,
                minor: 1,
                patch: 33,
            }),
            BackendCapabilities {
                supports_daemon_flag: false,
                uses_jsonl_files: true,
                uses_dolt_backend: false,
                supports_list_all_flag: true,
                supports_delete_hard_flag: false,
            },
        ),
        (
            CliClient::Unknown,
            Some(CliVersion {
                major: 9,
                minor: 9,
                patch: 9,
            }),
            BackendCapabilities {
                supports_daemon_flag: false,
                uses_jsonl_files: false,
                uses_dolt_backend: false,
                supports_list_all_flag: false,
                supports_delete_hard_flag: false,
            },
        ),
        (
            CliClient::Bd,
            None,
            BackendCapabilities {
                supports_daemon_flag: false,
                uses_jsonl_files: false,
                uses_dolt_backend: false,
                supports_list_all_flag: false,
                supports_delete_hard_flag: false,
            },
        ),
    ];

    #[test]
    fn capabilities_for_pins_a18c724_values() {
        for (client, version, expected) in A18C724_CAPABILITIES {
            assert_eq!(
                capabilities_for(client, version),
                expected,
                "{client:?} {version:?}"
            );
        }
    }
}
