//! [`BeadsError`]: the discriminated-union error every backend returns.
//!
//! `Display` reproduces, byte for byte, the `String` errors the Tauri commands
//! return today, so the IPC edge keeps its strings with `map_err(|e| e.to_string())`
//! and the frontend's prefix matches (`SCHEMA_MIGRATION_ERROR`) keep working.
//! [`BeadsError::code`] and [`BeadsError::remediation`] add a stable machine code and
//! a user-facing next step per variant.

use std::fmt;

use btit_types::CliClient;

use crate::detect::cli_client_name;

/// JSON shape a parser expected but did not find (see [`BeadsError::UnexpectedShape`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedShape {
    /// A top-level JSON array.
    Array,
    /// A top-level JSON array, or br's paginated `{"issues": [...]}` envelope.
    ArrayOrEnvelope,
}

/// What a strict deserialization was reading (see [`BeadsError::ParseFailed`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseTarget {
    /// `status --json` output (`bd_status`).
    Status,
    /// `show --json` output (`bd_show`).
    Issue,
    /// `create --json` output (`bd_create`).
    CreatedIssue,
    /// `update --json` output (`bd_update`).
    UpdatedIssue,
    /// The `show --json` fallback after an empty `update` output (`bd_update`).
    UpdatedIssueFetch,
    /// `close --json` output (`bd_close`).
    CloseResult,
    /// `search --json` output (`bd_search`).
    SearchResults,
}

/// Error returned by every beads backend operation.
///
/// Variants carry typed context; `Display` renders today's command error strings.
#[derive(Debug)]
#[non_exhaustive]
pub enum BeadsError {
    /// The CLI process could not be spawned (missing or not executable).
    Spawn {
        /// Binary name or path that was spawned.
        binary: String,
        /// First argv word of a raw invocation (`doctor`, `migrate`, …), or `None` for a JSON call.
        operation: Option<String>,
        /// The spawn error.
        source: std::io::Error,
    },
    /// The CLI ran and exited unsuccessfully.
    CommandFailed {
        /// Binary name or path that ran.
        binary: String,
        /// `ExitStatus::code()`; `None` when terminated by a signal.
        status: Option<i32>,
        /// `ExitStatus`'s `Display` text, used when `stderr` is empty.
        status_display: String,
        /// Captured standard error (lossy UTF-8).
        stderr: String,
    },
    /// The database schema is incompatible (bd 0.49.4 `no such column: spec_id`).
    SchemaMigration {
        /// Binary name or path that reported the failure.
        binary: String,
    },
    /// The CLI output is not JSON at all.
    InvalidJson {
        /// Caller-supplied label for logs (e.g. the command name).
        context: String,
        /// The parse error.
        source: serde_json::Error,
    },
    /// The CLI output is JSON with a top-level shape btit does not know.
    UnexpectedShape {
        /// Caller-supplied label for logs (e.g. the command name).
        context: String,
        /// The shape that was expected.
        expected: ExpectedShape,
    },
    /// Strict deserialization of a command's JSON output failed.
    ParseFailed {
        /// What was being parsed.
        target: ParseTarget,
        /// Issue id, when the message names one.
        id: Option<String>,
        /// The deserialization error.
        source: serde_json::Error,
    },
    /// The operation is not available for this client.
    Unsupported {
        /// Human-readable operation name.
        operation: &'static str,
        /// The client that lacks it.
        client: CliClient,
    },
}

impl BeadsError {
    /// Stable machine-readable code for this variant.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Spawn { .. } => "BTIT_BEADS_SPAWN",
            Self::CommandFailed { .. } => "BTIT_BEADS_COMMAND_FAILED",
            Self::SchemaMigration { .. } => "BTIT_BEADS_SCHEMA_MIGRATION",
            Self::InvalidJson { .. } => "BTIT_BEADS_INVALID_JSON",
            Self::UnexpectedShape { .. } => "BTIT_BEADS_UNEXPECTED_SHAPE",
            Self::ParseFailed { .. } => "BTIT_BEADS_PARSE_FAILED",
            Self::Unsupported { .. } => "BTIT_BEADS_UNSUPPORTED",
        }
    }

    /// User-facing next step for this variant.
    #[must_use]
    pub fn remediation(&self) -> &'static str {
        match self {
            Self::Spawn { .. } => {
                "Install the CLI or point Settings at its path; the searched directories are in check_bd_compatibility.searchedPaths."
            }
            Self::CommandFailed { .. } => {
                "Read stderr; run the same command in a terminal from the project directory."
            }
            Self::SchemaMigration { .. } => "Use Repair database (bd_repair_database).",
            Self::InvalidJson { .. } => {
                "Upgrade the CLI; the output is not JSON even with --json."
            }
            Self::UnexpectedShape { .. } => {
                "Upgrade the CLI; the JSON shape is not one btit knows."
            }
            Self::ParseFailed { .. } => {
                "Report the CLI version and the raw output from the log."
            }
            Self::Unsupported { .. } => "Switch the CLI binary in Settings.",
        }
    }
}

