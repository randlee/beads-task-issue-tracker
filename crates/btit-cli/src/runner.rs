//! [`CliInvoker`], what [`ops`](crate::ops) needs from a CLI, and [`CliRunner`], its
//! process-spawning implementation for one configured binary.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use btit_beads::detect::{cli_client_name, parse_cli_probe};
use btit_beads::error::BeadsError;
use btit_beads::gates::capabilities_for;
use btit_beads::{log_info, log_warn};
use btit_types::{BackendCapabilities, CliClient, CliOutput, CliProbe, CliVersion, ProjectRef};

use crate::locks::ProjectLocks;

/// What `ops` needs from a CLI: identity, cached version info and the two invocation forms.
///
/// Implemented by [`CliRunner`], by the test-support `RecordingInvoker`, and, until
/// b-7, by the app's transitional `AppInvoker`. Object-safe and `Send + Sync`.
///
/// # Errors
///
/// [`run_json`](Self::run_json) and [`run_raw`](Self::run_raw) return [`BeadsError`]
/// as documented on [`run::spawn_json`](crate::run::spawn_json) and
/// [`run::run_raw`](crate::run::run_raw).
pub trait CliInvoker: Send + Sync {
    /// The configured binary name or path.
    fn binary(&self) -> String;

    /// Cached `(client, version)`; probes lazily like `get_cli_client_info` (cli.rs:326-365).
    fn client_info(&self) -> Option<CliProbe>;

    /// The five version gates for [`client_info`](Self::client_info); all `false` without one.
    fn capabilities(&self) -> BackendCapabilities {
        match self.client_info() {
            Some(p) => capabilities_for(p.client, p.version),
            None => BackendCapabilities::default(),
        }
    }

    /// The detected client; [`CliClient::Unknown`] without cached info.
    fn client(&self) -> CliClient {
        self.client_info().map_or(CliClient::Unknown, |p| p.client)
    }

    /// The detected version; `None` without cached info.
    fn version(&self) -> Option<CliVersion> {
        self.client_info().and_then(|p| p.version)
    }

    /// Fresh `<binary> --version` (`probe_cli_binary`, cli.rs:185-196); never touches the cache.
    fn probe(&self) -> Option<CliProbe>;

    /// `<binary> <command…> <args…> [--no-daemon] --json` under the project lock (`execute_bd`, cli.rs:523-592).
    ///
    /// # Errors
    ///
    /// As [`run::spawn_json`](crate::run::spawn_json), plus
    /// [`BeadsError::Unsupported`] for a non-local project.
    fn run_json(
        &self,
        project: &ProjectRef,
        command: &str,
        args: &[String],
    ) -> Result<String, BeadsError>;

    /// `<binary> <args…>` with `PATH`/`BEADS_PATH`, no `--json`, no lock.
    ///
    /// # Errors
    ///
    /// As [`run::run_raw`](crate::run::run_raw), plus [`BeadsError::Unsupported`]
    /// for a non-local project.
    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError>;
}

/// One configured binary. Owned by `BdCli`/`BrCli`; the app holds one at a time.
///
/// The version probe runs lazily on the first [`client_info`](CliInvoker::client_info)
/// call and only a parsed success is cached, so a failing binary is re-probed on
/// every call until it answers.
#[derive(Debug)]
pub struct CliRunner {
    binary: String,
    locks: Arc<ProjectLocks>,
    probe: Mutex<Option<CliProbe>>,
}

impl CliRunner {
    /// A runner for `binary` that probes `--version` on first use.
    #[must_use]
    pub fn new(binary: impl Into<String>, locks: Arc<ProjectLocks>) -> Self {
        Self {
            binary: binary.into(),
            locks,
            probe: Mutex::new(None),
        }
    }

    /// A runner for `binary` whose cache is pre-seeded with `probe`, so no `--version`
    /// is spawned until the cache is needed again (e.g. a rebuild from a fresh probe).
    #[must_use]
    pub fn with_probe(
        binary: impl Into<String>,
        locks: Arc<ProjectLocks>,
        probe: CliProbe,
    ) -> Self {
        Self {
            binary: binary.into(),
            locks,
            probe: Mutex::new(Some(probe)),
        }
    }

