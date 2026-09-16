//! The b-4 authoritative argv table: every cell is an assertion here.
//!
//! Columns are the seeded probes `Bd 1.0.4`, `Bd 0.54.0`, `Bd 0.49.6`, `Br 0.1.33`,
//! `Unknown 9.9.9` and no probe. The recorded argv of a JSON call is
//! `run::json_argv(command, args, no_daemon)`; `--no-daemon`, `--all` vs the two-call
//! fallback, `--hard`, and `sync --no-daemon` are derived from
//! `btit_beads::gates::capabilities_for(client, version)` for the column's probe (and
//! `--suggest-next` from the client kind, as the backends supply it), so a gate change
//! moves the expectation with it. Three cells are also pinned as literals.

use btit_beads::gates::capabilities_for;
use btit_cli::ops;
use btit_cli::testing::RecordingInvoker;
use btit_cli::CliInvoker;
use btit_types::{
    BackendCapabilities, CliClient, CliProbe, CreatePayload, ListQuery, ProjectRef, UpdatePayload,
};

const ID: &str = "bd-1";

fn probe(client: CliClient, version: (u32, u32, u32)) -> CliProbe {
    CliProbe {
        client,
        version: Some(version.into()),
        raw: format!("{client:?} {}.{}.{}", version.0, version.1, version.2),
    }
}

/// The six columns of the table, labelled for assertion messages.
fn columns() -> Vec<(&'static str, Option<CliProbe>)> {
    vec![
        ("Bd 1.0.4", Some(probe(CliClient::Bd, (1, 0, 4)))),
        ("Bd 0.54.0", Some(probe(CliClient::Bd, (0, 54, 0)))),
        ("Bd 0.49.6", Some(probe(CliClient::Bd, (0, 49, 6)))),
        ("Br 0.1.33", Some(probe(CliClient::Br, (0, 1, 33)))),
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

/// Expected JSON argv: `words`, `--no-daemon` when the column's gate says so, `--json`.
fn json(words: &[&str], p: Option<&CliProbe>) -> Vec<String> {
    let mut v: Vec<String> = words.iter().map(ToString::to_string).collect();
    if caps(p).supports_daemon_flag {
        v.push("--no-daemon".to_string());
    }
    v.push("--json".to_string());
    v
}

fn literal(words: &[&str]) -> Vec<String> {
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

#[test]
fn list_with_filters() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone()).reply_json(Ok("[]".to_string()));
        let query = ListQuery {
            status: Some(vec!["open".to_string()]),
            issue_type: Some(vec!["bug".to_string()]),
            priority: Some(vec!["p1".to_string()]),
            assignee: Some("a".to_string()),
            include_all: None,
        };
        let issues = ops::list(&inv, &project(), &query).unwrap();
        assert!(issues.is_empty(), "{label}");
        assert_eq!(
            inv.calls(),
            vec![json(
                &[
                    "list",
                    "--status=open",
                    "--type=bug",
                    "--priority=1",
                    "--assignee=a",
                    "--limit=0"
                ],
                p.as_ref()
            )],
            "{label}"
        );
    }
}

#[test]
fn list_include_all() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone())
            .reply_json(Ok("[]".to_string()))
            .reply_json(Ok("[]".to_string()));
        let query = ListQuery {
            include_all: Some(true),
            ..ListQuery::default()
        };
        ops::list(&inv, &project(), &query).unwrap();
        let expected = if caps(p.as_ref()).supports_list_all_flag {
            vec![json(&["list", "--all", "--limit=0"], p.as_ref())]
        } else {
            vec![
                json(&["list", "--limit=0"], p.as_ref()),
                json(&["list", "--limit=0", "--status=closed"], p.as_ref()),
            ]
        };
        assert_eq!(inv.calls(), expected, "{label}");
    }
}

#[test]
fn list_include_all_literal_bd_0_54_0_two_calls() {
    let inv = RecordingInvoker::new(Some(probe(CliClient::Bd, (0, 54, 0))));
    let query = ListQuery {
        include_all: Some(true),
        ..ListQuery::default()
    };
    // Empty replies parse as invalid JSON, so feed arrays.
    let inv = inv
        .reply_json(Ok("[]".to_string()))
        .reply_json(Ok("[]".to_string()));
    ops::list(&inv, &project(), &query).unwrap();
    assert_eq!(
        inv.calls(),
        vec![
            literal(&["list", "--limit=0", "--json"]),
            literal(&["list", "--limit=0", "--status=closed", "--json"]),
        ]
    );
}

#[test]
fn ready() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone()).reply_json(Ok("[]".to_string()));
        ops::ready(&inv, &project()).unwrap();
        assert_eq!(inv.calls(), vec![json(&["ready"], p.as_ref())], "{label}");
    }
}

