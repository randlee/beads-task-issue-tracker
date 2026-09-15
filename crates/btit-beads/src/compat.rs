//! Compatibility warnings shown for the detected CLI client and version.

use btit_types::{CliClient, CliVersion};

use crate::detect::MIN_SUPPORTED_BD_MAJOR;

/// Human-readable compatibility warnings for a detected client. Pure.
#[must_use]
#[expect(
    clippy::uninlined_format_args,
    reason = "moved verbatim from btit-app in b-3; behaviour and body edits belong to b-9"
)]
pub fn cli_compatibility_warnings(client: CliClient, version: Option<CliVersion>) -> Vec<String> {
    let version: Option<(u32, u32, u32)> = version.map(Into::into);
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

#[cfg(test)]
#[expect(
    clippy::uninlined_format_args,
    reason = "tests moved verbatim from btit-app in b-3; bodies change only for CliVersion conversions"
)]
mod tests {
    use super::*;

    #[test]
    fn warnings_empty_for_supported_bd() {
        assert!(cli_compatibility_warnings(CliClient::Bd, Some((1, 0, 4).into())).is_empty());
        assert!(cli_compatibility_warnings(CliClient::Bd, Some((3, 1, 0).into())).is_empty());
    }

    #[test]
    fn warnings_flag_legacy_bd() {
        let w = cli_compatibility_warnings(CliClient::Bd, Some((0, 49, 6).into()));
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("0.49.6"));
        assert!(w[0].contains("legacy"));
        assert!(w[0].contains(&format!("bd {}.x", MIN_SUPPORTED_BD_MAJOR)));

        let w = cli_compatibility_warnings(CliClient::Bd, Some((0, 56, 0).into()));
        assert_eq!(w.len(), 2, "0.50-0.56 also gets the server-mode note");
        assert!(w[1].contains("Dolt"));
    }

    #[test]
    fn warnings_for_br_unknown_and_unparsable() {
        let w = cli_compatibility_warnings(CliClient::Br, Some((0, 1, 33).into()));
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
    fn warnings_for_0_49_x_have_no_dolt_note() {
        for v in [(0, 49, 0), (0, 49, 6), (0, 49, 99)] {
            let w = cli_compatibility_warnings(CliClient::Bd, Some(v.into()));
            assert_eq!(w.len(), 1, "{v:?}: {w:?}");
            assert!(
                w[0].contains(&format!("bd {}.{}.{}", v.0, v.1, v.2)),
                "{w:?}"
            );
            assert!(!w.iter().any(|m| m.contains("Dolt")), "{w:?}");
        }
    }

    #[test]
    fn warnings_for_0_50_through_0_56_include_dolt_note() {
        for v in [(0, 50, 0), (0, 53, 2), (0, 56, 9), (0, 99, 0)] {
            let w = cli_compatibility_warnings(CliClient::Bd, Some(v.into()));
            assert_eq!(w.len(), 2, "{v:?}: {w:?}");
            assert!(
                w[0].contains(&format!("bd {}.{}.{}", v.0, v.1, v.2)),
                "{w:?}"
            );
            assert!(w[0].contains("legacy"), "{w:?}");
            assert!(w[1].contains("Dolt"), "{w:?}");
            assert!(w[1].contains("polling"), "{w:?}");
        }
    }

    #[test]
    fn warnings_mention_supported_floor_for_unparsable_bd() {
        let w = cli_compatibility_warnings(CliClient::Bd, None);
        assert_eq!(w.len(), 1);
        assert!(
            w[0].contains(&format!("bd {}.x", MIN_SUPPORTED_BD_MAJOR)),
            "{w:?}"
        );
    }

    #[test]
    fn warnings_for_br_and_unknown_ignore_version() {
        assert_eq!(cli_compatibility_warnings(CliClient::Br, None).len(), 1);
        assert_eq!(
            cli_compatibility_warnings(CliClient::Br, Some((9, 9, 9).into())).len(),
            1
        );
        assert_eq!(
            cli_compatibility_warnings(CliClient::Unknown, Some((1, 0, 0).into())).len(),
            1
        );
        assert_eq!(
            cli_compatibility_warnings(CliClient::Unknown, None).len(),
            1
        );
    }
}
