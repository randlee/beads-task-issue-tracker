//! Compile-time pin of `btit-bd`'s public API.
//!
//! Fails to compile if `BdCli`'s constructors, `BD_RELEASE_SOURCE`,
//! `project_uses_dolt_for`, `to_dolt_result`, or `BdCli`'s `BeadsBackend`/
//! `CliBackend` object-safety change.

use std::path::Path;
use std::sync::Arc;

use btit_bd::dolt::to_dolt_result;
use btit_bd::{project_uses_dolt_for, BdCli, BD_RELEASE_SOURCE};
use btit_beads::backend::{BeadsBackend, CliBackend};
use btit_cli::locks::ProjectLocks;
use btit_types::{CliClient, CliOutput, CliProbe, DoltOpResult, ReleaseSource};

fn assert_send_sync<T: ?Sized + Send + Sync>() {}

type ProjectUsesDoltForFn = fn(Option<(CliClient, u32, u32, u32)>, &Path) -> bool;

#[test]
fn constructors_and_release_source() {
    assert_send_sync::<BdCli>();

    let locks = Arc::new(ProjectLocks::new());
    let _new: BdCli = BdCli::new("bd", Arc::clone(&locks));

    let probe = CliProbe {
        client: CliClient::Bd,
        version: Some((1, 0, 4).into()),
        raw: "bd version 1.0.4".to_string(),
    };
    let _seeded: BdCli = BdCli::with_seeded_probe("bd", locks, probe);

    assert_eq!(
        BD_RELEASE_SOURCE,
        ReleaseSource {
            api_url: "https://api.github.com/repos/steveyegge/beads/releases/latest",
            releases_url: "https://github.com/steveyegge/beads/releases",
        }
    );
}

#[test]
fn dolt_helper_signatures() {
    let _: ProjectUsesDoltForFn = project_uses_dolt_for;
    let _: fn(CliOutput) -> DoltOpResult = to_dolt_result;
}

fn _obj(b: &BdCli) -> &dyn BeadsBackend {
    b
}

fn _cli(b: &BdCli) -> &dyn CliBackend {
    b
}

#[cfg(feature = "test-support")]
#[test]
fn with_invoker_signature() {
    use btit_cli::runner::CliInvoker;
    use btit_cli::testing::RecordingInvoker;

    let _: fn(Box<dyn CliInvoker>) -> BdCli = BdCli::with_invoker;
    let cli = BdCli::with_invoker(Box::new(RecordingInvoker::new(None)));
    let _: &dyn BeadsBackend = &cli;
}
