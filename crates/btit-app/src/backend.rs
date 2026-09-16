//! The app's backend slot: the one `Arc<dyn BeadsBackend>` every beads command runs through.
//!
//! [`install`] builds the backend for the configured binary at startup, [`replace`]
//! rebuilds it when Settings changes the binary, and [`current`] is the only place a
//! command obtains a backend (so a per-project resolver, OQ-8, would change only this
//! module). CLI-only facts are reached through [`with_cli`]. The factory selects
//! `BrCli` for a `br` probe with a parsed version and `BdCli` otherwise, which is the
//! rule the former cached client info applied (`cli.rs:361-364` at `a18c724`).
//!
//! This is the last process-global piece of CLI state; the library crates hold none.
//! `check_bd_compatibility` lives here because it re-detects the client and may
//! rebuild the slot.

use std::sync::{Arc, LazyLock, PoisonError, RwLock};

use btit_beads::backend::{BeadsBackend, CliBackend};
use btit_beads::compat::cli_compatibility_warnings;
use btit_beads::detect::{cli_client_name, is_legacy_bd, MIN_SUPPORTED_BD_MAJOR};
use btit_beads::error::BeadsError;
use btit_cli::locks::ProjectLocks;
use btit_cli::path::extended_path_entries;
use btit_cli::probe::probe_cli_binary;
use btit_types::{CliClient, CliProbe, CompatibilityInfo};

use crate::config::get_cli_binary;

/// Binary the slot is built for when a command runs before `setup` installed one
/// (the configured-binary default the app has always started from).
const DEFAULT_BINARY: &str = "bd";

/// One lock map for the whole process: replacing the backend never allows two
/// concurrent CLI processes on one project (bd's embedded Dolt crashes on that).
static PROJECT_LOCKS: LazyLock<Arc<ProjectLocks>> =
    LazyLock::new(|| Arc::new(ProjectLocks::default()));

/// The process's backend slot.
static SLOT: Slot = Slot::new();

/// A replaceable backend behind a `RwLock`.
///
/// Interior mutability is required: Tauri commands are free functions with no app
/// state parameter for the backend (their signatures are a frozen contract), and the
/// backend must be swappable at runtime by `set_cli_binary_path` and
/// `check_bd_compatibility`. Reads (every command) vastly outnumber writes (startup,
/// Settings changes), hence `RwLock`. The type exists so tests use their own
/// instance instead of the process global.
struct Slot {
    inner: RwLock<Option<Arc<dyn BeadsBackend>>>,
}

impl Slot {
    const fn new() -> Self {
        Self {
            inner: RwLock::new(None),
        }
    }

    /// The installed backend, or a backend for [`DEFAULT_BINARY`] when none is installed yet.
    fn current(&self) -> Arc<dyn BeadsBackend> {
        if let Some(b) = self
            .inner
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
        {
            return Arc::clone(b);
        }
        // Probe outside the lock (it spawns), then fill the slot only if it is still
        // empty, so a concurrent `install` from `setup` is never overwritten.
        let fallback = build_backend_with(DEFAULT_BINARY, probe_cli_binary(DEFAULT_BINARY));
        let mut slot = self.inner.write().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(slot.get_or_insert(fallback))
    }

    /// Runs `f` on the current backend's CLI facet.
    fn with_cli<T>(
        &self,
        operation: &'static str,
        f: impl FnOnce(&dyn CliBackend) -> T,
    ) -> Result<T, BeadsError> {
        let backend = self.current();
        backend.cli().map(f).ok_or(BeadsError::Unsupported {
            operation,
            client: CliClient::Unknown,
        })
    }