#[test]
fn ready_literal_bd_0_49_6() {
    let inv = RecordingInvoker::new(Some(probe(CliClient::Bd, (0, 49, 6))))
        .reply_json(Ok("[]".to_string()));
    ops::ready(&inv, &project()).unwrap();
    assert_eq!(
        inv.calls(),
        vec![literal(&["ready", "--no-daemon", "--json"])]
    );
}

#[test]
fn status() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone()).reply_json(Ok("{}".to_string()));
        ops::status(&inv, &project()).unwrap();
        assert_eq!(inv.calls(), vec![json(&["status"], p.as_ref())], "{label}");
    }
}

#[test]
fn show() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone()).reply_json(Ok(issue_json(ID)));
        assert!(
            ops::show(&inv, &project(), ID).unwrap().is_some(),
            "{label}"
        );
        assert_eq!(
            inv.calls(),
            vec![json(&["show", ID], p.as_ref())],
            "{label}"
        );
    }
}

fn full_create_payload() -> CreatePayload {
    CreatePayload {
        title: "Title".to_string(),
        description: Some("Desc".to_string()),
        issue_type: Some("bug".to_string()),
        priority: Some("p1".to_string()),
        assignee: Some("alice".to_string()),
        labels: Some(vec!["l1".to_string(), "l2".to_string()]),
        external_ref: Some("ext-1".to_string()),
        estimate_minutes: Some(30),
        design_notes: Some("design".to_string()),
        acceptance_criteria: Some("accept".to_string()),
        working_notes: Some("notes".to_string()),
        parent: Some("bd-0".to_string()),
        spec_id: Some("spec-1".to_string()),
        cwd: None,
    }
}

#[test]
fn create() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone()).reply_json(Ok(issue_json(ID)));
        ops::create(&inv, &project(), &full_create_payload()).unwrap();
        assert_eq!(
            inv.calls(),
            vec![json(
                &[
                    "create",
                    "Title",
                    "--description",
                    "Desc",
                    "--type",
                    "bug",
                    "--priority",
                    "1",
                    "--assignee",
                    "alice",
                    "--labels",
                    "l1,l2",
                    "--external-ref",
                    "ext-1",
                    "--estimate",
                    "30",
                    "--design",
                    "design",
                    "--acceptance",
                    "accept",
                    "--notes",
                    "notes",
                    "--parent",
                    "bd-0",
                    "--spec-id",
                    "spec-1",
                ],
                p.as_ref()
            )],
            "{label}"
        );
    }
}

fn full_update_payload() -> UpdatePayload {
    UpdatePayload {
        title: Some("Title".to_string()),
        description: Some("Desc".to_string()),
        issue_type: Some("bug".to_string()),
        status: Some("in_progress".to_string()),
        priority: Some("p2".to_string()),
        assignee: Some("alice".to_string()),
        labels: Some(vec!["l1".to_string(), "l2".to_string()]),
        external_ref: Some("ext-1".to_string()),
        estimate_minutes: Some(45),
        design_notes: Some("design".to_string()),
        acceptance_criteria: Some("accept".to_string()),
        working_notes: Some("notes".to_string()),
        parent: Some("bd-0".to_string()),
        metadata: Some(r#"{"k":1}"#.to_string()),
        spec_id: Some("spec-1".to_string()),
        cwd: None,
    }
}

const UPDATE_WORDS: &[&str] = &[
    "update",
    ID,
    "--title",
    "Title",
    "--description",
    "Desc",
    "--type",
    "bug",
    "--status",
    "in_progress",
    "--priority",
    "2",
    "--assignee",
    "alice",
    "--set-labels",
    "l1,l2",
    "--external-ref",
    "ext-1",
    "--estimate",
    "45",
    "--design",
    "design",
    "--acceptance",
    "accept",
    "--notes",
    "notes",
    "--metadata",
    r#"{"k":1}"#,
    "--spec-id",
    "spec-1",
    "--parent",
    "bd-0",
];

#[test]
fn update() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone()).reply_json(Ok(issue_json(ID)));
        assert!(
            ops::update(&inv, &project(), ID, &full_update_payload())
                .unwrap()
                .is_some(),
            "{label}"
        );
        assert_eq!(inv.calls(), vec![json(UPDATE_WORDS, p.as_ref())], "{label}");
    }
}

#[test]
fn update_empty_stdout_falls_back_to_show() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone())
            .reply_json(Ok(String::new()))
            .reply_json(Ok(issue_json(ID)));
        assert!(
            ops::update(&inv, &project(), ID, &full_update_payload())
                .unwrap()
                .is_some(),
            "{label}"
        );
        assert_eq!(
            inv.calls(),
            vec![
                json(UPDATE_WORDS, p.as_ref()),
                json(&["show", ID], p.as_ref())
            ],
            "{label}"
        );
    }
}

