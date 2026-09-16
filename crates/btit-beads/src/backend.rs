//! The beads backend contract.
//!
//! [`BeadsBackend`] is transport-neutral: a future SQL or `DoltHub` transport
//! implements it alone. Everything transport- or backend-specific is reached
//! through optional accessors ([`BeadsBackend::cli`], [`BeadsBackend::dolt`],
//! [`BeadsBackend::close_suggestions`]) that default to `None`, so callers hold an
//! `Arc<dyn BeadsBackend>` and never downcast. All traits are object-safe and
//! `Send + Sync`, and synchronous (calls block for the child's lifetime, as today).
//!
//! Source citations (`file:line`) refer to the pre-split app sources at `a18c724`.

use std::path::Path;

use btit_types::{
    BackendCapabilities, BdRawIssue, CliClient, CliOutput, CliProbe, CliVersion, CreatePayload,
    DoltOpResult, ListQuery, ProjectRef, RelationType, ReleaseSource, UpdatePayload,
};

use crate::error::BeadsError;

/// Transport-neutral operations every beads backend supports. Implemented by `btit-bd` and
/// `btit-br`; a future SQL/`DoltHub` transport implements this trait alone (no CLI notion here).
///
/// # Errors
///
/// Every fallible method returns a [`BeadsError`]; its variants document the failure modes.
#[expect(
    clippy::missing_errors_doc,
    reason = "failure modes are the BeadsError variants, documented once on the trait and on BeadsError"
)]
pub trait BeadsBackend: Send + Sync {
    /// Whether the project's `.beads` is a Dolt project (today: `project_uses_dolt`, cli.rs:473-515,
    /// called with `<working_dir>/.beads` by every caller). br: always `false`.
    fn project_uses_dolt(&self, project: &ProjectRef) -> bool;

    /// `list` with filters and `--limit=0` (today: `bd_list`, issue_commands.rs:18-69; `bd_count`
    /// :81-90; `bd_poll_data`, polling.rs:43-56).
    fn list(&self, project: &ProjectRef, query: &ListQuery) -> Result<Vec<BdRawIssue>, BeadsError>;
    /// `ready` (today: `bd_ready`, issue_commands.rs:139-141).
    fn ready(&self, project: &ProjectRef) -> Result<Vec<BdRawIssue>, BeadsError>;
    /// `status` (today: `bd_status`, issue_commands.rs:149-152).
    fn status(&self, project: &ProjectRef) -> Result<serde_json::Value, BeadsError>;
    /// `show <id>`; `None` when the issue does not exist (today: `bd_show`, issue_commands.rs:162-205).
    fn show(&self, project: &ProjectRef, id: &str) -> Result<Option<BdRawIssue>, BeadsError>;
    /// `create` with the payload's flags (today: `bd_create`, issue_commands.rs:214-274).
    fn create(
        &self,
        project: &ProjectRef,
        payload: &CreatePayload,
    ) -> Result<BdRawIssue, BeadsError>;
    /// `update <id>`, falling back to `show` on empty output (today: `bd_update`, issue_commands.rs:285-394).
    fn update(
        &self,
        project: &ProjectRef,
        id: &str,
        updates: &UpdatePayload,
    ) -> Result<Option<BdRawIssue>, BeadsError>;
    /// `close <id>` (today: `bd_close`, issue_commands.rs:409-423).
    fn close(&self, project: &ProjectRef, id: &str) -> Result<serde_json::Value, BeadsError>;
    /// `search <query>` (today: `bd_search`, issue_commands.rs:433-447).
    fn search(&self, project: &ProjectRef, query: &str) -> Result<Vec<BdRawIssue>, BeadsError>;
    /// `label add <id> <label>` (today: `bd_label_add`, issue_commands.rs:455-456).
    fn label_add(&self, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError>;
    /// `label remove <id> <label>` (today: `bd_label_remove`, issue_commands.rs:463-464).
    fn label_remove(&self, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError>;
    /// `delete <id> --force [--hard]`; the caller removes the attachment folder (issue_commands.rs:480-504).
    fn delete(&self, project: &ProjectRef, id: &str) -> Result<(), BeadsError>;
    /// `comments add <id> <content>` (today: `bd_comments_add`, issue_commands.rs:511-513).
    fn comment_add(&self, project: &ProjectRef, id: &str, content: &str) -> Result<(), BeadsError>;
    /// `dep add <issue> <depends_on> [--type <t>]` (issue_commands.rs:520-522, 538-540).
    fn dep_add(
        &self,
        project: &ProjectRef,
        issue_id: &str,
        depends_on_id: &str,
        relation_type: Option<&str>,
    ) -> Result<(), BeadsError>;
    /// `dep remove <issue> <depends_on>` (today: `bd_dep_remove`, issue_commands.rs:529-531, 547-549).
    fn dep_remove(
        &self,
        project: &ProjectRef,
        issue_id: &str,
        depends_on_id: &str,
    ) -> Result<(), BeadsError>;
    /// Relation types offered to the UI (issue_commands.rs:555-581).
    fn relation_types(&self) -> Vec<RelationType>;
    /// `sync [--no-daemon]` without `--json` and without the project lock (migration.rs:222-232).
    /// A non-zero exit is `Err(CommandFailed)`; the caller keeps today's log/return text.
    fn sync(&self, project: &ProjectRef) -> Result<(), BeadsError>;

    /// CLI-only facts and raw invocations; `None` for a non-CLI transport (new accessor).
    fn cli(&self) -> Option<&dyn CliBackend> {
        None
    }
    /// Dolt operations; `None` for backends without Dolt (br today) (new accessor).
    fn dolt(&self) -> Option<&dyn DoltOperations> {
        None
    }
    /// br-only `--suggest-next`; `None` for other backends (new accessor).
    fn close_suggestions(&self) -> Option<&dyn CloseSuggestions> {
        None
    }
}

/// What only a spawned CLI has. Implemented by `btit-bd` and `btit-br`; not by remote transports.
///
/// # Errors
///
/// [`CliBackend::run_raw`] returns [`BeadsError::Spawn`] when the process cannot be started.
#[expect(
    clippy::missing_errors_doc,
    reason = "failure modes are the BeadsError variants, documented once on the trait and on BeadsError"
)]
pub trait CliBackend: BeadsBackend {
    /// Configured binary name or path (today `config::get_cli_binary`, which returns an owned `String`, config.rs:58-60).
    fn binary(&self) -> String;
    /// Fresh `<binary> --version` from the temp dir with the extended PATH (today `probe_cli_binary`, cli.rs:185-196).
    fn probe(&self) -> Option<CliProbe>;
    /// Detected client kind (today: `get_cli_client_info().0`, cli.rs:326-365); `Unknown` after a failed probe.
    fn client(&self) -> CliClient;
    /// Detected version, `None` when `--version` failed or did not parse (today: the
    /// `get_cli_client_info` tuple part, cli.rs:326-365).
    fn version(&self) -> Option<CliVersion>;
    /// The five version gates (today: `supports_*`/`uses_*` wrappers, cli.rs:380-466).
    fn capabilities(&self) -> BackendCapabilities;
    /// Raw invocation: `<binary> <args…>` in the project dir with `PATH` and `BEADS_PATH` set, no `--json`, no lock
    /// (the pattern at migration.rs:227-232, 282-288, 333-339, 403-408, 623-629, 665-671, 771-777, 879-885, 944-950, 1003-1009, 1081-1087).
    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError>;
    /// GitHub release location for `check_bd_cli_update` (updates.rs:285-292).
    fn release_source(&self) -> ReleaseSource;
}