    /// The `check_bd_compatibility` rule for `binary`, given the `fresh` probe taken for it.
    ///
    /// Rebuilds the slot from `fresh` only when it parsed and its `(client, version)`
    /// differs from the slot's; the capability fields are then the slot's values.
    fn compatibility(&self, binary: String, fresh: Option<&CliProbe>) -> CompatibilityInfo {
        // A `None` probe is (Unknown, None).
        let (found, version_string, client, version) = match fresh {
            Some(p) => (true, p.raw.clone(), p.client, p.version),
            None => (
                false,
                format!("{binary} not found"),
                CliClient::Unknown,
                None,
            ),
        };

        let mut warnings = if found {
            cli_compatibility_warnings(client, version)
        } else {
            vec![format!(
                "{binary} was not found on PATH or is not executable. Install bd {MIN_SUPPORTED_BD_MAJOR}.x or point Settings at the binary."
            )]
        };
        if found && client == CliClient::Unknown {
            warnings.push(format!("--version output was: {version_string}"));
        }

        // Keep the slot coherent with what was just observed: only a parsed probe
        // rebuilds it, only when it differs, and seeded with that probe (no second spawn).
        if let (Some(p), Some(_)) = (fresh, version) {
            let slot_now = self.with_cli("compat", |c| (c.client(), c.version())).ok();
            if slot_now != Some((p.client, p.version)) {
                // `false` when a concurrent `replace` changed the binary: the newer backend wins.
                let _swapped = self.install_with_if(&binary, Some(p.clone()));
            }
        }

        let caps = self
            .with_cli("compat", |c| c.capabilities())
            .unwrap_or_default();

        CompatibilityInfo {
            binary,
            found,
            version: version_string,
            client_type: cli_client_name(client).to_string(),
            version_tuple: version.map(|v| vec![v.major, v.minor, v.patch]),
            legacy: is_legacy_bd(client, version),
            min_supported_major: MIN_SUPPORTED_BD_MAJOR,
            supports_daemon_flag: caps.supports_daemon_flag,
            uses_jsonl_files: caps.uses_jsonl_files,
            uses_dolt_backend: caps.uses_dolt_backend,
            supports_list_all_flag: caps.supports_list_all_flag,
            searched_paths: extended_path_entries(),
            warnings,
        }
    }

    /// Replaces the slot's content with a backend built from `probe`.
    fn install_with(&self, binary: &str, probe: Option<CliProbe>) {
        let backend = build_backend_with(binary, probe);
        *self.inner.write().unwrap_or_else(PoisonError::into_inner) = Some(backend);
    }

    /// Replaces the slot's content only while it still holds `expected_binary`.
    ///
    /// The check and the swap happen under one write lock, so a `replace` that
    /// landed after `probe` was taken for `expected_binary` is never overwritten by a
    /// backend built for the old binary. Returns whether the swap happened.
    fn install_with_if(&self, expected_binary: &str, probe: Option<CliProbe>) -> bool {
        let mut slot = self.inner.write().unwrap_or_else(PoisonError::into_inner);
        let same = slot
            .as_ref()
            .and_then(|b| b.cli().map(|c| c.binary() == expected_binary))
            .unwrap_or(false);
        if same {
            *slot = Some(build_backend_with(expected_binary, probe));
        }
        same
    }
}

/// Builds the backend for `binary` from an already-taken `probe`.
///
/// `br` with a parsed version selects `BrCli`; everything else (`bd`, an unknown
/// client, a `br` banner whose version did not parse, or no answer) selects `BdCli`.
/// A parsed probe seeds the runner's cache; otherwise the runner probes lazily.
fn build_backend_with(binary: &str, probe: Option<CliProbe>) -> Arc<dyn BeadsBackend> {
    let locks = Arc::clone(&PROJECT_LOCKS);
    match probe {
        Some(p) if p.client == CliClient::Br && p.version.is_some() => {
            Arc::new(btit_br::BrCli::with_seeded_probe(binary, locks, p))
        }
        Some(p) if p.version.is_some() => {
            Arc::new(btit_bd::BdCli::with_seeded_probe(binary, locks, p))
        }
        _ => Arc::new(btit_bd::BdCli::new(binary, locks)),
    }
}

/// Probes `binary` once and builds its backend (see [`build_backend_with`]).
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the named factory of the backend contract (plan \"Backend selection state\"); install keeps its probe for startup logging, so only tests call it today"
    )
)]
pub(crate) fn build_backend(binary: &str) -> Arc<dyn BeadsBackend> {
    build_backend_with(binary, probe_cli_binary(binary))
}