    fn cached(&self) -> MutexGuard<'_, Option<CliProbe>> {
        self.probe.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl CliInvoker for CliRunner {
    fn binary(&self) -> String {
        self.binary.clone()
    }

    fn probe(&self) -> Option<CliProbe> {
        crate::probe::probe_cli_binary(&self.binary)
    }

    fn client_info(&self) -> Option<CliProbe> {
        // The guard is held across the `--version` spawn on purpose, as in
        // cli.rs:326-338: concurrent callers wait for one probe instead of each
        // spawning their own. Only parsed successes are cached, so a failed probe
        // is retried by the next caller.
        let mut cached = self.cached();
        if let Some(p) = cached.as_ref() {
            return Some(p.clone());
        }
        let output = crate::run::probe_version_output(&self.binary).ok()?; // spawn failure → None
        if !output.success {
            log_warn!("[cli_detect] Failed to get version from {}", self.binary);
            return None;
        }
        let probe = parse_cli_probe(&output.stdout);
        // cache parsed successes only (cli.rs:351-360)
        if let Some(v) = probe.version {
            log_info!(
                "[cli_detect] Detected {} client v{}",
                cli_client_name(probe.client),
                v
            );
            *cached = Some(probe.clone());
            Some(probe)
        } else {
            log_warn!("[cli_detect] Could not parse version from: {}", probe.raw);
            None
        }
    }

    fn run_json(
        &self,
        project: &ProjectRef,
        command: &str,
        args: &[String],
    ) -> Result<String, BeadsError> {
        let info = self.client_info(); // one cache read (or one probe) per invocation
        let (client, full_args) = crate::run::json_invocation(info.as_ref(), command, args);
        let wd = crate::run::resolve_working_dir(project, client)?;
        crate::run::spawn_json(&self.binary, &self.locks, &wd, &full_args)
    }

    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> {
        let client = self.client_info().map_or(CliClient::Unknown, |p| p.client);
        let wd = crate::run::resolve_working_dir(project, client)?;
        crate::run::run_raw(&self.binary, &wd, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MISSING: &str = "definitely-not-a-real-cli-binary-xyz";

    fn temp_project() -> ProjectRef {
        ProjectRef::local(Some(std::env::temp_dir().to_string_lossy().to_string()))
    }

    #[test]
    fn with_probe_seeds_the_cache_and_capabilities() {
        let probe = CliProbe {
            client: CliClient::Bd,
            version: Some((0, 49, 6).into()),
            raw: "bd version 0.49.6".into(),
        };
        let runner = CliRunner::with_probe(MISSING, Arc::new(ProjectLocks::new()), probe.clone());
        assert_eq!(runner.binary(), MISSING);
        assert_eq!(runner.client_info(), Some(probe.clone()));
        assert_eq!(runner.client(), CliClient::Bd);
        assert_eq!(runner.version(), probe.version);
        assert_eq!(
            runner.capabilities(),
            capabilities_for(CliClient::Bd, probe.version)
        );
        // `probe()` spawns afresh and never reads the seeded cache.
        assert_eq!(runner.probe(), None);
        assert_eq!(runner.client_info(), Some(probe));
    }

    #[test]
    fn missing_binary_has_no_client_info_and_is_not_cached() {
        let runner = CliRunner::new(MISSING, Arc::new(ProjectLocks::new()));
        assert_eq!(runner.client_info(), None);
        assert!(runner.cached().is_none());
        assert_eq!(runner.client(), CliClient::Unknown);
        assert_eq!(runner.capabilities(), BackendCapabilities::default());
    }

    #[test]
    fn run_json_and_run_raw_report_spawn_errors() {
        let runner = CliRunner::new(MISSING, Arc::new(ProjectLocks::new()));
        let project = temp_project();
        let json = runner.run_json(&project, "ready", &[]).unwrap_err();
        assert!(matches!(
            json,
            BeadsError::Spawn {
                operation: None,
                ..
            }
        ));
        let raw = runner.run_raw(&project, &["sync"]).unwrap_err();
        assert!(matches!(raw, BeadsError::Spawn { operation: Some(op), .. } if op == "sync"));
    }

    #[test]
    fn invoker_is_object_safe_send_sync() {
        fn assert_send_sync<T: ?Sized + Send + Sync>() {}
        assert_send_sync::<dyn CliInvoker>();
        assert_send_sync::<CliRunner>();
        let runner = CliRunner::new("bd", Arc::new(ProjectLocks::new()));
        let inv: &dyn CliInvoker = &runner;
        assert_eq!(inv.binary(), "bd");
    }
}
