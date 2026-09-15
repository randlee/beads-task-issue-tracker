//! Compile-time pin of `btit-br`'s public API.
//!
//! Fails to compile if `BrCli::new`, `BrCli::with_seeded_probe`,
//! `BrCli::with_invoker` (feature `test-support`), or `BR_RELEASE_SOURCE` change
//! shape, or if `BrCli` stops implementing `BeadsBackend` and `CliBackend` as trait
//! objects.

use std::sync::Arc;

use btit_beads::{BeadsBackend, CliBackend};
use btit_br::{BrCli, BR_RELEASE_SOURCE};
use btit_cli::ProjectLocks;
use btit_types::{CliProbe, ReleaseSource};

#[test]
fn constructors() {
    let locks = Arc::new(ProjectLocks::new());
    // `impl Into<String>` parameters cannot be pinned as bare fn pointers (they are
    // generic per call site), so the pin is the call itself plus the trait-object
    // conversions below: a signature change here fails to compile.
    let cli: BrCli = BrCli::new("br", Arc::clone(&locks));
    let probe = CliProbe {
        client: btit_types::CliClient::Br,
        version: Some((0, 1, 33).into()),
        raw: "br 0.1.33".to_string(),
    };
    let seeded: BrCli = BrCli::with_seeded_probe("br", Arc::clone(&locks), probe);
    let _obj: &dyn BeadsBackend = &cli;
    let _cli_backend: &dyn CliBackend = &cli;
    let _seeded_obj: &dyn BeadsBackend = &seeded;
    assert_eq!(BR_RELEASE_SOURCE, RELEASE_SOURCE);
}

const RELEASE_SOURCE: ReleaseSource = ReleaseSource {
    api_url: "https://api.github.com/repos/Dicklesworthstone/beads_rust/releases/latest",
    releases_url: "https://github.com/Dicklesworthstone/beads_rust/releases",
};

#[cfg(feature = "test-support")]
#[test]
fn with_invoker_behind_test_support() {
    use btit_cli::testing::RecordingInvoker;

    let _: fn(Box<dyn btit_cli::CliInvoker>) -> BrCli = BrCli::with_invoker;
    let cli = BrCli::with_invoker(Box::new(RecordingInvoker::new(None)));
    let _obj: &dyn BeadsBackend = &cli;
}

/// Pinned to catch a new required method or a changed signature.
fn _obj(b: &BrCli) -> &dyn BeadsBackend {
    b
}

/// Pinned to catch a new required method or a changed signature.
fn _cli(b: &BrCli) -> &dyn CliBackend {
    b
}
