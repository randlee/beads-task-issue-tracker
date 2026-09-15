//! CLI client identity, version and probe/capability types.

use serde::Serialize;

/// Which beads CLI answered `--version`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CliClient {
    /// Go `bd` (steveyegge/beads), the primary CLI.
    Bd,
    /// Rust `br` (`beads_rust`), secondary.
    Br,
    /// `--version` output not recognized.
    Unknown,
}

/// Parsed `major.minor.patch` of a CLI. Replaces the `(u32, u32, u32)` tuple at crate boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CliVersion {
    /// Major version component.
    pub major: u32,
    /// Minor version component.
    pub minor: u32,
    /// Patch version component.
    pub patch: u32,
}

impl From<(u32, u32, u32)> for CliVersion {
    fn from((major, minor, patch): (u32, u32, u32)) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl From<CliVersion> for (u32, u32, u32) {
    fn from(v: CliVersion) -> Self {
        (v.major, v.minor, v.patch)
    }
}

impl std::fmt::Display for CliVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Result of running `<bin> --version` on a candidate CLI binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliProbe {
    /// The detected CLI client.
    pub client: CliClient,
    /// The parsed version, or `None` when it could not be parsed.
    pub version: Option<CliVersion>,
    /// Trimmed first line of `--version` output, for logs and the UI.
    pub raw: String,
}

/// The five version-gated capabilities (today the `supports_*`/`uses_*` wrappers).
#[expect(
    clippy::struct_excessive_bools,
    reason = "Five independent, orthogonal capability flags, not app state: a state machine or enum encoding would be less readable than the plain struct every call site expects."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BackendCapabilities {
    /// True when the CLI accepts a `--no-daemon` flag.
    pub supports_daemon_flag: bool,
    /// True when the CLI stores issues as `issues.jsonl` files.
    pub uses_jsonl_files: bool,
    /// True when the CLI stores issues in an embedded Dolt database.
    pub uses_dolt_backend: bool,
    /// True when `bd list --all` returns correct results.
    pub supports_list_all_flag: bool,
    /// True when `bd delete --hard` is a supported flag.
    pub supports_delete_hard_flag: bool,
}

/// GitHub release location used by `check_bd_cli_update`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSource {
    /// GitHub API URL for the latest release.
    pub api_url: &'static str,
    /// Human-facing releases page URL.
    pub releases_url: &'static str,
}

/// Raw process result for un-JSON'd CLI invocations (`CliBackend::run_raw`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    /// `ExitStatus::code()`; `None` when terminated by a signal.
    pub status: Option<i32>,
    /// Whether the process exited successfully.
    pub success: bool,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
}

/// Snapshot of CLI compatibility, returned to the frontend by `check_bd_compatibility`.
#[expect(
    clippy::struct_excessive_bools,
    reason = "The frontend's `CompatibilityInfo` DTO wire shape is pinned (Deliverable 2); the bool fields are independent flags the UI reads individually, not app state."
)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityInfo {
    /// Configured binary name or path (e.g. "bd").
    pub binary: String,
    /// False when `binary --version` could not be run.
    pub found: bool,
    /// Raw `--version` output, or a "not found" message.
    pub version: String,
    /// "bd", "br", or "unknown"
    pub client_type: String,
    /// `[major, minor, patch]`, or `None` when the version could not be parsed.
    pub version_tuple: Option<Vec<u32>>,
    /// True when a bd below the supported floor is in use.
    pub legacy: bool,
    /// The minimum supported `bd` major version.
    pub min_supported_major: u32,
    /// True when the CLI accepts a `--no-daemon` flag.
    pub supports_daemon_flag: bool,
    /// True when the CLI stores issues as `issues.jsonl` files.
    pub uses_jsonl_files: bool,
    /// True when the CLI stores issues in an embedded Dolt database.
    pub uses_dolt_backend: bool,
    /// True when `bd list --all` returns correct results.
    pub supports_list_all_flag: bool,
    /// Directories searched when resolving the binary (for the "not found" UI).
    pub searched_paths: Vec<String>,
    /// Human-readable compatibility warnings.
    pub warnings: Vec<String>,
}
