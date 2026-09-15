//! Test-only backends and project fixtures for the `_with` cores (b-8 Deliverable 6).
//!
//! Fixtures are rule-invariant for `project_uses_dolt` (b-10 changes none of their
//! answers): a Dolt project is `.beads/.dolt/` without `metadata.json`; a SQLite
//! project has `.beads/beads.db` (and `issues.jsonl` where a body reads it) and
//! neither `metadata.json` nor `.dolt/`. Probes are `Bd 1.0.4` or `Bd 0.49.6` only.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use btit_beads::backend::{BeadsBackend, CliBackend};
use btit_beads::error::BeadsError;
use btit_cli::testing::RecordingInvoker;
use btit_cli::CliInvoker;
use btit_types::{
    BdRawIssue, CliClient, CliOutput, CliProbe, CliVersion, CreatePayload, ListQuery, ProjectRef,
    RelationType, UpdatePayload,
};

/// bd 1.x: no daemon flag, no JSONL files.
pub(crate) const BD_1_0_4: (u32, u32, u32) = (1, 0, 4);
/// bd 0.49: daemon flag and JSONL files.
pub(crate) const BD_0_49_6: (u32, u32, u32) = (0, 49, 6);

/// A parsed `bd` probe at `version`.
pub(crate) fn bd_probe(version: (u32, u32, u32)) -> CliProbe {
    CliProbe {
        client: CliClient::Bd,
        version: Some(CliVersion::from(version)),
        raw: format!("bd version {}.{}.{}", version.0, version.1, version.2),
    }
}

/// Shares a `RecordingInvoker` with the test after it is boxed into a `BdCli`.
struct Shared(Arc<RecordingInvoker>);

impl CliInvoker for Shared {
    fn binary(&self) -> String {
        self.0.binary()
    }
    fn client_info(&self) -> Option<CliProbe> {
        self.0.client_info()
    }
    fn probe(&self) -> Option<CliProbe> {
        CliInvoker::probe(self.0.as_ref())
    }
    fn run_json(
        &self,
        project: &ProjectRef,
        command: &str,
        args: &[String],
    ) -> Result<String, BeadsError> {
        self.0.run_json(project, command, args)
    }
    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> {
        self.0.run_raw(project, args)
    }
}

/// A `BdCli` over `inv` (scripted by the caller), plus the handle that reads its calls.
pub(crate) fn recording_bd(inv: RecordingInvoker) -> (btit_bd::BdCli, Arc<RecordingInvoker>) {
    let inv = Arc::new(inv);
    let backend = btit_bd::BdCli::with_invoker(Box::new(Shared(Arc::clone(&inv))));
    (backend, inv)
}

/// A `RecordingInvoker` seeded with a `bd` probe at `version`.
pub(crate) fn bd_invoker(version: (u32, u32, u32)) -> RecordingInvoker {
    RecordingInvoker::new(Some(bd_probe(version)))
}

/// A successful raw reply with `stdout`.
#[expect(clippy::unnecessary_wraps, reason = "matches reply_raw's Result<CliOutput, BeadsError> parameter shape")]
pub(crate) fn raw_ok(stdout: &str) -> Result<CliOutput, BeadsError> {
    Ok(CliOutput {
        status: Some(0),
        success: true,
        stdout: stdout.to_string(),
        stderr: String::new(),
    })
}

/// A failed raw reply (exit 1) with `stderr`.
#[expect(clippy::unnecessary_wraps, reason = "matches reply_raw's Result<CliOutput, BeadsError> parameter shape")]
pub(crate) fn raw_fail(stderr: &str) -> Result<CliOutput, BeadsError> {
    Ok(CliOutput {
        status: Some(1),
        success: false,
        stdout: String::new(),
        stderr: stderr.to_string(),
    })
}

/// A spawn failure for raw subcommand `operation` whose `io::Error` text is `nope`.
pub(crate) fn spawn_err(operation: &str) -> Result<CliOutput, BeadsError> {
    Err(BeadsError::Spawn {
        binary: "bd".to_string(),
        operation: Some(operation.to_string()),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "nope"),
    })
}

/// A backend whose Dolt answer is fixed, with an optional CLI facet and no `dolt()`.
///
/// Models the two cases no shipped backend reaches today: a non-CLI transport
/// (`cli: None`) and a Dolt project on a backend without `DoltOperations`.
pub(crate) struct FakeBackend {
    pub(crate) uses_dolt: bool,
    pub(crate) cli: Option<btit_bd::BdCli>,
}

fn unsupported<T>() -> Result<T, BeadsError> {
    Err(BeadsError::Unsupported {
        operation: "fake backend",
        client: CliClient::Unknown,
    })
}