/// Probes `binary` once, installs its backend, and returns the probe for startup logging.
///
/// The probe runs from the temp dir so bd never auto-migrates a project in the cwd.
pub(crate) fn install(binary: &str) -> Option<CliProbe> {
    let probe = probe_cli_binary(binary);
    install_with(binary, probe.clone());
    probe
}

/// Installs the backend built from an already-taken `probe`.
pub(crate) fn install_with(binary: &str, probe: Option<CliProbe>) {
    SLOT.install_with(binary, probe);
}

/// Rebuilds the slot for a newly configured `binary` (`set_cli_binary_path`).
pub(crate) fn replace(binary: &str) {
    let _probe = install(binary);
}

/// The backend every command runs through.
pub(crate) fn current() -> Arc<dyn BeadsBackend> {
    SLOT.current()
}

/// Runs `f` on the current backend's CLI facet.
///
/// # Errors
///
/// [`BeadsError::Unsupported`] when the backend has no CLI (a non-CLI transport;
/// unreachable with `BdCli`/`BrCli`).
pub(crate) fn with_cli<T>(
    operation: &'static str,
    f: impl FnOnce(&dyn CliBackend) -> T,
) -> Result<T, BeadsError> {
    SLOT.with_cli(operation, f)
}

/// Logs the `[startup]` block from the probe `install` took (no further spawn).
pub(crate) fn log_startup(binary: &str, probe: Option<&CliProbe>) {
    match probe {
        Some(p) => {
            log::info!(
                "[startup] {} found: {} ({})",
                binary,
                p.raw,
                cli_client_name(p.client)
            );
            for w in cli_compatibility_warnings(p.client, p.version) {
                log::warn!("[startup] {w}");
            }
        }
        None => {
            log::error!(
                "[startup] {} not found or not executable. Searched: {}",
                binary,
                extended_path_entries().join(if cfg!(windows) { "; " } else { ":" })
            );
        }
    }
}