#[test]
fn close() {
    for (label, p) in columns() {
        let suggest_next = client(p.as_ref()) == CliClient::Br;
        let inv = RecordingInvoker::new(p.clone()).reply_json(Ok("{}".to_string()));
        ops::close(&inv, &project(), ID, suggest_next).unwrap();
        let words: &[&str] = if suggest_next {
            &["close", ID, "--suggest-next"]
        } else {
            &["close", ID]
        };
        assert_eq!(inv.calls(), vec![json(words, p.as_ref())], "{label}");
    }
}

#[test]
fn search() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone()).reply_json(Ok("[]".to_string()));
        ops::search(&inv, &project(), "needle").unwrap();
        assert_eq!(
            inv.calls(),
            vec![json(&["search", "needle"], p.as_ref())],
            "{label}"
        );
    }
}

#[test]
fn label_add_and_remove() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone());
        ops::label_add(&inv, &project(), ID, "urgent").unwrap();
        ops::label_remove(&inv, &project(), ID, "urgent").unwrap();
        assert_eq!(
            inv.calls(),
            vec![
                json(&["label", "add", ID, "urgent"], p.as_ref()),
                json(&["label", "remove", ID, "urgent"], p.as_ref()),
            ],
            "{label}"
        );
    }
}

#[test]
fn delete() {
    for (label, p) in columns() {
        let hard = caps(p.as_ref()).supports_delete_hard_flag;
        let inv = RecordingInvoker::new(p.clone());
        ops::delete(&inv, &project(), ID, hard).unwrap();
        let words: &[&str] = if hard {
            &["delete", ID, "--force", "--hard"]
        } else {
            &["delete", ID, "--force"]
        };
        assert_eq!(inv.calls(), vec![json(words, p.as_ref())], "{label}");
    }
}

#[test]
fn delete_literal_bd_0_49_6_hard() {
    let inv = RecordingInvoker::new(Some(probe(CliClient::Bd, (0, 49, 6))));
    ops::delete(&inv, &project(), ID, true).unwrap();
    assert_eq!(
        inv.calls(),
        vec![literal(&[
            "delete",
            ID,
            "--force",
            "--hard",
            "--no-daemon",
            "--json"
        ])]
    );
}

#[test]
fn comment_add() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone());
        ops::comment_add(&inv, &project(), ID, "hello world").unwrap();
        assert_eq!(
            inv.calls(),
            vec![json(&["comments", "add", ID, "hello world"], p.as_ref())],
            "{label}"
        );
    }
}

#[test]
fn dep_add_without_and_with_type() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone());
        ops::dep_add(&inv, &project(), "bd-a", "bd-b", None).unwrap();
        ops::dep_add(&inv, &project(), "bd-a", "bd-b", Some("tracks")).unwrap();
        assert_eq!(
            inv.calls(),
            vec![
                json(&["dep", "add", "bd-a", "bd-b"], p.as_ref()),
                json(
                    &["dep", "add", "bd-a", "bd-b", "--type", "tracks"],
                    p.as_ref()
                ),
            ],
            "{label}"
        );
    }
}

#[test]
fn dep_remove() {
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone());
        ops::dep_remove(&inv, &project(), "bd-a", "bd-b").unwrap();
        assert_eq!(
            inv.calls(),
            vec![json(&["dep", "remove", "bd-a", "bd-b"], p.as_ref())],
            "{label}"
        );
    }
}

#[test]
fn sync() {
    for (label, p) in columns() {
        let no_daemon = caps(p.as_ref()).supports_daemon_flag;
        let inv = RecordingInvoker::new(p.clone());
        ops::sync(&inv, &project(), no_daemon).unwrap();
        let words: &[&str] = if no_daemon {
            &["sync", "--no-daemon"]
        } else {
            &["sync"]
        };
        assert_eq!(inv.calls(), vec![literal(words)], "{label}");
    }
}

#[test]
fn relation_types() {
    const COMMON: &[&str] = &[
        "relates-to",
        "related",
        "discovered-from",
        "duplicates",
        "supersedes",
        "caused-by",
        "replies-to",
    ];
    for (label, p) in columns() {
        let inv = RecordingInvoker::new(p.clone());
        let values: Vec<&str> = ops::relation_types(inv.client())
            .iter()
            .map(|t| t.value)
            .collect();
        let mut expected = COMMON.to_vec();
        if client(p.as_ref()) != CliClient::Br {
            expected.extend(["tracks", "until", "validates"]);
        }
        assert_eq!(values, expected, "{label}");
        assert!(inv.calls().is_empty(), "{label}");
    }
}