impl BeadsBackend for FakeBackend {
    fn project_uses_dolt(&self, _project: &ProjectRef) -> bool {
        self.uses_dolt
    }
    fn list(&self, _: &ProjectRef, _: &ListQuery) -> Result<Vec<BdRawIssue>, BeadsError> {
        unsupported()
    }
    fn ready(&self, _: &ProjectRef) -> Result<Vec<BdRawIssue>, BeadsError> {
        unsupported()
    }
    fn status(&self, _: &ProjectRef) -> Result<serde_json::Value, BeadsError> {
        unsupported()
    }
    fn show(&self, _: &ProjectRef, _: &str) -> Result<Option<BdRawIssue>, BeadsError> {
        unsupported()
    }
    fn create(&self, _: &ProjectRef, _: &CreatePayload) -> Result<BdRawIssue, BeadsError> {
        unsupported()
    }
    fn update(
        &self,
        _: &ProjectRef,
        _: &str,
        _: &UpdatePayload,
    ) -> Result<Option<BdRawIssue>, BeadsError> {
        unsupported()
    }
    fn close(&self, _: &ProjectRef, _: &str) -> Result<serde_json::Value, BeadsError> {
        unsupported()
    }
    fn search(&self, _: &ProjectRef, _: &str) -> Result<Vec<BdRawIssue>, BeadsError> {
        unsupported()
    }
    fn label_add(&self, _: &ProjectRef, _: &str, _: &str) -> Result<(), BeadsError> {
        unsupported()
    }
    fn label_remove(&self, _: &ProjectRef, _: &str, _: &str) -> Result<(), BeadsError> {
        unsupported()
    }
    fn delete(&self, _: &ProjectRef, _: &str) -> Result<(), BeadsError> {
        unsupported()
    }
    fn comment_add(&self, _: &ProjectRef, _: &str, _: &str) -> Result<(), BeadsError> {
        unsupported()
    }
    fn dep_add(&self, _: &ProjectRef, _: &str, _: &str, _: Option<&str>) -> Result<(), BeadsError> {
        unsupported()
    }
    fn dep_remove(&self, _: &ProjectRef, _: &str, _: &str) -> Result<(), BeadsError> {
        unsupported()
    }
    fn relation_types(&self) -> Vec<RelationType> {
        Vec::new()
    }
    fn sync(&self, _: &ProjectRef) -> Result<(), BeadsError> {
        unsupported()
    }
    fn cli(&self) -> Option<&dyn CliBackend> {
        self.cli.as_ref().map(|c| c as &dyn CliBackend)
    }
}

/// A unique temp project directory with a `.beads/` subdirectory, removed on drop.
pub(crate) struct TempProject {
    root: PathBuf,
}

impl TempProject {
    /// `<tmp>/btit_b8_<tag>_<pid>_<n>_<nanos>/.beads/`.
    pub(crate) fn new(tag: &str) -> Result<Self, String> {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "btit_b8_{tag}_{}_{}_{nanos}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let project = Self { root };
        std::fs::create_dir_all(project.beads_dir()).map_err(|e| e.to_string())?;
        Ok(project)
    }

    /// A Dolt project: `.beads/.dolt/` and no `metadata.json`.
    pub(crate) fn dolt(tag: &str) -> Result<Self, String> {
        let project = Self::new(tag)?;
        std::fs::create_dir_all(project.beads_dir().join(".dolt")).map_err(|e| e.to_string())?;
        Ok(project)
    }

    /// A SQLite project: `.beads/beads.db`, no `metadata.json`, no `.dolt/`.
    pub(crate) fn sqlite(tag: &str) -> Result<Self, String> {
        let project = Self::new(tag)?;
        project.write(".beads/beads.db", "")?;
        Ok(project)
    }

    /// The project root as the `cwd` string commands receive.
    #[expect(clippy::unnecessary_wraps, reason = "mirrors the Option<String> cwd shape every command call site passes")]
    pub(crate) fn cwd(&self) -> Option<String> {
        Some(self.root.to_string_lossy().into_owned())
    }

    /// `<root>/.beads`.
    pub(crate) fn beads_dir(&self) -> PathBuf {
        self.root.join(".beads")
    }

    /// Writes `content` to `relative` under the project root.
    pub(crate) fn write(&self, relative: &str, content: &str) -> Result<(), String> {
        std::fs::write(self.root.join(relative), content).map_err(|e| e.to_string())
    }

    /// Whether `relative` exists under the project root.
    pub(crate) fn exists(&self, relative: &str) -> bool {
        self.root.join(relative).exists()
    }

    /// `<root>/.beads/<name>` as the string a raw argv carries.
    pub(crate) fn beads_arg(&self, name: &str) -> String {
        self.beads_dir().join(name).to_string_lossy().into_owned()
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