/// Dolt operations (bd only today). Transport-neutral: returns what migration.rs reads
/// (exit status, trimmed stdout, trimmed stderr), never raw process output.
///
/// # Errors
///
/// Spawn failures are [`BeadsError::Spawn`] with `operation` set to the subcommand.
#[expect(
    clippy::missing_errors_doc,
    reason = "failure modes are the BeadsError variants, documented once on the trait and on BeadsError"
)]
pub trait DoltOperations: Send + Sync {
    /// `doctor --fix --yes` (migration.rs:333-339)
    fn doctor_fix(&self, project: &ProjectRef) -> Result<DoltOpResult, BeadsError>;
    /// `migrate --to-dolt --yes` (migration.rs:623-629)
    fn migrate_to_dolt(&self, project: &ProjectRef) -> Result<DoltOpResult, BeadsError>;
    /// `init --prefix <prefix>` (migration.rs:665-671, 771-777)
    fn init(&self, project: &ProjectRef, prefix: &str) -> Result<DoltOpResult, BeadsError>;
    /// `import -i <file>` (migration.rs:879-885)
    fn import_jsonl(&self, project: &ProjectRef, file: &Path) -> Result<DoltOpResult, BeadsError>;
}

/// br-only: `close <id> --suggest-next` (issue_commands.rs:410-413).
///
/// # Errors
///
/// Returns the same [`BeadsError`] variants as [`BeadsBackend::close`].
#[expect(
    clippy::missing_errors_doc,
    reason = "failure modes are the BeadsError variants, documented once on the trait and on BeadsError"
)]
pub trait CloseSuggestions: Send + Sync {
    /// `close <id> --suggest-next` (today: the `bd_close` br branch, issue_commands.rs:410-413).
    fn close_suggesting_next(
        &self,
        project: &ProjectRef,
        id: &str,
    ) -> Result<serde_json::Value, BeadsError>;
}