#[tauri::command]
pub(crate) async fn check_bd_compatibility() -> CompatibilityInfo {
    let binary = get_cli_binary();
    let fresh = probe_cli_binary(&binary); // one spawn, as before
    SLOT.compatibility(binary, fresh.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use btit_beads::gates::capabilities_for;
    use btit_cli::testing::RecordingInvoker;
    use btit_cli::CliInvoker;
    use btit_types::{BackendCapabilities, CliOutput, CliVersion, ProjectRef};

    /// Never on PATH, so a lazily probing backend built for it spawns nothing useful.
    const MISSING: &str = "definitely-not-a-real-cli-binary-b7";

    fn probe(client: CliClient, version: Option<(u32, u32, u32)>) -> CliProbe {
        CliProbe {
            client,
            version: version.map(CliVersion::from),
            raw: format!("{} version", cli_client_name(client)),
        }
    }

    /// Shares a `RecordingInvoker` with the test after it is boxed into a backend.
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

    /// A slot holding a `RecordingInvoker`-backed `BdCli` for `binary` seeded with `seed`.
    fn recording_slot(
        binary: &str,
        seed: Option<CliProbe>,
    ) -> (Slot, Arc<RecordingInvoker>, Arc<dyn BeadsBackend>) {
        let inv = Arc::new(RecordingInvoker::new(seed).with_binary(binary));
        let backend: Arc<dyn BeadsBackend> = Arc::new(btit_bd::BdCli::with_invoker(Box::new(
            Shared(Arc::clone(&inv)),
        )));
        let slot = Slot::new();
        *slot.inner.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&backend));
        (slot, inv, backend)
    }

    fn is_br(backend: &Arc<dyn BeadsBackend>) -> bool {
        backend.close_suggestions().is_some() && backend.dolt().is_none()
    }

    fn is_bd(backend: &Arc<dyn BeadsBackend>) -> bool {
        backend.dolt().is_some() && backend.close_suggestions().is_none()
    }

    fn binary_of(backend: &Arc<dyn BeadsBackend>) -> Option<String> {
        backend.cli().map(CliBackend::binary)
    }

    fn compat_caps(info: &CompatibilityInfo) -> (bool, bool, bool, bool) {
        (
            info.supports_daemon_flag,
            info.uses_jsonl_files,
            info.uses_dolt_backend,
            info.supports_list_all_flag,
        )
    }

    fn caps_tuple(caps: BackendCapabilities) -> (bool, bool, bool, bool) {
        (
            caps.supports_daemon_flag,
            caps.uses_jsonl_files,
            caps.uses_dolt_backend,
            caps.supports_list_all_flag,
        )
    }

    // ---- Factory (Deliverable 2) -------------------------------------------------

    #[test]
    fn factory_br_with_parsed_version_selects_br_cli() {
        let seed = probe(CliClient::Br, Some((0, 1, 33)));
        let backend = build_backend_with(MISSING, Some(seed.clone()));
        assert!(is_br(&backend));
        let cli = backend.cli();
        assert_eq!(cli.map(CliBackend::client), Some(CliClient::Br));
        assert_eq!(cli.and_then(CliBackend::version), seed.version);
    }

    #[test]
    fn factory_br_without_parsed_version_selects_bd_cli() {
        let backend = build_backend_with(MISSING, Some(probe(CliClient::Br, None)));
        assert!(is_bd(&backend));
        assert_eq!(binary_of(&backend).as_deref(), Some(MISSING));
    }

    #[test]
    fn factory_bd_with_parsed_version_selects_seeded_bd_cli() {
        let seed = probe(CliClient::Bd, Some((1, 0, 4)));
        let backend = build_backend_with(MISSING, Some(seed.clone()));
        assert!(is_bd(&backend));
        // Seeded: the runner answers from the cache although MISSING cannot be spawned.
        assert_eq!(backend.cli().and_then(CliBackend::version), seed.version);
    }

    #[test]
    fn factory_unknown_client_selects_bd_cli() {
        for version in [None, Some((2, 3, 4))] {
            let backend = build_backend_with(MISSING, Some(probe(CliClient::Unknown, version)));
            assert!(is_bd(&backend));
        }
    }

    #[test]
    fn factory_without_probe_selects_bd_cli() {
        assert!(is_bd(&build_backend_with(MISSING, None)));
        assert!(is_bd(&build_backend(MISSING)));
    }

    // ---- Slot --------------------------------------------------------------------

    #[test]
    fn with_cli_reads_the_installed_backend() {
        let slot = Slot::new();
        slot.install_with(MISSING, Some(probe(CliClient::Bd, Some((1, 0, 4)))));
        let facts = slot.with_cli("test", |c| (c.binary(), c.client()));
        assert!(matches!(facts, Ok((ref b, CliClient::Bd)) if b == MISSING));
    }

    #[test]
    fn install_with_if_swaps_only_for_the_expected_binary() {
        let (slot, _inv, before) =
            recording_slot("bd", Some(probe(CliClient::Bd, Some((1, 0, 4)))));
        assert!(!slot.install_with_if("other", Some(probe(CliClient::Br, Some((0, 1, 33))))));
        assert!(Arc::ptr_eq(&slot.current(), &before));
        assert!(slot.install_with_if("bd", Some(probe(CliClient::Br, Some((0, 1, 33))))));
        assert!(is_br(&slot.current()));
    }

    #[test]
    fn replace_between_fresh_probe_and_rebuild_is_not_overwritten() {
        let (slot, _inv, _before) =
            recording_slot("bd", Some(probe(CliClient::Bd, Some((1, 0, 4)))));
        // Fresh probe taken for `bd` reports br …
        let fresh = probe(CliClient::Br, Some((0, 1, 33)));
        // … then Settings replaces the binary before the rebuild runs.
        slot.install_with(MISSING, None);
        assert!(!slot.install_with_if("bd", Some(fresh)));
        assert_eq!(binary_of(&slot.current()).as_deref(), Some(MISSING));
        assert!(is_bd(&slot.current()));
    }

    // ---- Startup (Deliverable 3) -------------------------------------------------

    #[test]
    fn log_startup_spawns_no_probe() {
        let seed = probe(CliClient::Bd, Some((1, 0, 4)));
        let (slot, inv, _backend) = recording_slot("bd", Some(seed.clone()));
        log_startup("bd", Some(&seed));
        log_startup("bd", None);
        assert_eq!(inv.probe_calls(), 0);
        // The slot is untouched and still answers from its seeded cache.
        assert!(matches!(
            slot.with_cli("test", |c| c.client()),
            Ok(CliClient::Bd)
        ));
        assert_eq!(inv.probe_calls(), 0);
    }

    // ---- check_bd_compatibility (Deliverable 4) ----------------------------------

    #[test]
    fn compatibility_without_fresh_probe_keeps_slot_and_reports_not_found() {
        let seed = probe(CliClient::Bd, Some((1, 0, 4)));
        let (slot, inv, before) = recording_slot("bd", Some(seed.clone()));
        let info = slot.compatibility("bd".into(), None);
        assert!(Arc::ptr_eq(&slot.current(), &before));
        assert!(!info.found);
        assert_eq!(info.client_type, "unknown");
        assert_eq!(info.version_tuple, None);
        assert_eq!(info.version, "bd not found");
        // Capability fields are the slot's, never all-false by construction.
        assert_eq!(
            compat_caps(&info),
            caps_tuple(capabilities_for(seed.client, seed.version))
        );
        assert_eq!(inv.probe_calls(), 0);
    }

    #[test]
    fn compatibility_with_unparsed_version_keeps_slot() {
        let (slot, _inv, before) =
            recording_slot("bd", Some(probe(CliClient::Bd, Some((1, 0, 4)))));
        let info = slot.compatibility("bd".into(), Some(&probe(CliClient::Bd, None)));
        assert!(Arc::ptr_eq(&slot.current(), &before));
        assert!(info.found);
        assert_eq!(info.version_tuple, None);
    }

    #[test]
    fn compatibility_with_changed_client_rebuilds_slot_as_br() {
        let (slot, _inv, before) =
            recording_slot("bd", Some(probe(CliClient::Bd, Some((1, 0, 4)))));
        let fresh = probe(CliClient::Br, Some((0, 1, 33)));
        let info = slot.compatibility("bd".into(), Some(&fresh));
        let after = slot.current();
        assert!(!Arc::ptr_eq(&after, &before));
        assert!(is_br(&after));
        assert_eq!(binary_of(&after).as_deref(), Some("bd"));
        assert_eq!(info.client_type, "br");
        assert_eq!(info.version_tuple, Some(vec![0, 1, 33]));
        assert_eq!(
            compat_caps(&info),
            caps_tuple(capabilities_for(fresh.client, fresh.version))
        );
    }

    #[test]
    fn compatibility_with_unchanged_client_keeps_slot() {
        let seed = probe(CliClient::Bd, Some((1, 0, 4)));
        let (slot, _inv, before) = recording_slot("bd", Some(seed.clone()));
        let info = slot.compatibility("bd".into(), Some(&seed));
        assert!(Arc::ptr_eq(&slot.current(), &before));
        assert_eq!(info.version_tuple, Some(vec![1, 0, 4]));
        assert!(!info.legacy);
        assert_eq!(
            compat_caps(&info),
            caps_tuple(capabilities_for(seed.client, seed.version))
        );
    }

    #[test]
    fn compatibility_capabilities_match_a_seeded_probe() {
        let seed = probe(CliClient::Bd, Some((0, 49, 6)));
        let slot = Slot::new();
        slot.install_with(MISSING, Some(seed.clone()));
        let info = slot.compatibility(MISSING.into(), Some(&seed));
        assert!(info.legacy);
        assert_eq!(
            compat_caps(&info),
            caps_tuple(capabilities_for(seed.client, seed.version))
        );
    }
}
