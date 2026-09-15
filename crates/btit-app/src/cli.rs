//! Transitional shims for the modules b-8 rewires. Each is one call into the backend slot.

use crate::backend;
use btit_types::{BackendCapabilities, CliClient, CliVersion, ProjectRef};

pub(crate) use btit_beads::detect::{detect_cli_client, parse_bd_version};

/// Whether the project owning `beads_dir` uses Dolt; callers pass `<working_dir>/.beads`.
pub(crate) fn project_uses_dolt(beads_dir: &std::path::Path) -> bool {
    let cwd = beads_dir.parent().map(|p| p.to_string_lossy().into_owned());
    backend::current().project_uses_dolt(&ProjectRef::local(cwd))
}

fn caps() -> BackendCapabilities {
    backend::with_cli("capabilities", |c| c.capabilities()).unwrap_or_default()
}

pub(crate) fn supports_daemon_flag() -> bool {
    caps().supports_daemon_flag
}

pub(crate) fn uses_jsonl_files() -> bool {
    caps().uses_jsonl_files
}

/// The slot's detected `(client, version)`; `(Unknown, None)` without a parsed probe.
pub(crate) fn client_info() -> (CliClient, Option<CliVersion>) {
    backend::with_cli("client info", |c| (c.client(), c.version()))
        .unwrap_or((CliClient::Unknown, None))
}
