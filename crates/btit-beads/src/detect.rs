//! CLI detection: candidate ranking, default-binary selection and `--version` parsing.
//!
//! Pure: the process-spawning probe lives with the transport; everything here maps
//! inputs (probe results, `--version` text) to outputs.

use btit_types::{CliClient, CliProbe, CliVersion};

/// CLI binaries probed during auto-detection, in priority order.
/// `bd` (Go) is the primary CLI this app targets; `br` (Rust) is secondary.
pub const CLI_CANDIDATES: &[&str] = &["bd", "br"];

/// Binary assumed when no candidate responds to `--version`.
pub const CLI_FALLBACK: &str = "bd";

/// Minimum `bd` major version this app targets. Older versions still run
/// through the version-gated code paths but are reported as legacy and
/// surfaced to the user. Raise this when the supported floor moves.
pub const MIN_SUPPORTED_BD_MAJOR: u32 = 1;

/// Outcome of auto-detection.
#[derive(Debug, Clone, PartialEq)]
pub struct CliSelection {
    binary: String,
    /// `None` when no candidate answered and `binary` is the fallback.
    probe: Option<CliProbe>,
}

impl CliSelection {
    /// The selected binary name (a candidate, or the fallback when none answered).
    #[must_use]
    pub fn binary(&self) -> &str {
        &self.binary
    }

    /// The winning candidate's probe; `None` when no candidate answered and
    /// [`binary`](Self::binary) is the fallback.
    #[must_use]
    pub fn probe(&self) -> Option<&CliProbe> {
        self.probe.as_ref()
    }

    /// True when a `bd` older than `MIN_SUPPORTED_BD_MAJOR` was selected.
    #[must_use]
    #[expect(
        clippy::map_unwrap_or,
        reason = "moved verbatim from btit-app in b-3; behaviour and body edits belong to b-9"
    )]
    pub fn is_legacy(&self) -> bool {
        self.probe
            .as_ref()
            .map(|p| is_legacy_bd(p.client, p.version))
            .unwrap_or(false)
    }
}

/// A `bd` whose major version is below the supported floor (or whose version
/// could not be parsed) is legacy. `br` and unknown clients are never "legacy".
#[must_use]
pub fn is_legacy_bd(client: CliClient, version: Option<CliVersion>) -> bool {
    let version: Option<(u32, u32, u32)> = version.map(Into::into);
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
#[must_use]
pub fn rank_cli_candidate(probe: &CliProbe) -> u8 {
    match probe.client {
        CliClient::Bd if !is_legacy_bd(probe.client, probe.version) => 0,
        CliClient::Bd => 1,
        CliClient::Br => 2,
        CliClient::Unknown => 3,
    }
}

/// Pure selection logic: probe every candidate, pick the best-ranked one
/// (first wins on ties), or fall back to `fallback` when none answers.
pub fn select_default_binary<F: Fn(&str) -> Option<CliProbe>>(
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
        Some((_, bin, p)) => CliSelection {
            binary: bin.to_string(),
            probe: Some(p),
        },
        None => CliSelection {
            binary: fallback.to_string(),
            probe: None,
        },
    }
}

/// Parse `--version` stdout into a `CliProbe`. Pure.
#[must_use]
pub fn parse_cli_probe(stdout: &str) -> CliProbe {
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

/// Short client name used in logs, warnings and `CompatibilityInfo.clientType`.
#[must_use]
pub fn cli_client_name(client: CliClient) -> &'static str {
    match client {
        CliClient::Bd => "bd",
        CliClient::Br => "br",
        CliClient::Unknown => "unknown",
    }
}

// ============================================================================
// CLI Client Detection (bd vs br)
// ============================================================================

