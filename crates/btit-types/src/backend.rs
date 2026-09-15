//! Backend-facing value types: project location, relation kinds, and Dolt op results.

/// Where a beads project lives. Today only the local working directory that every
/// Tauri command receives as `cwd: Option<String>`; the enum is non-exhaustive so a
/// remote transport (beads Dolt server, `DoltHub`) can add a variant without changing
/// callers that pass a `ProjectRef` through.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectRef {
    /// The local working directory, as sent by the frontend.
    Local {
        /// Project working directory; `None` defers to the process cwd / `BEADS_PATH`.
        cwd: Option<String>,
    },
}

impl ProjectRef {
    /// Build a `ProjectRef::Local` from an optional working directory.
    #[must_use]
    pub fn local(cwd: Option<String>) -> Self {
        Self::Local { cwd }
    }
}

/// A dependency relation type offered by `bd_available_relation_types`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RelationType {
    /// Wire value (e.g. "blocks").
    pub value: &'static str,
    /// Human-readable label.
    pub label: &'static str,
}

/// Transport-neutral result of a `DoltOperations` call: what `migration.rs` reads from the process
/// today (`status.success()`, `stdout.trim()`, `stderr.trim()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoltOpResult {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Trimmed standard output (the success message today).
    pub message: String,
    /// Trimmed standard error (the failure detail today).
    pub detail: String,
}
