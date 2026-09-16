//! [`BrCli`]: the `br` (`beads_rust`) backend over a `Box<dyn CliInvoker>`.

use std::sync::Arc;

use btit_beads::backend::{BeadsBackend, CliBackend, CloseSuggestions};
use btit_beads::error::BeadsError;
use btit_cli::ops;
use btit_cli::{CliInvoker, CliRunner, ProjectLocks};
use btit_types::{
    BackendCapabilities, BdRawIssue, CliClient, CliOutput, CliProbe, CliVersion, CreatePayload,
    ListQuery, ProjectRef, RelationType, ReleaseSource, UpdatePayload,
};

/// GitHub release location for `br` (`beads_rust`) (today: `updates.rs:286,290`).
pub const BR_RELEASE_SOURCE: ReleaseSource = ReleaseSource {
    api_url: "https://api.github.com/repos/Dicklesworthstone/beads_rust/releases/latest",
    releases_url: "https://github.com/Dicklesworthstone/beads_rust/releases",
};

/// The Rust `br` CLI (`beads_rust`): SQLite+JSONL only, no daemon, `--suggest-next` on close.
#[rustfmt::skip]
pub struct BrCli { inv: Box<dyn CliInvoker> }

// `CliInvoker` (`btit-cli`) is not `Debug` (it is a `Send + Sync` object-safe trait
// with no such bound), so `#[derive(Debug)]` does not compile here; this manual impl
// prints the configured binary, the one identifying fact `Debug` needs to carry.
impl std::fmt::Debug for BrCli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrCli")
            .field("binary", &self.inv.binary())
            .finish()
    }
}

impl BrCli {
    /// A backend for `binary`, spawned through a fresh [`CliRunner`] that probes
    /// `--version` on first use.
    #[must_use]
    pub fn new(binary: impl Into<String>, locks: Arc<ProjectLocks>) -> Self {
        Self {
            inv: Box::new(CliRunner::new(binary, locks)),
        }
    }

    /// `new` with the probe cache pre-seeded, so no second `--version` is spawned
    /// (b-7's factory and `check_bd_compatibility` rebuild).
    #[must_use]
    pub fn with_seeded_probe(
        binary: impl Into<String>,
        locks: Arc<ProjectLocks>,
        probe: CliProbe,
    ) -> Self {
        Self {
            inv: Box::new(CliRunner::with_probe(binary, locks, probe)),
        }
    }

    /// A backend over a caller-supplied invoker, for tests (e.g. `RecordingInvoker`).
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn with_invoker(inv: Box<dyn CliInvoker>) -> Self {
        Self { inv }
    }
}

impl BeadsBackend for BrCli {
    fn project_uses_dolt(&self, _project: &ProjectRef) -> bool {
        // br never uses Dolt (cli.rs:487).
        false
    }

    fn list(&self, project: &ProjectRef, query: &ListQuery) -> Result<Vec<BdRawIssue>, BeadsError> {
        ops::list(self.inv.as_ref(), project, query)
    }

    fn ready(&self, project: &ProjectRef) -> Result<Vec<BdRawIssue>, BeadsError> {
        ops::ready(self.inv.as_ref(), project)
    }

    fn status(&self, project: &ProjectRef) -> Result<serde_json::Value, BeadsError> {
        ops::status(self.inv.as_ref(), project)
    }

    fn show(&self, project: &ProjectRef, id: &str) -> Result<Option<BdRawIssue>, BeadsError> {
        ops::show(self.inv.as_ref(), project, id)
    }

    fn create(
        &self,
        project: &ProjectRef,
        payload: &CreatePayload,
    ) -> Result<BdRawIssue, BeadsError> {
        ops::create(self.inv.as_ref(), project, payload)
    }

    fn update(
        &self,
        project: &ProjectRef,
        id: &str,
        updates: &UpdatePayload,
    ) -> Result<Option<BdRawIssue>, BeadsError> {
        ops::update(self.inv.as_ref(), project, id, updates)
    }

    fn close(&self, project: &ProjectRef, id: &str) -> Result<serde_json::Value, BeadsError> {
        // br always requests `--suggest-next` on close (issue_commands.rs:410-413).
        self.close_suggesting_next(project, id)
    }

    fn search(&self, project: &ProjectRef, query: &str) -> Result<Vec<BdRawIssue>, BeadsError> {
        ops::search(self.inv.as_ref(), project, query)
    }

    fn label_add(&self, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError> {
        ops::label_add(self.inv.as_ref(), project, id, label)
    }

    fn label_remove(&self, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError> {
        ops::label_remove(self.inv.as_ref(), project, id, label)
    }

    fn delete(&self, project: &ProjectRef, id: &str) -> Result<(), BeadsError> {
        ops::delete(
            self.inv.as_ref(),
            project,
            id,
            self.capabilities().supports_delete_hard_flag,
        )
    }

    fn comment_add(&self, project: &ProjectRef, id: &str, content: &str) -> Result<(), BeadsError> {
        ops::comment_add(self.inv.as_ref(), project, id, content)
    }

    fn dep_add(
        &self,
        project: &ProjectRef,
        issue_id: &str,
        depends_on_id: &str,
        relation_type: Option<&str>,
    ) -> Result<(), BeadsError> {
        ops::dep_add(
            self.inv.as_ref(),
            project,
            issue_id,
            depends_on_id,
            relation_type,
        )
    }

    fn dep_remove(
        &self,
        project: &ProjectRef,
        issue_id: &str,
        depends_on_id: &str,
    ) -> Result<(), BeadsError> {
        ops::dep_remove(self.inv.as_ref(), project, issue_id, depends_on_id)
    }

    fn relation_types(&self) -> Vec<RelationType> {
        // The seven common relation types only; br has no tracks/until/validates.
        ops::relation_types(CliClient::Br)
    }

    fn sync(&self, project: &ProjectRef) -> Result<(), BeadsError> {
        ops::sync(
            self.inv.as_ref(),
            project,
            self.capabilities().supports_daemon_flag,
        )
    }

    fn cli(&self) -> Option<&dyn CliBackend> {
        Some(self)
    }

    fn close_suggestions(&self) -> Option<&dyn CloseSuggestions> {
        Some(self)
    }
}

impl CliBackend for BrCli {
    fn binary(&self) -> String {
        self.inv.binary()
    }

    fn probe(&self) -> Option<CliProbe> {
        self.inv.probe()
    }

    fn client(&self) -> CliClient {
        self.inv.client()
    }

    fn version(&self) -> Option<CliVersion> {
        self.inv.version()
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.inv.capabilities()
    }

    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> {
        self.inv.run_raw(project, args)
    }

    fn release_source(&self) -> ReleaseSource {
        BR_RELEASE_SOURCE
    }
}

impl CloseSuggestions for BrCli {
    fn close_suggesting_next(
        &self,
        project: &ProjectRef,
        id: &str,
    ) -> Result<serde_json::Value, BeadsError> {
        ops::close(self.inv.as_ref(), project, id, true)
    }
}
