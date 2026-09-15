//! Behaviour of the `ops` bodies beyond their argv: not-found and shape handling,
//! fallbacks, result merging and error mapping, through `RecordingInvoker`.

use std::io;

use btit_beads::error::{BeadsError, ParseTarget};
use btit_cli::ops;
use btit_cli::testing::RecordingInvoker;
use btit_cli::CliInvoker;
use btit_types::{CliClient, CliOutput, CliProbe, ListQuery, ProjectRef, UpdatePayload};

/// The seeded probe `Bd 1.0.4`, in the `Option` form `RecordingInvoker::new` takes.
#[expect(
    clippy::unnecessary_wraps,
    reason = "returns the Option<CliProbe> that RecordingInvoker::new takes"
)]
fn bd1() -> Option<CliProbe> {
    Some(CliProbe {
        client: CliClient::Bd,
        version: Some((1, 0, 4).into()),
        raw: "bd version 1.0.4".to_string(),
    })
}

fn project() -> ProjectRef {
    ProjectRef::local(None)
}

fn issue_json(id: &str, status: &str) -> String {
    format!(
        r#"{{"id":"{id}","title":"T","description":null,"status":"{status}","priority":1,"issue_type":"bug","owner":null,"assignee":null,"labels":[],"created_at":"2025-01-01T00:00:00Z","created_by":null,"updated_at":"2025-01-01T00:00:00Z","closed_at":null,"close_reason":null,"blocked_by":null,"blocks":null,"comments":null,"external_ref":null,"estimate":null,"design":null,"acceptance_criteria":null,"notes":null,"parent":null,"dependents":null,"dependencies":null,"dependency_count":null,"dependent_count":null,"metadata":null,"spec_id":null,"comment_count":null}}"#
    )
}

fn failed(stderr: &str) -> BeadsError {
    BeadsError::CommandFailed {
        binary: "bd".to_string(),
        status: Some(1),
        status_display: "exit status: 1".to_string(),
        stderr: stderr.to_string(),
    }
}

