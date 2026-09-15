//! Compile-time pin of `btit-cli`'s public API.
//!
//! `btit-bd` (b-5), `btit-br` (b-6) and the app (b-7, b-8) build against this contract.
//! It fails to compile if `CliInvoker` gains or changes a method, or if a signature of
//! `CliRunner`, `ProjectLocks`, `run`, `probe`, `path`, `command`, `ops` or (with
//! feature `test-support`) `testing::RecordingInvoker` changes.

use std::process::Command;
use std::sync::{Arc, Mutex};

use btit_beads::error::BeadsError;
use btit_cli::command::new_command;
use btit_cli::ops;
use btit_cli::path::{extended_path_entries, get_extended_path};
use btit_cli::probe::{default_cli_binary, probe_cli_binary};
use btit_cli::run::{
    json_argv, json_invocation, probe_version_output, resolve_working_dir, run_json, run_raw,
    spawn_json,
};
use btit_cli::{CliInvoker, CliRunner, ProjectLocks};
use btit_types::{
    BackendCapabilities, BdRawIssue, CliClient, CliOutput, CliProbe, CliVersion, CreatePayload,
    ListQuery, ProjectRef, RelationType, UpdatePayload,
};

/// Implements exactly the five required methods: a new required method or a changed
/// signature breaks this impl.
struct Pin;