impl fmt::Display for BeadsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn {
                binary,
                operation: None,
                source,
            } => write!(f, "Failed to execute {binary}: {source}"),
            Self::Spawn {
                binary,
                operation: Some(op),
                source,
            } => write!(f, "Failed to run {binary} {op}: {source}"),
            Self::CommandFailed {
                status_display,
                stderr,
                ..
            } => {
                if stderr.is_empty() {
                    write!(f, "bd command failed with status: {status_display}")
                } else {
                    f.write_str(stderr)
                }
            }
            Self::SchemaMigration { .. } => f.write_str(
                "SCHEMA_MIGRATION_ERROR: Database schema is incompatible. Please use the repair function to fix this issue.",
            ),
            Self::InvalidJson { source, .. } => write!(f, "Invalid JSON: {source}"),
            Self::UnexpectedShape { expected, .. } => match expected {
                ExpectedShape::Array => f.write_str("Expected JSON array"),
                ExpectedShape::ArrayOrEnvelope => {
                    f.write_str("Expected JSON array or paginated envelope")
                }
            },
            Self::ParseFailed { target, id, source } => match (target, id) {
                (ParseTarget::Status, _) => write!(f, "Failed to parse status: {source}"),
                (ParseTarget::Issue, None) => write!(f, "Failed to parse issue: {source}"),
                (ParseTarget::Issue, Some(id)) => {
                    write!(f, "Failed to parse issue {id}: {source}")
                }
                (ParseTarget::CreatedIssue, _) => {
                    write!(f, "Failed to parse created issue: {source}")
                }
                (ParseTarget::UpdatedIssueFetch, _) => {
                    write!(f, "Failed to fetch updated issue: {source}")
                }
                (ParseTarget::UpdatedIssue, _) => {
                    write!(f, "Failed to parse updated issue: {source}")
                }
                (ParseTarget::CloseResult, _) => {
                    write!(f, "Failed to parse close result: {source}")
                }
                (ParseTarget::SearchResults, _) => {
                    write!(f, "Failed to parse search results: {source}")
                }
            },
            Self::Unsupported { operation, client } => write!(
                f,
                "{operation} is not supported by the {} client",
                cli_client_name(*client)
            ),
        }
    }
}

