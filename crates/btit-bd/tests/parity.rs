//! Full-method parity: for each seeded probe, every `BeadsBackend` and
//! `DoltOperations` method is called once through [`BdCli::with_invoker`] over a
//! [`RecordingInvoker`], and the recorded argv is checked against an expectation
//! built independently from [`capabilities_for`] rather than written as literals, so
//! a gate change moves the expectation with it.
//!
//! `BdCli::with_invoker` only exists with the crate's `test-support` feature, so
//! this whole file is a no-op binary without it (`cargo test -p btit-bd` still
//! passes; `cargo test -p btit-bd --features test-support` runs it).
#![cfg(feature = "test-support")]

use std::sync::Arc;

use btit_bd::{BdCli, BD_RELEASE_SOURCE};
use btit_beads::backend::BeadsBackend;
use btit_beads::error::BeadsError;
use btit_beads::gates::capabilities_for;
use btit_cli::runner::CliInvoker;
use btit_cli::testing::RecordingInvoker;
use btit_types::{
    BackendCapabilities, CliClient, CliOutput, CliProbe, CreatePayload, ListQuery, ProjectRef,
    RelationType, UpdatePayload,
};

const ID: &str = "bd-1";

/// Delegates to a shared `RecordingInvoker` so the test can both hand `BdCli` a
/// `Box<dyn CliInvoker>` and keep reading `calls()` on the same instance afterward.
struct Shared(Arc<RecordingInvoker>);

