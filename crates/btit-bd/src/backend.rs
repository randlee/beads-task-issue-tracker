//! [`BdCli`]: the backend for the Go `bd` CLI (steveyegge/beads), and for an
//! unrecognized client, which shares every `_ =>` arm with bd today.

use std::path::Path;
use std::sync::Arc;

use btit_beads::backend::{BeadsBackend, CliBackend, DoltOperations};
use btit_beads::error::BeadsError;
use btit_cli::locks::ProjectLocks;
use btit_cli::ops;
use btit_cli::run::resolve_working_dir;
use btit_cli::runner::{CliInvoker, CliRunner};
use btit_types::{
    BackendCapabilities, BdRawIssue, CliClient, CliOutput, CliProbe, CliVersion, CreatePayload,
    DoltOpResult, ListQuery, ProjectRef, RelationType, ReleaseSource, UpdatePayload,
};

/// GitHub location of `bd` releases (`updates.rs:287,291`), used by the app's
/// `check_bd_cli_update` selection (b-8).
pub const BD_RELEASE_SOURCE: ReleaseSource = ReleaseSource {
    api_url: "https://api.github.com/repos/steveyegge/beads/releases/latest",
    releases_url: "https://github.com/steveyegge/beads/releases",
};

/// The Go `bd` CLI (also used for an unrecognized client, which shares bd's defaults
/// today), over a `Box<dyn CliInvoker>`: a [`CliRunner`] in production, a scripted
/// `RecordingInvoker` in tests (feature `test-support`).
#[rustfmt::skip]
pub struct BdCli { inv: Box<dyn CliInvoker> }

impl std::fmt::Debug for BdCli {
    /// `CliInvoker` is not `Debug` (it must stay object-safe across a test-support
    /// scripted implementation too), so this reports only the configured binary.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BdCli")
            .field("binary", &self.inv.binary())
            .finish_non_exhaustive()
    }
}

impl BdCli {
    /// A `BdCli` wrapping a fresh [`CliRunner`] for `binary`, which probes
    /// `--version` lazily on first use.
    #[must_use]
    pub fn new(binary: impl Into<String>, locks: Arc<ProjectLocks>) -> Self {
        Self {
            inv: Box::new(CliRunner::new(binary, locks)),
        }
    }

    /// A `BdCli` whose probe cache is pre-seeded with `probe`, so no second
    /// `--version` is spawned (b-7's factory and `check_bd_compatibility`'s rebuild).
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

    /// A `BdCli` over any [`CliInvoker`], for tests (e.g. a scripted
    /// `RecordingInvoker`); the app's tests (b-8) use this too.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn with_invoker(inv: Box<dyn CliInvoker>) -> Self {
        Self { inv }
    }
}

impl BeadsBackend for BdCli {
    fn project_uses_dolt(&self, project: &ProjectRef) -> bool {
        // One `client_info()` read feeds both the client and the version tuple: an
        // unparsed or missing binary is re-probed on every read, and this runs on every
        // poll tick (QA-1 RSH-001, b-7).
        let info = self.inv.client_info();
        let client = info.as_ref().map_or(CliClient::Unknown, |p| p.client);
        let tuple = info.and_then(|p| p.version.map(|v| (p.client, v.major, v.minor, v.patch)));
        match resolve_working_dir(project, client) {
            Ok(wd) => crate::dolt::project_uses_dolt_for(tuple, &Path::new(&wd).join(".beads")),
            Err(_) => false,
        }
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
        ops::close(self.inv.as_ref(), project, id, false)
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
        ops::relation_types(self.client())
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

    fn dolt(&self) -> Option<&dyn DoltOperations> {
        Some(self)
    }
}

impl CliBackend for BdCli {
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
        BD_RELEASE_SOURCE
    }
}

impl DoltOperations for BdCli {
    fn doctor_fix(&self, project: &ProjectRef) -> Result<DoltOpResult, BeadsError> {
        self.inv
            .run_raw(project, &crate::dolt::doctor_fix_args())
            .map(crate::dolt::to_dolt_result)
    }

    fn migrate_to_dolt(&self, project: &ProjectRef) -> Result<DoltOpResult, BeadsError> {
        self.inv
            .run_raw(project, &crate::dolt::migrate_to_dolt_args())
            .map(crate::dolt::to_dolt_result)
    }

    fn init(&self, project: &ProjectRef, prefix: &str) -> Result<DoltOpResult, BeadsError> {
        self.inv
            .run_raw(project, &crate::dolt::init_args(prefix))
            .map(crate::dolt::to_dolt_result)
    }

    fn import_jsonl(&self, project: &ProjectRef, file: &Path) -> Result<DoltOpResult, BeadsError> {
        let file = file.to_string_lossy();
        self.inv
            .run_raw(project, &crate::dolt::import_args(&file))
            .map(crate::dolt::to_dolt_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Moved from btit-app/src/cli.rs (b-5 Deliverable 5) ---------------------
    //
    // Exercises the wrapper end to end: `BdCli::project_uses_dolt` still spawns
    // `bd --version` through a fresh `CliRunner` (B10 fixes that in b-10, not here).

    #[test]
    fn project_uses_dolt_false_without_beads_dir() {
        let temp_dir = std::env::temp_dir().join(format!(
            "beads_test_no_beads_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&temp_dir);

        let result = BdCli::new("bd", Arc::new(ProjectLocks::new())).project_uses_dolt(
            &ProjectRef::local(Some(temp_dir.to_string_lossy().to_string())),
        );
        assert!(!result);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