/// Detect the client type from the version string.
/// - "bd version 0.49.6 (Homebrew)" → Bd
/// - "br 0.1.13 (rustc 1.85.0-nightly)" → Br
#[must_use]
pub fn detect_cli_client(version_str: &str) -> CliClient {
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
#[must_use]
#[expect(
    clippy::unnecessary_map_or,
    clippy::redundant_closure_for_method_calls,
    reason = "moved verbatim from btit-app in b-3; behaviour and body edits belong to b-9"
)]
pub fn parse_bd_version(version_str: &str) -> Option<CliVersion> {
    // Look for a semver-like pattern: digits.digits.digits
    // Accept "1.2.3" and "v1.2.3" (some CLIs print a v-prefixed tag)
    let re_like = version_str
        .split_whitespace()
        .map(|word| word.trim_start_matches(['v', 'V']))
        .find(|word| {
            word.contains('.') && word.chars().next().map_or(false, |c| c.is_ascii_digit())
        });

    let version_part = re_like?;
    let parts: Vec<&str> = version_part.split('.').collect();
    if parts.len() >= 3 {
        let major = parts.first()?.parse::<u32>().ok()?;
        let minor = parts.get(1)?.parse::<u32>().ok()?;
        // Patch may have trailing non-numeric chars (e.g. "6-beta")
        let patch_str: String = parts
            .get(2)?
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let patch = patch_str.parse::<u32>().ok()?;
        Some((major, minor, patch).into())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    // ---- CLI auto-detection -------------------------------------------------

    #[test]
    fn rank_prefers_supported_bd_then_legacy_bd_then_br_then_unknown() {
        assert_eq!(
            rank_cli_candidate(&probe(CliClient::Bd, Some((1, 0, 4)))),
            0
        );
        assert_eq!(
            rank_cli_candidate(&probe(CliClient::Bd, Some((2, 3, 0)))),
            0
        );
        assert_eq!(
            rank_cli_candidate(&probe(CliClient::Bd, Some((0, 49, 6)))),
            1
        );
        assert_eq!(
            rank_cli_candidate(&probe(CliClient::Bd, Some((0, 56, 0)))),
            1
        );
        assert_eq!(rank_cli_candidate(&probe(CliClient::Bd, None)), 1);
        assert_eq!(
            rank_cli_candidate(&probe(CliClient::Br, Some((0, 1, 33)))),
            2
        );
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
        let sel = select_default_binary(&["first", "second"], "fb", |_| {
            Some(probe(CliClient::Bd, Some((1, 2, 0))))
        });
        assert_eq!(sel.binary, "first");
    }

    #[test]
    fn parse_probe_uses_first_line_only() {
        let p = parse_cli_probe("bd version 1.0.4 (ce242a879)\nextra line\n");
        assert_eq!(p.client, CliClient::Bd);
        assert_eq!(
            p.version,
            Some(CliVersion {
                major: 1,
                minor: 0,
                patch: 4
            })
        );
        assert_eq!(p.raw, "bd version 1.0.4 (ce242a879)");
    }

    #[test]
    fn legacy_detection_respects_floor() {
        assert!(is_legacy_bd(CliClient::Bd, Some((0, 49, 6).into())));
        assert!(is_legacy_bd(CliClient::Bd, Some((0, 99, 0).into())));
        assert!(is_legacy_bd(CliClient::Bd, None));
        assert!(!is_legacy_bd(
            CliClient::Bd,
            Some((MIN_SUPPORTED_BD_MAJOR, 0, 0).into())
        ));
        assert!(!is_legacy_bd(
            CliClient::Bd,
            Some((MIN_SUPPORTED_BD_MAJOR + 1, 0, 0).into())
        ));
        assert!(!is_legacy_bd(CliClient::Br, Some((0, 1, 33).into())));
        assert!(!is_legacy_bd(CliClient::Unknown, None));
    }

    // ---- CLI client detection -----------------------------------------------

    #[test]
    fn detect_bd_1x_banner() {
        // Real output from bd 1.0.4
        assert_eq!(
            detect_cli_client("bd version 1.0.4 (ce242a879)"),
            CliClient::Bd
        );
        assert_eq!(
            parse_bd_version("bd version 1.0.4 (ce242a879)"),
            Some((1, 0, 4).into())
        );
    }

    #[test]
    fn detect_bd_legacy_banner() {
        assert_eq!(
            detect_cli_client("bd version 0.49.6 (Homebrew)"),
            CliClient::Bd
        );
        assert_eq!(
            parse_bd_version("bd version 0.49.6 (Homebrew)"),
            Some((0, 49, 6).into())
        );
    }

    #[test]
    fn detect_br_banner() {
        let banner = "br 0.1.33 (rustc 1.85.0-nightly)";
        assert_eq!(detect_cli_client(banner), CliClient::Br);
        assert_eq!(parse_bd_version(banner), Some((0, 1, 33).into()));
    }

    #[test]
    fn detect_unknown_banner() {
        assert_eq!(
            detect_cli_client("something-else 2.0.0"),
            CliClient::Unknown
        );
        assert_eq!(detect_cli_client(""), CliClient::Unknown);
    }

    #[test]
    fn parse_version_tolerates_prerelease_suffix_and_garbage() {
        assert_eq!(
            parse_bd_version("bd version 1.2.0-beta (abc)"),
            Some((1, 2, 0).into())
        );
        assert_eq!(
            parse_bd_version("bd version 1.2.3-fork"),
            Some((1, 2, 3).into())
        );
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
        assert_eq!(
            p.version,
            Some(CliVersion {
                major: 1,
                minor: 0,
                patch: 4
            })
        );
        assert_eq!(p.raw, "bd version 1.0.4 (abc)");
        assert!(!p.raw.contains('\r'));
    }

    #[test]
    fn parse_probe_trims_surrounding_whitespace_on_first_line() {
        let p = parse_cli_probe("  bd version 1.0.4 (abc)  \n");
        assert_eq!(p.client, CliClient::Bd);
        assert_eq!(
            p.version,
            Some(CliVersion {
                major: 1,
                minor: 0,
                patch: 4
            })
        );
        assert_eq!(p.raw, "bd version 1.0.4 (abc)");
    }

    #[test]
    fn parse_probe_skips_leading_blank_lines() {
        let p = parse_cli_probe("\n  \nbd version 1.0.4 (abc)\n");
        assert_eq!(p.client, CliClient::Bd);
        assert_eq!(
            p.version,
            Some(CliVersion {
                major: 1,
                minor: 0,
                patch: 4
            })
        );
        assert_eq!(p.raw, "bd version 1.0.4 (abc)");
    }

    #[test]
    fn parse_probe_accepts_v_prefixed_version() {
        let p = parse_cli_probe("bd v1.2.0");
        assert_eq!(p.client, CliClient::Bd);
        assert_eq!(
            p.version,
            Some(CliVersion {
                major: 1,
                minor: 2,
                patch: 0
            })
        );
        assert_eq!(p.raw, "bd v1.2.0");
        assert!(!is_legacy_bd(p.client, p.version));
        assert_eq!(rank_cli_candidate(&p), 0);
        assert_eq!(
            parse_bd_version("bd version V0.49.6"),
            Some((0, 49, 6).into())
        );
    }

    #[test]
    fn parse_probe_br_banner_with_beads_rust_in_later_word() {
        let p = parse_cli_probe("beads beads_rust 0.1.5 (rustc 1.85.0)\n");
        assert_eq!(p.client, CliClient::Br);
        assert_eq!(
            p.version,
            Some(CliVersion {
                major: 0,
                minor: 1,
                patch: 5
            })
        );

        let p = parse_cli_probe("cli beads-rust 0.2.0");
        assert_eq!(p.client, CliClient::Br);
        assert_eq!(
            p.version,
            Some(CliVersion {
                major: 0,
                minor: 2,
                patch: 0
            })
        );
    }

    #[test]
    fn parse_probe_bd_1x_prerelease_with_dotted_suffix() {
        let p = parse_cli_probe("bd version 1.3.0-rc.1 (abc)\n");
        assert_eq!(p.client, CliClient::Bd);
        assert_eq!(
            p.version,
            Some(CliVersion {
                major: 1,
                minor: 3,
                patch: 0
            })
        );
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
        assert_eq!(parse_bd_version("BD version 1.0.0"), Some((1, 0, 0).into()));
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
        let sel = select_default_binary(&["x", "y", "z"], "fb", |_| {
            Some(probe(CliClient::Unknown, None))
        });
        assert_eq!(
            sel.binary, "x",
            "a found-but-unknown binary still beats the fallback"
        );
        assert!(sel.probe.is_some());
        assert_eq!(
            sel.probe.as_ref().map(|p| p.client),
            Some(CliClient::Unknown)
        );
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
        assert!(!is_legacy_bd(
            CliClient::Bd,
            Some((MIN_SUPPORTED_BD_MAJOR, 0, 0).into())
        ));
        assert!(is_legacy_bd(
            CliClient::Bd,
            Some((MIN_SUPPORTED_BD_MAJOR - 1, 99, 99).into())
        ));
        // Minor/patch never influence the decision.
        assert!(!is_legacy_bd(
            CliClient::Bd,
            Some((MIN_SUPPORTED_BD_MAJOR, 99, 99).into())
        ));
        assert!(is_legacy_bd(
            CliClient::Bd,
            Some((MIN_SUPPORTED_BD_MAJOR - 1, 0, 0).into())
        ));
        // Version is irrelevant for non-bd clients even at 0.0.0.
        assert!(!is_legacy_bd(CliClient::Br, Some((0, 0, 0).into())));
        assert!(!is_legacy_bd(CliClient::Br, None));
        assert!(!is_legacy_bd(CliClient::Unknown, Some((0, 0, 0).into())));
    }

    #[test]
    fn cli_selection_is_legacy_follows_probe() {
        let legacy = CliSelection {
            binary: "bd".into(),
            probe: Some(probe(CliClient::Bd, Some((0, 49, 6)))),
        };
        assert!(legacy.is_legacy());
        let unparsable = CliSelection {
            binary: "bd".into(),
            probe: Some(probe(CliClient::Bd, None)),
        };
        assert!(unparsable.is_legacy());
        let ok = CliSelection {
            binary: "bd".into(),
            probe: Some(probe(CliClient::Bd, Some((1, 0, 0)))),
        };
        assert!(!ok.is_legacy());
        let br = CliSelection {
            binary: "br".into(),
            probe: Some(probe(CliClient::Br, Some((0, 1, 0)))),
        };
        assert!(!br.is_legacy());
        let none = CliSelection {
            binary: "bd".into(),
            probe: None,
        };
        assert!(!none.is_legacy());
    }

    #[test]
    fn cli_client_name_mapping() {
        assert_eq!(cli_client_name(CliClient::Bd), "bd");
        assert_eq!(cli_client_name(CliClient::Br), "br");
        assert_eq!(cli_client_name(CliClient::Unknown), "unknown");
    }
}