impl CliInvoker for Shared {
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

fn probe(client: CliClient, version: (u32, u32, u32)) -> CliProbe {
    CliProbe {
        client,
        version: Some(version.into()),
        raw: format!("{client:?} {}.{}.{}", version.0, version.1, version.2),
    }
}

/// The five columns: the probes Deliverable 6 names, plus the no-probe case.
fn columns() -> Vec<(&'static str, Option<CliProbe>)> {
    vec![
        ("Bd 1.0.4", Some(probe(CliClient::Bd, (1, 0, 4)))),
        ("Bd 0.54.0", Some(probe(CliClient::Bd, (0, 54, 0)))),
        ("Bd 0.49.6", Some(probe(CliClient::Bd, (0, 49, 6)))),
        ("Unknown 9.9.9", Some(probe(CliClient::Unknown, (9, 9, 9)))),
        ("no probe", None),
    ]
}

fn caps(p: Option<&CliProbe>) -> BackendCapabilities {
    p.map_or_else(BackendCapabilities::default, |p| {
        capabilities_for(p.client, p.version)
    })
}

fn client(p: Option<&CliProbe>) -> CliClient {
    p.map_or(CliClient::Unknown, |p| p.client)
}

/// Expected JSON-call argv: `words`, `--no-daemon` when the column's gate says so, `--json`.
fn json(words: &[&str], p: Option<&CliProbe>) -> Vec<String> {
    let mut v: Vec<String> = words.iter().map(ToString::to_string).collect();
    if caps(p).supports_daemon_flag {
        v.push("--no-daemon".to_string());
    }
    v.push("--json".to_string());
    v
}

/// Expected raw-call argv: exactly `words`, no flags added.
fn raw(words: &[&str]) -> Vec<String> {
    words.iter().map(ToString::to_string).collect()
}

fn project() -> ProjectRef {
    ProjectRef::local(Some("/projects/demo".to_string()))
}

fn issue_json(id: &str) -> String {
    format!(
        r#"{{"id":"{id}","title":"T","description":null,"status":"open","priority":1,"issue_type":"bug","owner":null,"assignee":null,"labels":[],"created_at":"2025-01-01T00:00:00Z","created_by":null,"updated_at":"2025-01-01T00:00:00Z","closed_at":null,"close_reason":null,"blocked_by":null,"blocks":null,"comments":null,"external_ref":null,"estimate":null,"design":null,"acceptance_criteria":null,"notes":null,"parent":null,"dependents":null,"dependencies":null,"dependency_count":null,"dependent_count":null,"metadata":null,"spec_id":null,"comment_count":null}}"#
    )
}

fn minimal_create_payload() -> CreatePayload {
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

fn minimal_update_payload() -> UpdatePayload {
    UpdatePayload {
        title: Some("New Title".to_string()),
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

const TEN_RELATION_TYPES: &[&str] = &[
    "relates-to",
    "related",
    "discovered-from",
    "duplicates",
    "supersedes",
    "caused-by",
    "replies-to",
    "tracks",
    "until",
    "validates",
];

#[test]
fn full_method_parity_per_probe() {
    for (label, p) in columns() {
        let recorder = Arc::new(
            RecordingInvoker::new(p.clone())
                // Consumed in call order by every run_json call below. The six ops
                // that parse their output (list, ready, status, create, update,
                // close) are called first so these six replies land on them; the
                // rest fall through to the default empty-success reply.
                .reply_json(Ok("[]".to_string())) // list
                .reply_json(Ok("[]".to_string())) // ready
                .reply_json(Ok("{}".to_string())) // status
                .reply_json(Ok(issue_json(ID))) // create
                .reply_json(Ok(issue_json(ID))) // update
                .reply_json(Ok("{}".to_string())), // close
        );
        let cli = BdCli::with_invoker(Box::new(Shared(Arc::clone(&recorder))));
        let proj = project();

        // ---- BeadsBackend -----------------------------------------------------
        assert!(
            cli.list(&proj, &ListQuery::default()).unwrap().is_empty(),
            "{label}"
        );
        assert!(cli.ready(&proj).unwrap().is_empty(), "{label}");
        cli.status(&proj).unwrap();
        cli.create(&proj, &minimal_create_payload()).unwrap();
        assert!(
            cli.update(&proj, ID, &minimal_update_payload())
                .unwrap()
                .is_some(),
            "{label}"
        );
        cli.close(&proj, ID).unwrap();
        assert!(cli.show(&proj, ID).unwrap().is_none(), "{label}");
        assert!(cli.search(&proj, "needle").unwrap().is_empty(), "{label}");
        cli.label_add(&proj, ID, "urgent").unwrap();
        cli.label_remove(&proj, ID, "urgent").unwrap();
        cli.delete(&proj, ID).unwrap();
        cli.comment_add(&proj, ID, "hello").unwrap();
        cli.dep_add(&proj, "bd-a", "bd-b", Some("tracks")).unwrap();
        cli.dep_remove(&proj, "bd-a", "bd-b").unwrap();

        let relation_values: Vec<&str> = cli
            .relation_types()
            .iter()
            .map(|t: &RelationType| t.value)
            .collect();
        assert_eq!(relation_values, TEN_RELATION_TYPES, "{label}");

        cli.sync(&proj).unwrap();

        assert!(cli.cli().is_some(), "{label}");
        assert!(cli.dolt().is_some(), "{label}");
        assert!(cli.close_suggestions().is_none(), "{label}");

        // ---- CliBackend ---------------------------------------------------
        let backend = cli.cli().expect("cli() is Some");
        assert_eq!(backend.binary(), "bd", "{label}");
        assert_eq!(backend.probe(), p.clone(), "{label}");
        assert_eq!(backend.client(), client(p.as_ref()), "{label}");
        assert_eq!(
            backend.version(),
            p.as_ref().and_then(|p| p.version),
            "{label}"
        );
        assert_eq!(backend.capabilities(), caps(p.as_ref()), "{label}");
        assert_eq!(backend.release_source(), BD_RELEASE_SOURCE, "{label}");

        // ---- DoltOperations -------------------------------------------------
        let dolt = cli.dolt().expect("dolt() is Some");
        dolt.doctor_fix(&proj).unwrap();
        dolt.migrate_to_dolt(&proj).unwrap();
        dolt.init(&proj, "proj").unwrap();
        dolt.import_jsonl(&proj, std::path::Path::new("issues.jsonl"))
            .unwrap();

        // ---- Expected argv, in call order ----------------------------------
        let hard = caps(p.as_ref()).supports_delete_hard_flag;
        let delete_words: &[&str] = if hard {
            &["delete", ID, "--force", "--hard"]
        } else {
            &["delete", ID, "--force"]
        };
        let no_daemon = caps(p.as_ref()).supports_daemon_flag;
        let sync_words: &[&str] = if no_daemon {
            &["sync", "--no-daemon"]
        } else {
            &["sync"]
        };

        let mut expected: Vec<Vec<String>> = vec![
            json(&["list", "--limit=0"], p.as_ref()),
            json(&["ready"], p.as_ref()),
            json(&["status"], p.as_ref()),
            json(&["create", "Title"], p.as_ref()),
            json(&["update", ID, "--title", "New Title"], p.as_ref()),
            json(&["close", ID], p.as_ref()),
            json(&["show", ID], p.as_ref()),
            json(&["search", "needle"], p.as_ref()),
            json(&["label", "add", ID, "urgent"], p.as_ref()),
            json(&["label", "remove", ID, "urgent"], p.as_ref()),
            json(delete_words, p.as_ref()),
            json(&["comments", "add", ID, "hello"], p.as_ref()),
            json(
                &["dep", "add", "bd-a", "bd-b", "--type", "tracks"],
                p.as_ref(),
            ),
            json(&["dep", "remove", "bd-a", "bd-b"], p.as_ref()),
        ];
        // `run_json` and `run_raw` share one `calls` vector, in call order: the raw
        // calls (sync, then the four Dolt operations) are appended after the JSON ones.
        expected.push(raw(sync_words));
        expected.push(raw(&["doctor", "--fix", "--yes"]));
        expected.push(raw(&["migrate", "--to-dolt", "--yes"]));
        expected.push(raw(&["init", "--prefix", "proj"]));
        expected.push(raw(&["import", "-i", "issues.jsonl"]));

        assert_eq!(recorder.calls(), expected, "{label}");
    }
}