impl CliInvoker for Pin {
    fn binary(&self) -> String {
        "pin".to_string()
    }
    fn client_info(&self) -> Option<CliProbe> {
        None
    }
    fn probe(&self) -> Option<CliProbe> {
        None
    }
    fn run_json(
        &self,
        _project: &ProjectRef,
        _command: &str,
        _args: &[String],
    ) -> Result<String, BeadsError> {
        Ok(String::new())
    }
    fn run_raw(&self, _project: &ProjectRef, _args: &[&str]) -> Result<CliOutput, BeadsError> {
        Ok(CliOutput {
            status: Some(0),
            success: true,
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}

fn assert_send_sync<T: ?Sized + Send + Sync>() {}

#[test]
fn invoker_trait_shape_and_provided_defaults() {
    assert_send_sync::<dyn CliInvoker>();
    assert_send_sync::<CliRunner>();
    assert_send_sync::<ProjectLocks>();
    let inv: &dyn CliInvoker = &Pin;
    let _: String = inv.binary();
    let _: Option<CliProbe> = inv.client_info();
    let _: Option<CliProbe> = inv.probe();
    let caps: BackendCapabilities = inv.capabilities();
    let client: CliClient = inv.client();
    let version: Option<CliVersion> = inv.version();
    assert_eq!(caps, BackendCapabilities::default());
    assert_eq!(client, CliClient::Unknown);
    assert_eq!(version, None);
}

type ListFn = fn(&dyn CliInvoker, &ProjectRef, &ListQuery) -> Result<Vec<BdRawIssue>, BeadsError>;
type IssuesFn = fn(&dyn CliInvoker, &ProjectRef) -> Result<Vec<BdRawIssue>, BeadsError>;
type StatusFn = fn(&dyn CliInvoker, &ProjectRef) -> Result<serde_json::Value, BeadsError>;
type ShowFn = fn(&dyn CliInvoker, &ProjectRef, &str) -> Result<Option<BdRawIssue>, BeadsError>;
type CreateFn = fn(&dyn CliInvoker, &ProjectRef, &CreatePayload) -> Result<BdRawIssue, BeadsError>;
type UpdateFn = fn(
    &dyn CliInvoker,
    &ProjectRef,
    &str,
    &UpdatePayload,
) -> Result<Option<BdRawIssue>, BeadsError>;
type CloseFn =
    fn(&dyn CliInvoker, &ProjectRef, &str, bool) -> Result<serde_json::Value, BeadsError>;
type SearchFn = fn(&dyn CliInvoker, &ProjectRef, &str) -> Result<Vec<BdRawIssue>, BeadsError>;
type PairFn = fn(&dyn CliInvoker, &ProjectRef, &str, &str) -> Result<(), BeadsError>;
type DeleteFn = fn(&dyn CliInvoker, &ProjectRef, &str, bool) -> Result<(), BeadsError>;
type DepAddFn =
    fn(&dyn CliInvoker, &ProjectRef, &str, &str, Option<&str>) -> Result<(), BeadsError>;
type SyncFn = fn(&dyn CliInvoker, &ProjectRef, bool) -> Result<(), BeadsError>;
type JsonInvocationFn = fn(Option<&CliProbe>, &str, &[String]) -> (CliClient, Vec<String>);
type RunJsonFn = fn(&str, bool, &ProjectLocks, &str, &str, &[String]) -> Result<String, BeadsError>;

#[test]
fn ops_signatures() {
    let _: ListFn = ops::list;
    let _: IssuesFn = ops::ready;
    let _: StatusFn = ops::status;
    let _: ShowFn = ops::show;
    let _: CreateFn = ops::create;
    let _: UpdateFn = ops::update;
    let _: CloseFn = ops::close;
    let _: SearchFn = ops::search;
    let _: PairFn = ops::label_add;
    let _: PairFn = ops::label_remove;
    let _: DeleteFn = ops::delete;
    let _: PairFn = ops::comment_add;
    let _: DepAddFn = ops::dep_add;
    let _: PairFn = ops::dep_remove;
    let _: fn(CliClient) -> Vec<RelationType> = ops::relation_types;
    let _: SyncFn = ops::sync;
}

#[test]
fn transport_signatures() {
    let _: fn(&str, &[String], bool) -> Vec<String> = json_argv;
    let _: JsonInvocationFn = json_invocation;
    let _: fn(&ProjectRef, CliClient) -> Result<String, BeadsError> = resolve_working_dir;
    let _: fn(&str, &ProjectLocks, &str, &[String]) -> Result<String, BeadsError> = spawn_json;
    let _: RunJsonFn = run_json;
    let _: fn(&str, &str, &[&str]) -> Result<CliOutput, BeadsError> = run_raw;
    let _: fn(&str) -> Result<CliOutput, BeadsError> = probe_version_output;
    let _: fn(&str) -> Option<CliProbe> = probe_cli_binary;
    let _: fn() -> String = default_cli_binary;
    let _: fn() -> String = get_extended_path;
    let _: fn() -> Vec<String> = extended_path_entries;
    let _: fn(&str) -> Command = new_command;
    let _: fn(&ProjectLocks, &str) -> Arc<Mutex<()>> = ProjectLocks::guard;
    let _: fn() -> ProjectLocks = ProjectLocks::new;
}

#[test]
fn runner_constructors() {
    let locks = Arc::new(ProjectLocks::default());
    let runner: CliRunner = CliRunner::new("bd", Arc::clone(&locks));
    let _: &dyn CliInvoker = &runner;
    let probe = CliProbe {
        client: CliClient::Br,
        version: Some((0, 1, 33).into()),
        raw: "br 0.1.33".to_string(),
    };
    let seeded: CliRunner = CliRunner::with_probe(String::from("br"), locks, probe.clone());
    assert_eq!(seeded.client_info(), Some(probe));
    assert_eq!(seeded.binary(), "br");
}

#[cfg(feature = "test-support")]
#[test]
fn recording_invoker_signatures() {
    use btit_cli::testing::RecordingInvoker;

    let _: fn(Option<CliProbe>) -> RecordingInvoker = RecordingInvoker::new;
    let _: fn(RecordingInvoker, Result<String, BeadsError>) -> RecordingInvoker =
        RecordingInvoker::reply_json;
    let _: fn(RecordingInvoker, Result<CliOutput, BeadsError>) -> RecordingInvoker =
        RecordingInvoker::reply_raw;
    let _: fn(&RecordingInvoker) -> Vec<Vec<String>> = RecordingInvoker::calls;
    let _: fn(&RecordingInvoker) -> usize = RecordingInvoker::probe_calls;
    let inv = RecordingInvoker::new(None).with_binary("br");
    let _: &dyn CliInvoker = &inv;
    assert_send_sync::<RecordingInvoker>();
    assert_eq!(inv.calls(), Vec::<Vec<String>>::new());
}