impl std::error::Error for BeadsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn { source, .. } => Some(source),
            Self::InvalidJson { source, .. } | Self::ParseFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    use std::io;

    use super::*;

    fn json_error() -> serde_json::Error {
        serde_json::from_str::<serde_json::Value>("not json").unwrap_err()
    }

    fn io_error() -> io::Error {
        io::Error::new(
            io::ErrorKind::NotFound,
            "No such file or directory (os error 2)",
        )
    }

    #[test]
    fn spawn_without_operation_matches_execute_bd() {
        let e = BeadsError::Spawn {
            binary: "bd".into(),
            operation: None,
            source: io_error(),
        };
        assert_eq!(
            e.to_string(),
            "Failed to execute bd: No such file or directory (os error 2)"
        );
        assert_eq!(e.code(), "BTIT_BEADS_SPAWN");
        assert!(e.source().is_some());
    }

    #[test]
    fn spawn_with_operation_matches_migration_raw_calls() {
        let e = BeadsError::Spawn {
            binary: "bd".into(),
            operation: Some("doctor".into()),
            source: io_error(),
        };
        assert_eq!(
            e.to_string(),
            "Failed to run bd doctor: No such file or directory (os error 2)"
        );
        assert_eq!(e.code(), "BTIT_BEADS_SPAWN");
    }

    #[test]
    fn command_failed_uses_stderr_when_present() {
        let e = BeadsError::CommandFailed {
            binary: "bd".into(),
            status: Some(1),
            status_display: "exit status: 1".into(),
            stderr: "Error: issue not found\n".into(),
        };
        assert_eq!(e.to_string(), "Error: issue not found\n");
        assert_eq!(e.code(), "BTIT_BEADS_COMMAND_FAILED");
        assert!(e.source().is_none());
    }

    #[test]
    fn command_failed_falls_back_to_status_with_literal_bd() {
        let e = BeadsError::CommandFailed {
            binary: "br".into(),
            status: Some(2),
            status_display: "exit status: 2".into(),
            stderr: String::new(),
        };
        assert_eq!(
            e.to_string(),
            "bd command failed with status: exit status: 2"
        );
        assert_eq!(e.code(), "BTIT_BEADS_COMMAND_FAILED");
    }

    #[test]
    fn schema_migration_keeps_frontend_prefix() {
        let e = BeadsError::SchemaMigration {
            binary: "bd".into(),
        };
        let s = e.to_string();
        assert!(s.starts_with("SCHEMA_MIGRATION_ERROR"));
        assert_eq!(
            s,
            "SCHEMA_MIGRATION_ERROR: Database schema is incompatible. Please use the repair function to fix this issue."
        );
        assert_eq!(e.code(), "BTIT_BEADS_SCHEMA_MIGRATION");
        assert!(e.source().is_none());
    }

    #[test]
    fn invalid_json_matches_parse_issues_tolerant() {
        let source = json_error();
        let expected = format!("Invalid JSON: {source}");
        let e = BeadsError::InvalidJson {
            context: "bd_list".into(),
            source,
        };
        assert_eq!(e.to_string(), expected);
        assert_eq!(e.code(), "BTIT_BEADS_INVALID_JSON");
        assert!(e.source().is_some());
    }

    #[test]
    fn unexpected_shape_strings() {
        let array = BeadsError::UnexpectedShape {
            context: "bd_list".into(),
            expected: ExpectedShape::Array,
        };
        assert_eq!(array.to_string(), "Expected JSON array");
        assert_eq!(array.code(), "BTIT_BEADS_UNEXPECTED_SHAPE");
        let envelope = BeadsError::UnexpectedShape {
            context: "bd_list".into(),
            expected: ExpectedShape::ArrayOrEnvelope,
        };
        assert_eq!(
            envelope.to_string(),
            "Expected JSON array or paginated envelope"
        );
        assert_eq!(envelope.code(), "BTIT_BEADS_UNEXPECTED_SHAPE");
        assert!(envelope.source().is_none());
    }

    #[test]
    fn parse_failed_strings_per_target() {
        let cases: [(ParseTarget, Option<&str>, &str); 8] = [
            (ParseTarget::Status, None, "Failed to parse status: "),
            (ParseTarget::Issue, None, "Failed to parse issue: "),
            (
                ParseTarget::Issue,
                Some("abc-1"),
                "Failed to parse issue abc-1: ",
            ),
            (
                ParseTarget::CreatedIssue,
                None,
                "Failed to parse created issue: ",
            ),
            (
                ParseTarget::UpdatedIssueFetch,
                None,
                "Failed to fetch updated issue: ",
            ),
            (
                ParseTarget::UpdatedIssue,
                None,
                "Failed to parse updated issue: ",
            ),
            (
                ParseTarget::CloseResult,
                None,
                "Failed to parse close result: ",
            ),
            (
                ParseTarget::SearchResults,
                None,
                "Failed to parse search results: ",
            ),
        ];
        for (target, id, prefix) in cases {
            let source = json_error();
            let expected = format!("{prefix}{source}");
            let e = BeadsError::ParseFailed {
                target,
                id: id.map(String::from),
                source,
            };
            assert_eq!(e.to_string(), expected, "{target:?} {id:?}");
            assert_eq!(e.code(), "BTIT_BEADS_PARSE_FAILED");
            assert!(e.source().is_some());
        }
    }

    #[test]
    fn unsupported_names_operation_and_client() {
        let e = BeadsError::Unsupported {
            operation: "Dolt repair",
            client: CliClient::Br,
        };
        assert_eq!(
            e.to_string(),
            "Dolt repair is not supported by the br client"
        );
        assert_eq!(e.code(), "BTIT_BEADS_UNSUPPORTED");
        assert!(e.source().is_none());
    }

    #[test]
    fn remediation_per_variant() {
        let cases = [
            (
                BeadsError::Spawn {
                    binary: "bd".into(),
                    operation: None,
                    source: io_error(),
                },
                "Install the CLI or point Settings at its path; the searched directories are in check_bd_compatibility.searchedPaths.",
            ),
            (
                BeadsError::CommandFailed {
                    binary: "bd".into(),
                    status: None,
                    status_display: String::new(),
                    stderr: String::new(),
                },
                "Read stderr; run the same command in a terminal from the project directory.",
            ),
            (
                BeadsError::SchemaMigration {
                    binary: "bd".into(),
                },
                "Use Repair database (bd_repair_database).",
            ),
            (
                BeadsError::InvalidJson {
                    context: String::new(),
                    source: json_error(),
                },
                "Upgrade the CLI; the output is not JSON even with --json.",
            ),
            (
                BeadsError::UnexpectedShape {
                    context: String::new(),
                    expected: ExpectedShape::Array,
                },
                "Upgrade the CLI; the JSON shape is not one btit knows.",
            ),
            (
                BeadsError::ParseFailed {
                    target: ParseTarget::Status,
                    id: None,
                    source: json_error(),
                },
                "Report the CLI version and the raw output from the log.",
            ),
            (
                BeadsError::Unsupported {
                    operation: "x",
                    client: CliClient::Unknown,
                },
                "Switch the CLI binary in Settings.",
            ),
        ];
        for (e, remediation) in cases {
            assert_eq!(e.remediation(), remediation, "{}", e.code());
        }
    }
}
