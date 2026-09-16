//! Full-method parity: `BrCli` delegates every `BeadsBackend`/`CliBackend`/
//! `CloseSuggestions` method to the wrapped `CliInvoker`, recording exactly the argv
//! the b-4 Required Work table's `Br 0.1.33` column (and, for `no probe`, the column
//! documented in sprint b-6) says it should.
//!
//! `BrCli::with_invoker` takes ownership of a `Box<dyn CliInvoker>`, so these tests
//! wrap the scripted `RecordingInvoker` in an `Arc` behind a local `CliInvoker`
//! adapter ([`ArcInvoker`]): the adapter is boxed into `BrCli`, and the `Arc` clone
//! kept by the test still reads `calls()` afterward.
//!
//! `BrCli::with_invoker` only exists with the crate's `test-support` feature, so this
//! whole file is gated on it.

#![cfg(feature = "test-support")]

use std::sync::Arc;

use btit_beads::error::BeadsError;
use btit_beads::gates::capabilities_for;
use btit_beads::{BeadsBackend, CliBackend};
use btit_br::{BrCli, BR_RELEASE_SOURCE};
use btit_cli::testing::RecordingInvoker;
use btit_cli::CliInvoker;
use btit_types::{
    CliClient, CliOutput, CliProbe, CreatePayload, ListQuery, ProjectRef, ReleaseSource,
    UpdatePayload,
};

const ID: &str = "br-1";

/// Adapts a shared `RecordingInvoker` to `CliInvoker`, so the test can keep reading
/// `calls()` after handing a `Box<dyn CliInvoker>` to `BrCli::with_invoker`.
struct ArcInvoker(Arc<RecordingInvoker>);