fn empty_update() -> UpdatePayload {
    UpdatePayload {
        title: None,
        description: None,
        issue_type: None,
        status: Some("closed".to_string()),
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

// ---- show -----------------------------------------------------------------------

#[test]
fn show_not_found_error_texts_are_none() {
    for stderr in ["Error: no issue found matching bd-9", "Issue NOT FOUND"] {
        let inv = RecordingInvoker::new(bd1()).reply_json(Err(failed(stderr)));
        assert!(
            ops::show(&inv, &project(), "bd-9").unwrap().is_none(),
            "{stderr}"
        );
    }
}

#[test]
fn show_other_errors_propagate() {
    let inv = RecordingInvoker::new(bd1()).reply_json(Err(failed("database locked")));
    let err = ops::show(&inv, &project(), "bd-1").unwrap_err();
    assert_eq!(err.to_string(), "database locked");
}

#[test]
fn show_empty_output_and_empty_array_are_none() {
    let inv = RecordingInvoker::new(bd1())
        .reply_json(Ok("  \n".to_string()))
        .reply_json(Ok("[]".to_string()));
    assert!(ops::show(&inv, &project(), "bd-1").unwrap().is_none());
    assert!(ops::show(&inv, &project(), "bd-1").unwrap().is_none());
}

#[test]
fn show_accepts_object_or_array() {
    let object = issue_json("bd-1", "open");
    let array = format!("[{}]", issue_json("bd-2", "open"));
    let inv = RecordingInvoker::new(bd1())
        .reply_json(Ok(object))
        .reply_json(Ok(array));
    assert_eq!(
        ops::show(&inv, &project(), "bd-1")
            .unwrap()
            .map(|i| i.id)
            .as_deref(),
        Some("bd-1")
    );
    assert_eq!(
        ops::show(&inv, &project(), "bd-2")
            .unwrap()
            .map(|i| i.id)
            .as_deref(),
        Some("bd-2")
    );
}

#[test]
fn show_invalid_json_and_schema_mismatch_are_parse_failures() {
    let inv = RecordingInvoker::new(bd1())
        .reply_json(Ok("not json".to_string()))
        .reply_json(Ok(r#"{"id":"bd-1"}"#.to_string()));
    let invalid = ops::show(&inv, &project(), "bd-1").unwrap_err();
    assert!(matches!(
        invalid,
        BeadsError::ParseFailed {
            target: ParseTarget::Issue,
            id: None,
            ..
        }
    ));
    assert!(invalid.to_string().starts_with("Failed to parse issue: "));
    let mismatch = ops::show(&inv, &project(), "bd-1").unwrap_err();
    assert!(matches!(
        &mismatch,
        BeadsError::ParseFailed { target: ParseTarget::Issue, id: Some(id), .. } if id == "bd-1"
    ));
    assert!(mismatch
        .to_string()
        .starts_with("Failed to parse issue bd-1: "));
}

// ---- update -----------------------------------------------------------------------

#[test]
fn update_empty_output_fetches_with_show() {
    let inv = RecordingInvoker::new(bd1())
        .reply_json(Ok(String::new()))
        .reply_json(Ok(format!("[{}]", issue_json("bd-1", "closed"))));
    let issue = ops::update(&inv, &project(), "bd-1", &empty_update()).unwrap();
    assert_eq!(issue.map(|i| i.status).as_deref(), Some("closed"));
    assert_eq!(inv.calls().len(), 2);
}

#[test]
fn update_fallback_is_lenient_about_issue_shape() {
    let inv = RecordingInvoker::new(bd1())
        .reply_json(Ok(String::new()))
        .reply_json(Ok(r#"{"unexpected":true}"#.to_string()));
    assert!(ops::update(&inv, &project(), "bd-1", &empty_update())
        .unwrap()
        .is_none());
}

#[test]
fn update_fallback_invalid_json_is_fetch_failure() {
    let inv = RecordingInvoker::new(bd1())
        .reply_json(Ok(String::new()))
        .reply_json(Ok("not json".to_string()));
    let err = ops::update(&inv, &project(), "bd-1", &empty_update()).unwrap_err();
    assert!(matches!(
        err,
        BeadsError::ParseFailed {
            target: ParseTarget::UpdatedIssueFetch,
            ..
        }
    ));
    assert!(err
        .to_string()
        .starts_with("Failed to fetch updated issue: "));
}

#[test]
fn update_result_shapes() {
    let inv = RecordingInvoker::new(bd1())
        .reply_json(Ok(issue_json("bd-1", "closed")))
        .reply_json(Ok(r#"[{"id":"bd-1"}]"#.to_string()))
        .reply_json(Ok("garbage".to_string()));
    assert!(ops::update(&inv, &project(), "bd-1", &empty_update())
        .unwrap()
        .is_some());
    assert!(ops::update(&inv, &project(), "bd-1", &empty_update())
        .unwrap()
        .is_none());
    let err = ops::update(&inv, &project(), "bd-1", &empty_update()).unwrap_err();
    assert!(matches!(
        err,
        BeadsError::ParseFailed {
            target: ParseTarget::UpdatedIssue,
            ..
        }
    ));
    assert!(err
        .to_string()
        .starts_with("Failed to parse updated issue: "));
}

// ---- list, ready, search, status, create, close -------------------------------------

#[test]
fn list_fallback_returns_open_then_closed() {
    let inv = RecordingInvoker::new(None)
        .reply_json(Ok(format!("[{}]", issue_json("bd-1", "open"))))
        .reply_json(Ok(format!("[{}]", issue_json("bd-2", "closed"))));
    let query = ListQuery {
        include_all: Some(true),
        status: Some(vec!["open".to_string()]),
        ..ListQuery::default()
    };
    let ids: Vec<String> = ops::list(&inv, &project(), &query)
        .unwrap()
        .into_iter()
        .map(|i| i.id)
        .collect();
    assert_eq!(ids, ["bd-1", "bd-2"]);
    // The fallback does not apply filters.
    assert!(inv
        .calls()
        .iter()
        .all(|c| !c.iter().any(|a| a.starts_with("--status=open"))));
}

#[test]
fn list_and_ready_propagate_parse_errors() {
    let inv = RecordingInvoker::new(bd1())
        .reply_json(Ok("{}".to_string()))
        .reply_json(Ok("nope".to_string()));
    let list = ops::list(&inv, &project(), &ListQuery::default()).unwrap_err();
    assert_eq!(
        list.to_string(),
        "Expected JSON array or paginated envelope"
    );
    let ready = ops::ready(&inv, &project()).unwrap_err();
    assert!(matches!(ready, BeadsError::InvalidJson { .. }));
}

#[test]
fn search_empty_output_or_empty_array_is_empty() {
    let inv = RecordingInvoker::new(bd1())
        .reply_json(Ok(String::new()))
        .reply_json(Ok(" [] ".to_string()))
        .reply_json(Ok("{}".to_string()));
    assert!(ops::search(&inv, &project(), "q").unwrap().is_empty());
    assert!(ops::search(&inv, &project(), "q").unwrap().is_empty());
    let err = ops::search(&inv, &project(), "q").unwrap_err();
    assert!(err
        .to_string()
        .starts_with("Failed to parse search results: "));
}

#[test]
fn status_create_close_parse_failures_keep_their_messages() {
    let inv = RecordingInvoker::new(bd1())
        .reply_json(Ok("x".to_string()))
        .reply_json(Ok("x".to_string()))
        .reply_json(Ok("x".to_string()));
    assert!(ops::status(&inv, &project())
        .unwrap_err()
        .to_string()
        .starts_with("Failed to parse status: "));
    let create = ops::create(
        &inv,
        &project(),
        &btit_types::CreatePayload {
            title: "T".to_string(),
            description: None,
            issue_type: None,
            priority: None,
            assignee: None,
            labels: Some(Vec::new()),
            external_ref: None,
            estimate_minutes: None,
            design_notes: None,
            acceptance_criteria: None,
            working_notes: None,
            parent: Some(String::new()),
            spec_id: Some(String::new()),
            cwd: Some("/ignored".to_string()),
        },
    )
    .unwrap_err();
    assert!(create
        .to_string()
        .starts_with("Failed to parse created issue: "));
    assert!(ops::close(&inv, &project(), "bd-1", false)
        .unwrap_err()
        .to_string()
        .starts_with("Failed to parse close result: "));
    // Empty labels, parent and spec id are omitted from `create`.
    assert_eq!(
        inv.calls().get(1).cloned(),
        Some(vec![
            "create".to_string(),
            "T".to_string(),
            "--json".to_string()
        ])
    );
}

#[test]
fn close_with_and_without_suggest_next() {
    let inv = RecordingInvoker::new(bd1())
        .reply_json(Ok(r#"{"closed":["bd-1"]}"#.to_string()))
        .reply_json(Ok("{}".to_string()));
    assert_eq!(
        ops::close(&inv, &project(), "bd-1", true).unwrap()["closed"][0],
        "bd-1"
    );
    ops::close(&inv, &project(), "bd-1", false).unwrap();
    let calls = inv.calls();
    assert!(calls
        .first()
        .is_some_and(|c| c.contains(&"--suggest-next".to_string())));
    assert!(calls
        .get(1)
        .is_some_and(|c| !c.contains(&"--suggest-next".to_string())));
}

#[test]
fn delete_with_and_without_hard() {
    let inv = RecordingInvoker::new(bd1());
    ops::delete(&inv, &project(), "bd-1", true).unwrap();
    ops::delete(&inv, &project(), "bd-1", false).unwrap();
    let calls = inv.calls();
    assert!(calls
        .first()
        .is_some_and(|c| c.contains(&"--hard".to_string())));
    assert!(calls
        .get(1)
        .is_some_and(|c| !c.contains(&"--hard".to_string())));
}

#[test]
fn relation_types_br_vs_others() {
    let br = ops::relation_types(CliClient::Br);
    assert_eq!(br.len(), 7);
    assert_eq!(
        br.first().map(|t| (t.value, t.label)),
        Some(("relates-to", "Relates To"))
    );
    for client in [CliClient::Bd, CliClient::Unknown] {
        let all = ops::relation_types(client);
        assert_eq!(all.len(), 10);
        assert_eq!(all.get(..7), Some(br.as_slice()));
        assert_eq!(
            all.last().map(|t| (t.value, t.label)),
            Some(("validates", "Validates"))
        );
    }
}

// ---- sync -------------------------------------------------------------------------

#[test]
fn sync_nonzero_exit_is_command_failed_with_stderr() {
    let inv = RecordingInvoker::new(bd1())
        .with_binary("/opt/bd")
        .reply_raw(Ok(CliOutput {
            status: Some(2),
            success: false,
            stdout: String::new(),
            stderr: "remote rejected\n".to_string(),
        }));
    let err = ops::sync(&inv, &project(), false).unwrap_err();
    assert!(matches!(
        &err,
        BeadsError::CommandFailed { binary, status: Some(2), stderr, .. }
            if binary == "/opt/bd" && stderr == "remote rejected\n"
    ));
}

#[test]
fn sync_spawn_error_propagates() {
    let inv = RecordingInvoker::new(bd1()).reply_raw(Err(BeadsError::Spawn {
        binary: "bd".to_string(),
        operation: Some("sync".to_string()),
        source: io::Error::new(io::ErrorKind::NotFound, "missing"),
    }));
    let err = ops::sync(&inv, &project(), false).unwrap_err();
    assert_eq!(err.to_string(), "Failed to run bd sync: missing");
}

// ---- RecordingInvoker ---------------------------------------------------------------

#[test]
fn recording_invoker_counts_probes_and_reports_seeded_info() {
    let inv = RecordingInvoker::new(bd1());
    assert_eq!(inv.probe_calls(), 0);
    assert_eq!(inv.client_info(), bd1());
    assert_eq!(inv.probe_calls(), 0);
    assert_eq!(inv.probe(), bd1());
    assert_eq!(inv.probe(), bd1());
    assert_eq!(inv.probe_calls(), 2);
    assert_eq!(inv.binary(), "bd");
    let none = RecordingInvoker::new(None);
    assert_eq!(none.client(), CliClient::Unknown);
    assert_eq!(
        none.capabilities(),
        btit_types::BackendCapabilities::default()
    );
}
