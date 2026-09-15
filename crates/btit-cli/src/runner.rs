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

/// The cached state of a [`CliRunner`]'s `--version` probe.
///
/// `Unprobed` is the initial state; [`CliInvoker::client_info`] transitions it to
/// exactly one of `Failed` or `Ok` on its first call, and every later call returns
/// that cached state without spawning again. `Failed` covers a spawn failure, a
/// non-zero exit, or a successful exit whose output could not be parsed into a
/// version — all three are indistinguishable to callers, which see `None` from
/// [`client_info`](CliInvoker::client_info) either way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeState {
    /// No `--version` probe has run yet.
    Unprobed,
    /// The probe ran and did not produce a usable version; not retried until the
    /// runner is replaced (e.g. a binary change or a compatibility recheck).
    Failed,
    /// The probe ran and produced this version information.
    Ok(CliProbe),
}

/// Signature of the function that runs `<binary> --version` and returns its raw
/// output, used by [`CliRunner`] and its `test-support` seam
/// [`with_version_probe`](CliRunner::with_version_probe).
pub type VersionProbeFn = dyn Fn(&str) -> Result<CliOutput, BeadsError> + Send + Sync;

/// One configured binary. Owned by `BdCli`/`BrCli`; the app holds one at a time.
///
/// The version probe runs lazily on the first [`client_info`](CliInvoker::client_info)
/// call. Both a parsed success and a failure (spawn error, non-zero exit or
/// unparsable version) are cached: a failing binary is probed at most once per
/// runner, not once per call. See [`ProbeState`].
pub struct CliRunner {
    binary: String,
    locks: Arc<ProjectLocks>,
    probe: Mutex<ProbeState>,
    /// Runs `<binary> --version` and returns its raw output. Production runners use
    /// [`crate::run::probe_version_output`]; the `test-support` seam
    /// [`with_version_probe`](Self::with_version_probe) replaces it to count calls
    /// without spawning a process.
    version_probe: Box<VersionProbeFn>,
}

// `version_probe` is a `Box<dyn Fn>`, which is not `Debug`; this manual impl prints
// only `binary` and the cached probe state, matching `#[derive(Debug)]`'s shape for
// the fields that are meaningful to print.
impl std::fmt::Debug for CliRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliRunner")
            .field("binary", &self.binary)
            .field("probe", &*self.cached())
            .finish_non_exhaustive()
    }
}

impl CliRunner {
    /// A runner for `binary` that probes `--version` on first use.
    #[must_use]
    pub fn new(binary: impl Into<String>, locks: Arc<ProjectLocks>) -> Self {
        Self {
            binary: binary.into(),
            locks,
            probe: Mutex::new(ProbeState::Unprobed),
            version_probe: Box::new(crate::run::probe_version_output),
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
            probe: Mutex::new(ProbeState::Ok(probe)),
            version_probe: Box::new(crate::run::probe_version_output),
        }
    }