impl CliInvoker for ArcInvoker {
    fn binary(&self) -> String {
        self.0.binary()
    }
    fn client_info(&self) -> Option<CliProbe> {
        self.0.client_info()
    }
    fn probe(&self) -> Option<CliProbe> {
        self.0.probe()
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

fn project() -> ProjectRef {
    ProjectRef::local(Some("/projects/demo".to_string()))
}

fn issue_json(id: &str) -> String {
    format!(
        r#"{{"id":"{id}","title":"T","description":null,"status":"open","priority":1,"issue_type":"bug","owner":null,"assignee":null,"labels":[],"created_at":"2025-01-01T00:00:00Z","created_by":null,"updated_at":"2025-01-01T00:00:00Z","closed_at":null,"close_reason":null,"blocked_by":null,"blocks":null,"comments":null,"external_ref":null,"estimate":null,"design":null,"acceptance_criteria":null,"notes":null,"parent":null,"dependents":null,"dependencies":null,"dependency_count":null,"dependent_count":null,"metadata":null,"spec_id":null,"comment_count":null}}"#
    )
}

fn minimal_create() -> CreatePayload {
    CreatePayload {
        title: "Title".to_string(),
        description: None,
        issue_type: None,
        priority: None,
        assignee: None,
        labels: None,
        external_ref: None,
        estimate_minutes: None,
        design_notes: None,
        acceptance_criteria: None,
        working_notes: None,
        parent: None,
        spec_id: None,
        cwd: None,
    }
}

fn minimal_update() -> UpdatePayload {
    UpdatePayload {
        title: Some("Title".to_string()),
        description: None,
        issue_type: None,
        status: None,
        priority: None,
        assignee: None,
        labels: None,
        external_ref: None,
        estimate_minutes: None,
        design_notes: None,
        acceptance_criteria: None,
        working_notes: None,
        parent: None,
        metadata: None,
        spec_id: None,
        cwd: None,
    }
}

fn words(v: &[&str]) -> Vec<String> {
    v.iter().map(ToString::to_string).collect()
}

/// Builds a `BrCli` over a fresh `RecordingInvoker` seeded with `probe`, queued with
/// one reply per argv-producing `BeadsBackend`/`CloseSuggestions` method (in call
/// order below), and returns `(cli, rec)` so the test can call every method once and
/// then assert `rec.calls()`.
fn seeded(probe: Option<CliProbe>) -> (BrCli, Arc<RecordingInvoker>) {
    let rec = Arc::new(
        RecordingInvoker::new(probe)
            .reply_json(Ok("[]".to_string())) // list
            .reply_json(Ok("[]".to_string())) // ready
            .reply_json(Ok("{}".to_string())) // status
            .reply_json(Ok(issue_json(ID))) // show
            .reply_json(Ok(issue_json(ID))) // create
            .reply_json(Ok(issue_json(ID))) // update
            .reply_json(Ok("{}".to_string())) // close
            .reply_json(Ok("[]".to_string())), // search
    );
    let cli = BrCli::with_invoker(Box::new(ArcInvoker(Arc::clone(&rec))));
    (cli, rec)
}

/// Every argv-producing method, called once, in the order [`seeded`] queued replies
/// for. Not itself a `#[test]` fn, so it propagates instead of unwrapping; the
/// callers (which are `#[test]` fns, where `clippy.toml`'s
/// `allow-unwrap-in-tests` applies) unwrap the result.
fn call_every_method(cli: &BrCli) -> Result<(), BeadsError> {
    let p = project();
    cli.list(&p, &ListQuery::default())?;
    cli.ready(&p)?;
    cli.status(&p)?;
    cli.show(&p, ID)?;
    cli.create(&p, &minimal_create())?;
    cli.update(&p, ID, &minimal_update())?;
    cli.close(&p, ID)?;
    cli.search(&p, "needle")?;
    cli.label_add(&p, ID, "urgent")?;
    cli.label_remove(&p, ID, "urgent")?;
    cli.delete(&p, ID)?;
    cli.comment_add(&p, ID, "hello")?;
    cli.dep_add(&p, "br-a", "br-b", None)?;
    cli.dep_remove(&p, "br-a", "br-b")?;
    cli.sync(&p)?;
    Ok(())
}

fn expected_argv() -> Vec<Vec<String>> {
    vec![
        words(&["list", "--limit=0", "--json"]),
        words(&["ready", "--json"]),
        words(&["status", "--json"]),
        words(&["show", ID, "--json"]),
        words(&["create", "Title", "--json"]),
        words(&["update", ID, "--title", "Title", "--json"]),
        words(&["close", ID, "--suggest-next", "--json"]),
        words(&["search", "needle", "--json"]),
        words(&["label", "add", ID, "urgent", "--json"]),
        words(&["label", "remove", ID, "urgent", "--json"]),
        words(&["delete", ID, "--force", "--json"]),
        words(&["comments", "add", ID, "hello", "--json"]),
        words(&["dep", "add", "br-a", "br-b", "--json"]),
        words(&["dep", "remove", "br-a", "br-b", "--json"]),
        words(&["sync"]), // run_raw: no --json, no --no-daemon (supports_daemon_flag false)
    ]
}

fn br_0_1_33() -> CliProbe {
    CliProbe {
        client: CliClient::Br,
        version: Some((0, 1, 33).into()),
        raw: "br 0.1.33".to_string(),
    }
}

#[test]
fn full_method_parity_br_0_1_33() {
    let (cli, rec) = seeded(Some(br_0_1_33()));
    call_every_method(&cli).unwrap();
    assert_eq!(rec.calls(), expected_argv());
}

#[test]
fn full_method_parity_no_probe() {
    // No-probe column: every row is the same as Br 0.1.33 except `list(include_all)`
    // (covered separately below); `close` still gets `--suggest-next` because the
    // flag is br's, not version-gated.
    let (cli, rec) = seeded(None);
    call_every_method(&cli).unwrap();
    assert_eq!(rec.calls(), expected_argv());
}

#[test]
fn list_include_all_uses_all_flag_for_br_0_1_33() {
    let probe = Some(br_0_1_33());
    assert!(
        capabilities_for(CliClient::Br, probe.as_ref().and_then(|p| p.version))
            .supports_list_all_flag
    );
    let rec = Arc::new(RecordingInvoker::new(probe).reply_json(Ok("[]".to_string())));
    let cli = BrCli::with_invoker(Box::new(ArcInvoker(Arc::clone(&rec))));
    let query = ListQuery {
        include_all: Some(true),
        ..ListQuery::default()
    };
    cli.list(&project(), &query).unwrap();
    assert_eq!(
        rec.calls(),
        vec![words(&["list", "--all", "--limit=0", "--json"])]
    );
}

#[test]
fn list_include_all_no_probe_falls_back_to_two_calls() {
    assert!(!capabilities_for(CliClient::Unknown, None).supports_list_all_flag);
    let rec = Arc::new(
        RecordingInvoker::new(None)
            .reply_json(Ok("[]".to_string()))
            .reply_json(Ok("[]".to_string())),
    );
    let cli = BrCli::with_invoker(Box::new(ArcInvoker(Arc::clone(&rec))));
    let query = ListQuery {
        include_all: Some(true),
        ..ListQuery::default()
    };
    cli.list(&project(), &query).unwrap();
    assert_eq!(
        rec.calls(),
        vec![
            words(&["list", "--limit=0", "--json"]),
            words(&["list", "--limit=0", "--status=closed", "--json"]),
        ]
    );
}

#[test]
fn project_uses_dolt_is_always_false_even_with_a_dolt_marker() {
    let dir = std::env::temp_dir().join(format!("btit-br-parity-{}", std::process::id()));
    std::fs::create_dir_all(dir.join(".beads")).unwrap();
    std::fs::write(dir.join(".beads").join(".dolt"), b"marker").unwrap();
    let cli = BrCli::with_invoker(Box::new(ArcInvoker(Arc::new(RecordingInvoker::new(Some(
        br_0_1_33(),
    ))))));
    let p = ProjectRef::local(Some(dir.to_string_lossy().to_string()));
    assert!(!cli.project_uses_dolt(&p));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn relation_types_is_exactly_the_seven_common_entries_in_order() {
    let cli = BrCli::with_invoker(Box::new(ArcInvoker(Arc::new(RecordingInvoker::new(Some(
        br_0_1_33(),
    ))))));
    let values: Vec<&str> = cli.relation_types().iter().map(|t| t.value).collect();
    assert_eq!(
        values,
        vec![
            "relates-to",
            "related",
            "discovered-from",
            "duplicates",
            "supersedes",
            "caused-by",
            "replies-to",
        ]
    );
}

#[test]
fn capabilities_matches_gates_for_br_0_1_33() {
    let cli = BrCli::with_invoker(Box::new(ArcInvoker(Arc::new(RecordingInvoker::new(Some(
        br_0_1_33(),
    ))))));
    assert_eq!(
        cli.capabilities(),
        capabilities_for(CliClient::Br, Some((0, 1, 33).into()))
    );
}

#[test]
fn accessors_report_cli_and_close_suggestions_but_no_dolt() {
    let cli = BrCli::with_invoker(Box::new(ArcInvoker(Arc::new(RecordingInvoker::new(Some(
        br_0_1_33(),
    ))))));
    assert!(cli.cli().is_some());
    assert!(cli.dolt().is_none());
    assert!(cli.close_suggestions().is_some());
}

#[test]
fn release_source_is_the_beads_rust_repository() {
    let cli = BrCli::with_invoker(Box::new(ArcInvoker(Arc::new(RecordingInvoker::new(Some(
        br_0_1_33(),
    ))))));
    assert_eq!(cli.release_source(), BR_RELEASE_SOURCE);
    assert_eq!(
        BR_RELEASE_SOURCE,
        ReleaseSource {
            api_url: "https://api.github.com/repos/Dicklesworthstone/beads_rust/releases/latest",
            releases_url: "https://github.com/Dicklesworthstone/beads_rust/releases",
        }
    );
}