    /// A runner for `binary` whose `--version` probe is replaced by `f`.
    ///
    /// Test-only seam: lets tests count probe calls (e.g. with a captured
    /// `AtomicUsize`) and simulate spawn failures without touching a real process.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn with_version_probe(
        binary: impl Into<String>,
        locks: Arc<ProjectLocks>,
        f: Box<VersionProbeFn>,
    ) -> Self {
        Self {
            binary: binary.into(),
            locks,
            probe: Mutex::new(ProbeState::Unprobed),
            version_probe: f,
        }
    }

    fn cached(&self) -> MutexGuard<'_, ProbeState> {
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
        // spawning their own. Both a parsed success and a failure are cached, so a
        // failing binary is probed at most once per runner (B13).
        let mut cached = self.cached();
        match &*cached {
            ProbeState::Ok(p) => return Some(p.clone()),
            ProbeState::Failed => return None,
            ProbeState::Unprobed => {}
        }
        let Ok(output) = (self.version_probe)(&self.binary) else {
            *cached = ProbeState::Failed;
            return None;
        };
        if !output.success {
            log_warn!("[cli_detect] Failed to get version from {}", self.binary);
            *cached = ProbeState::Failed;
            return None;
        }
        let probe = parse_cli_probe(&output.stdout);
        if let Some(v) = probe.version {
            log_info!(
                "[cli_detect] Detected {} client v{}",
                cli_client_name(probe.client),
                v
            );
            *cached = ProbeState::Ok(probe.clone());
            Some(probe)
        } else {
            log_warn!("[cli_detect] Could not parse version from: {}", probe.raw);
            *cached = ProbeState::Failed;
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
    fn missing_binary_caches_failed_and_reports_unknown() {
        let runner = CliRunner::new(MISSING, Arc::new(ProjectLocks::new()));
        assert_eq!(runner.client_info(), None);
        assert_eq!(*runner.cached(), ProbeState::Failed);
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

    // ---- B13: failed-probe caching, via the `with_version_probe` test seam --------

    #[cfg(feature = "test-support")]
    mod probe_cache {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};

        /// A `version_probe` closure that counts its calls and returns `make_output()`
        /// on each one. A factory closure is used, rather than a stored `Result`,
        /// because `BeadsError` (via its `std::io::Error` spawn source) is not `Clone`.
        fn counting_probe(
            make_output: impl Fn() -> Result<CliOutput, BeadsError> + Send + Sync + 'static,
        ) -> (Box<VersionProbeFn>, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            let counted = Arc::clone(&calls);
            let f: Box<VersionProbeFn> = Box::new(move |_binary: &str| {
                counted.fetch_add(1, Ordering::SeqCst);
                make_output()
            });
            (f, calls)
        }

        fn spawn_error(binary: &str) -> BeadsError {
            BeadsError::Spawn {
                binary: binary.to_string(),
                operation: None,
                source: std::io::Error::other("no such binary"),
            }
        }

        #[test]
        fn probe_failure_is_cached_and_reports_unknown() {
            let (f, calls) = counting_probe(|| Err(spawn_error(MISSING)));
            let runner = CliRunner::with_version_probe(MISSING, Arc::new(ProjectLocks::new()), f);

            assert_eq!(runner.capabilities(), BackendCapabilities::default());
            assert_eq!(runner.capabilities(), BackendCapabilities::default());
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert_eq!(runner.client(), CliClient::Unknown);
            assert_eq!(runner.version(), None);
        }

        #[test]
        fn fresh_runner_probes_again() {
            let (f1, calls1) = counting_probe(|| Err(spawn_error(MISSING)));
            let locks = Arc::new(ProjectLocks::new());
            let runner1 = CliRunner::with_version_probe(MISSING, Arc::clone(&locks), f1);
            assert_eq!(runner1.client_info(), None);
            assert_eq!(calls1.load(Ordering::SeqCst), 1);

            let (f2, calls2) = counting_probe(|| Err(spawn_error(MISSING)));
            let runner2 = CliRunner::with_version_probe(MISSING, locks, f2);
            assert_eq!(runner2.client_info(), None);
            assert_eq!(calls2.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn parsed_probe_is_cached_once() {
            let (f, calls) = counting_probe(|| {
                Ok(CliOutput {
                    status: Some(0),
                    success: true,
                    stdout: "bd version 0.49.6".to_string(),
                    stderr: String::new(),
                })
            });
            let runner = CliRunner::with_version_probe("bd", Arc::new(ProjectLocks::new()), f);

            assert_eq!(
                runner.capabilities(),
                capabilities_for(CliClient::Bd, Some((0, 49, 6).into()))
            );
            assert_eq!(
                runner.capabilities(),
                capabilities_for(CliClient::Bd, Some((0, 49, 6).into()))
            );
            assert_eq!(
                runner.capabilities(),
                capabilities_for(CliClient::Bd, Some((0, 49, 6).into()))
            );
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
    }
}
