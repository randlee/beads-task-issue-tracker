//! Compile-time pin of `btit-beads`' public API.
//!
//! b-4..b-8 build against this contract while b-9 changes behaviour behind it, so
//! b-9 must leave this file byte-identical. It fails to compile if a public
//! function signature, trait method, `BeadsError` variant or field changes, and
//! fails at runtime if a `code()` string or an accessor default changes.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use btit_beads::compat::cli_compatibility_warnings;
use btit_beads::detect::{
    cli_client_name, detect_cli_client, is_legacy_bd, parse_bd_version, parse_cli_probe,
    rank_cli_candidate, select_default_binary, CliSelection, CLI_CANDIDATES, CLI_FALLBACK,
    MIN_SUPPORTED_BD_MAJOR,
};
use btit_beads::gates::{
    capabilities_for, supports_daemon_flag_for, supports_delete_hard_flag_for,
    supports_list_all_flag_for, uses_dolt_backend_for, uses_jsonl_files_for,
};
use btit_beads::issues::{
    normalize_issue_status, normalize_issue_type, normalize_metadata, priority_to_number,
    priority_to_string, transform_issue,
};
use btit_beads::logging::{LOGGING_ENABLED, VERBOSE_LOGGING};
use btit_beads::parse::parse_issues_tolerant;
use btit_beads::{
    BeadsBackend, BeadsError, CliBackend, CloseSuggestions, DoltOperations, ExpectedShape,
    ParseTarget,
};
use btit_types::{
    BackendCapabilities, BdRawIssue, CliClient, CliOutput, CliProbe, CliVersion, CreatePayload,
    DoltOpResult, Issue, ListQuery, ProjectRef, RelationType, ReleaseSource, UpdatePayload,
};

// ---- object safety and auto traits ------------------------------------------

fn _obj(_: &dyn CliBackend) {}
fn _obj_beads(_: &dyn BeadsBackend) {}
fn _obj_dolt(_: &dyn DoltOperations) {}
fn _obj_close(_: &dyn CloseSuggestions) {}

fn assert_send_sync<T: ?Sized + Send + Sync>() {}
fn assert_error<E: std::error::Error + 'static>() {}

#[test]
fn traits_and_error_are_send_sync() {
    assert_send_sync::<dyn BeadsBackend>();
    assert_send_sync::<dyn CliBackend>();
    assert_send_sync::<dyn DoltOperations>();
    assert_send_sync::<dyn CloseSuggestions>();
    assert_send_sync::<BeadsError>();
    assert_error::<BeadsError>();
}

// ---- free functions, constants, statics -------------------------------------

type ProbeFn = fn(&str) -> Option<CliProbe>;
type UpdateFn =
    fn(&Probe, &ProjectRef, &str, &UpdatePayload) -> Result<Option<BdRawIssue>, BeadsError>;
type DepAddFn = fn(&Probe, &ProjectRef, &str, &str, Option<&str>) -> Result<(), BeadsError>;

#[test]
fn free_function_signatures() {
    // detect
    let _: &[&str] = CLI_CANDIDATES;
    let _: &str = CLI_FALLBACK;
    let _: u32 = MIN_SUPPORTED_BD_MAJOR;
    let _: fn(CliClient, Option<CliVersion>) -> bool = is_legacy_bd;
    let _: fn(&CliProbe) -> u8 = rank_cli_candidate;
    let _: fn(&[&str], &str, ProbeFn) -> CliSelection = select_default_binary::<ProbeFn>;
    let _: fn(&str) -> CliProbe = parse_cli_probe;
    let _: fn(CliClient) -> &'static str = cli_client_name;
    let _: fn(&str) -> CliClient = detect_cli_client;
    let _: fn(&str) -> Option<CliVersion> = parse_bd_version;
    let _: fn(&CliSelection) -> &str = CliSelection::binary;
    let _: fn(&CliSelection) -> Option<&CliProbe> = CliSelection::probe;
    let _: fn(&CliSelection) -> bool = CliSelection::is_legacy;
    // compat
    let _: fn(CliClient, Option<CliVersion>) -> Vec<String> = cli_compatibility_warnings;
    // gates
    let _: fn(CliClient, u32, u32, u32) -> bool = supports_daemon_flag_for;
    let _: fn(CliClient, u32, u32, u32) -> bool = uses_jsonl_files_for;
    let _: fn(CliClient, u32, u32, u32) -> bool = supports_list_all_flag_for;
    let _: fn(CliClient, u32, u32, u32) -> bool = supports_delete_hard_flag_for;
    let _: fn(CliClient, u32, u32, u32) -> bool = uses_dolt_backend_for;
    let _: fn(CliClient, Option<CliVersion>) -> BackendCapabilities = capabilities_for;
    // issues
    let _: fn(i32) -> String = priority_to_string;
    let _: fn(&str) -> String = priority_to_number;
    let _: fn(&str) -> String = normalize_issue_type;
    let _: fn(&str) -> String = normalize_issue_status;
    let _: fn(BdRawIssue) -> Issue = transform_issue;
    let _: fn(Option<serde_json::Value>) -> Option<serde_json::Value> = normalize_metadata;
    // parse
    let _: fn(&str, &str) -> Result<Vec<BdRawIssue>, BeadsError> = parse_issues_tolerant;
    // error
    let _: fn(&BeadsError) -> &'static str = BeadsError::code;
    let _: fn(&BeadsError) -> &'static str = BeadsError::remediation;
    // logging
    let _: &AtomicBool = &LOGGING_ENABLED;
    let _: &AtomicBool = &VERBOSE_LOGGING;
}

#[test]
fn log_macros_are_exported() {
    btit_beads::log_info!("[api_freeze] {}", 1);
    btit_beads::log_warn!("[api_freeze] {}", 2);
    btit_beads::log_error!("[api_freeze] {}", 3);
    btit_beads::log_debug!("[api_freeze] {}", 4);
    btit_beads::logging::log::info!("[api_freeze] log facade re-export");
}

// ---- traits -----------------------------------------------------------------

fn value() -> serde_json::Value {
    serde_json::Value::Null
}

/// Implements every trait without overriding the accessors, pinning their `None` defaults.
struct Probe;

impl BeadsBackend for Probe {
    fn project_uses_dolt(&self, _project: &ProjectRef) -> bool {
        false
    }
    fn list(
        &self,
        _project: &ProjectRef,
        _query: &ListQuery,
    ) -> Result<Vec<BdRawIssue>, BeadsError> {
        Ok(Vec::new())
    }
    fn ready(&self, _project: &ProjectRef) -> Result<Vec<BdRawIssue>, BeadsError> {
        Ok(Vec::new())
    }
    fn status(&self, _project: &ProjectRef) -> Result<serde_json::Value, BeadsError> {
        Ok(value())
    }
    fn show(&self, _project: &ProjectRef, _id: &str) -> Result<Option<BdRawIssue>, BeadsError> {
        Ok(None)
    }
    fn create(
        &self,
        _project: &ProjectRef,
        _payload: &CreatePayload,
    ) -> Result<BdRawIssue, BeadsError> {
        Err(BeadsError::Unsupported {
            operation: "create",
            client: CliClient::Unknown,
        })
    }
    fn update(
        &self,
        _project: &ProjectRef,
        _id: &str,
        _updates: &UpdatePayload,
    ) -> Result<Option<BdRawIssue>, BeadsError> {
        Ok(None)
    }
    fn close(&self, _project: &ProjectRef, _id: &str) -> Result<serde_json::Value, BeadsError> {
        Ok(value())
    }
    fn search(&self, _project: &ProjectRef, _query: &str) -> Result<Vec<BdRawIssue>, BeadsError> {
        Ok(Vec::new())
    }
    fn label_add(&self, _project: &ProjectRef, _id: &str, _label: &str) -> Result<(), BeadsError> {
        Ok(())
    }
    fn label_remove(
        &self,
        _project: &ProjectRef,
        _id: &str,
        _label: &str,
    ) -> Result<(), BeadsError> {
        Ok(())
    }
    fn delete(&self, _project: &ProjectRef, _id: &str) -> Result<(), BeadsError> {
        Ok(())
    }
    fn comment_add(
        &self,
        _project: &ProjectRef,
        _id: &str,
        _content: &str,
    ) -> Result<(), BeadsError> {
        Ok(())
    }
    fn dep_add(
        &self,
        _project: &ProjectRef,
        _issue_id: &str,
        _depends_on_id: &str,
        _relation_type: Option<&str>,
    ) -> Result<(), BeadsError> {
        Ok(())
    }
    fn dep_remove(
        &self,
        _project: &ProjectRef,
        _issue_id: &str,
        _depends_on_id: &str,
    ) -> Result<(), BeadsError> {
        Ok(())
    }
    fn relation_types(&self) -> Vec<RelationType> {
        vec![RelationType {
            value: "blocks",
            label: "Blocks",
        }]
    }
    fn sync(&self, _project: &ProjectRef) -> Result<(), BeadsError> {
        Ok(())
    }
}

impl CliBackend for Probe {
    fn binary(&self) -> String {
        "bd".to_string()
    }
    fn probe(&self) -> Option<CliProbe> {
        None
    }
    fn client(&self) -> CliClient {
        CliClient::Unknown
    }
    fn version(&self) -> Option<CliVersion> {
        None
    }
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }
    fn run_raw(&self, _project: &ProjectRef, _args: &[&str]) -> Result<CliOutput, BeadsError> {
        Ok(CliOutput {
            status: Some(0),
            success: true,
            stdout: String::new(),
            stderr: String::new(),
        })
    }
    fn release_source(&self) -> ReleaseSource {
        ReleaseSource {
            api_url: "https://api.github.com/repos/steveyegge/beads/releases/latest",
            releases_url: "https://github.com/steveyegge/beads/releases",
        }
    }
}

fn dolt_ok() -> DoltOpResult {
    DoltOpResult {
        success: true,
        message: String::new(),
        detail: String::new(),
    }
}

impl DoltOperations for Probe {
    fn doctor_fix(&self, _project: &ProjectRef) -> Result<DoltOpResult, BeadsError> {
        Ok(dolt_ok())
    }
    fn migrate_to_dolt(&self, _project: &ProjectRef) -> Result<DoltOpResult, BeadsError> {
        Ok(dolt_ok())
    }
    fn init(&self, _project: &ProjectRef, _prefix: &str) -> Result<DoltOpResult, BeadsError> {
        Ok(dolt_ok())
    }
    fn import_jsonl(
        &self,
        _project: &ProjectRef,
        _file: &Path,
    ) -> Result<DoltOpResult, BeadsError> {
        Ok(dolt_ok())
    }
}

impl CloseSuggestions for Probe {
    fn close_suggesting_next(
        &self,
        _project: &ProjectRef,
        _id: &str,
    ) -> Result<serde_json::Value, BeadsError> {
        Ok(value())
    }
}

/// Overrides every accessor, pinning the accessor pattern (`Some(self)` as `&dyn Trait`).
struct Full(Probe);

impl BeadsBackend for Full {
    fn project_uses_dolt(&self, project: &ProjectRef) -> bool {
        self.0.project_uses_dolt(project)
    }
    fn list(&self, project: &ProjectRef, query: &ListQuery) -> Result<Vec<BdRawIssue>, BeadsError> {
        self.0.list(project, query)
    }
    fn ready(&self, project: &ProjectRef) -> Result<Vec<BdRawIssue>, BeadsError> {
        self.0.ready(project)
    }
    fn status(&self, project: &ProjectRef) -> Result<serde_json::Value, BeadsError> {
        self.0.status(project)
    }
    fn show(&self, project: &ProjectRef, id: &str) -> Result<Option<BdRawIssue>, BeadsError> {
        self.0.show(project, id)
    }
    fn create(
        &self,
        project: &ProjectRef,
        payload: &CreatePayload,
    ) -> Result<BdRawIssue, BeadsError> {
        self.0.create(project, payload)
    }
    fn update(
        &self,
        project: &ProjectRef,
        id: &str,
        updates: &UpdatePayload,
    ) -> Result<Option<BdRawIssue>, BeadsError> {
        self.0.update(project, id, updates)
    }
    fn close(&self, project: &ProjectRef, id: &str) -> Result<serde_json::Value, BeadsError> {
        self.0.close(project, id)
    }
    fn search(&self, project: &ProjectRef, query: &str) -> Result<Vec<BdRawIssue>, BeadsError> {
        self.0.search(project, query)
    }
    fn label_add(&self, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError> {
        self.0.label_add(project, id, label)
    }
    fn label_remove(&self, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError> {
        self.0.label_remove(project, id, label)
    }
    fn delete(&self, project: &ProjectRef, id: &str) -> Result<(), BeadsError> {
        self.0.delete(project, id)
    }
    fn comment_add(&self, project: &ProjectRef, id: &str, content: &str) -> Result<(), BeadsError> {
        self.0.comment_add(project, id, content)
    }
    fn dep_add(
        &self,
        project: &ProjectRef,
        issue_id: &str,
        depends_on_id: &str,
        relation_type: Option<&str>,
    ) -> Result<(), BeadsError> {
        self.0
            .dep_add(project, issue_id, depends_on_id, relation_type)
    }
    fn dep_remove(
        &self,
        project: &ProjectRef,
        issue_id: &str,
        depends_on_id: &str,
    ) -> Result<(), BeadsError> {
        self.0.dep_remove(project, issue_id, depends_on_id)
    }
    fn relation_types(&self) -> Vec<RelationType> {
        self.0.relation_types()
    }
    fn sync(&self, project: &ProjectRef) -> Result<(), BeadsError> {
        self.0.sync(project)
    }
    fn cli(&self) -> Option<&dyn CliBackend> {
        Some(&self.0)
    }
    fn dolt(&self) -> Option<&dyn DoltOperations> {
        Some(&self.0)
    }
    fn close_suggestions(&self) -> Option<&dyn CloseSuggestions> {
        Some(&self.0)
    }
}

#[test]
fn trait_method_signatures() {
    let _: fn(&Probe, &ProjectRef) -> bool = <Probe as BeadsBackend>::project_uses_dolt;
    let _: fn(&Probe, &ProjectRef, &ListQuery) -> Result<Vec<BdRawIssue>, BeadsError> =
        <Probe as BeadsBackend>::list;
    let _: fn(&Probe, &ProjectRef) -> Result<Vec<BdRawIssue>, BeadsError> =
        <Probe as BeadsBackend>::ready;
    let _: fn(&Probe, &ProjectRef) -> Result<serde_json::Value, BeadsError> =
        <Probe as BeadsBackend>::status;
    let _: fn(&Probe, &ProjectRef, &str) -> Result<Option<BdRawIssue>, BeadsError> =
        <Probe as BeadsBackend>::show;
    let _: fn(&Probe, &ProjectRef, &CreatePayload) -> Result<BdRawIssue, BeadsError> =
        <Probe as BeadsBackend>::create;
    let _: UpdateFn = <Probe as BeadsBackend>::update;
    let _: fn(&Probe, &ProjectRef, &str) -> Result<serde_json::Value, BeadsError> =
        <Probe as BeadsBackend>::close;
    let _: fn(&Probe, &ProjectRef, &str) -> Result<Vec<BdRawIssue>, BeadsError> =
        <Probe as BeadsBackend>::search;
    let _: fn(&Probe, &ProjectRef, &str, &str) -> Result<(), BeadsError> =
        <Probe as BeadsBackend>::label_add;
    let _: fn(&Probe, &ProjectRef, &str, &str) -> Result<(), BeadsError> =
        <Probe as BeadsBackend>::label_remove;
    let _: fn(&Probe, &ProjectRef, &str) -> Result<(), BeadsError> =
        <Probe as BeadsBackend>::delete;
    let _: fn(&Probe, &ProjectRef, &str, &str) -> Result<(), BeadsError> =
        <Probe as BeadsBackend>::comment_add;
    let _: DepAddFn = <Probe as BeadsBackend>::dep_add;
    let _: fn(&Probe, &ProjectRef, &str, &str) -> Result<(), BeadsError> =
        <Probe as BeadsBackend>::dep_remove;
    let _: fn(&Probe) -> Vec<RelationType> = <Probe as BeadsBackend>::relation_types;
    let _: fn(&Probe, &ProjectRef) -> Result<(), BeadsError> = <Probe as BeadsBackend>::sync;
    let _: fn(&Probe) -> Option<&dyn CliBackend> = <Probe as BeadsBackend>::cli;
    let _: fn(&Probe) -> Option<&dyn DoltOperations> = <Probe as BeadsBackend>::dolt;
    let _: fn(&Probe) -> Option<&dyn CloseSuggestions> = <Probe as BeadsBackend>::close_suggestions;

    let _: fn(&Probe) -> String = <Probe as CliBackend>::binary;
    let _: fn(&Probe) -> Option<CliProbe> = <Probe as CliBackend>::probe;
    let _: fn(&Probe) -> CliClient = <Probe as CliBackend>::client;
    let _: fn(&Probe) -> Option<CliVersion> = <Probe as CliBackend>::version;
    let _: fn(&Probe) -> BackendCapabilities = <Probe as CliBackend>::capabilities;
    let _: fn(&Probe, &ProjectRef, &[&str]) -> Result<CliOutput, BeadsError> =
        <Probe as CliBackend>::run_raw;
    let _: fn(&Probe) -> ReleaseSource = <Probe as CliBackend>::release_source;

    let _: fn(&Probe, &ProjectRef) -> Result<DoltOpResult, BeadsError> =
        <Probe as DoltOperations>::doctor_fix;
    let _: fn(&Probe, &ProjectRef) -> Result<DoltOpResult, BeadsError> =
        <Probe as DoltOperations>::migrate_to_dolt;
    let _: fn(&Probe, &ProjectRef, &str) -> Result<DoltOpResult, BeadsError> =
        <Probe as DoltOperations>::init;
    let _: fn(&Probe, &ProjectRef, &Path) -> Result<DoltOpResult, BeadsError> =
        <Probe as DoltOperations>::import_jsonl;

    let _: fn(&Probe, &ProjectRef, &str) -> Result<serde_json::Value, BeadsError> =
        <Probe as CloseSuggestions>::close_suggesting_next;
}

#[test]
fn accessor_defaults_are_none() {
    let backend: &dyn BeadsBackend = &Probe;
    assert!(backend.cli().is_none());
    assert!(backend.dolt().is_none());
    assert!(backend.close_suggestions().is_none());
}

#[test]
fn accessors_reach_backend_specific_traits_without_downcasting() {
    let project = ProjectRef::local(None);
    let backend: std::sync::Arc<dyn BeadsBackend> = std::sync::Arc::new(Full(Probe));
    let cli = backend.cli().expect("cli accessor");
    assert_eq!(cli.binary(), "bd");
    assert_eq!(cli.client(), CliClient::Unknown);
    assert!(cli.version().is_none());
    assert!(cli.probe().is_none());
    assert_eq!(cli.capabilities(), BackendCapabilities::default());
    assert!(cli.run_raw(&project, &["sync"]).is_ok());
    assert!(cli.release_source().api_url.contains("steveyegge/beads"));
    let dolt = backend.dolt().expect("dolt accessor");
    assert!(dolt.doctor_fix(&project).is_ok_and(|r| r.success));
    assert!(dolt.migrate_to_dolt(&project).is_ok());
    assert!(dolt.init(&project, "proj").is_ok());
    assert!(dolt
        .import_jsonl(&project, Path::new("issues.jsonl"))
        .is_ok());
    let close = backend.close_suggestions().expect("close accessor");
    assert!(close.close_suggesting_next(&project, "a-1").is_ok());

    assert!(!backend.project_uses_dolt(&project));
    assert!(backend.list(&project, &ListQuery::default()).is_ok());
    assert!(backend.ready(&project).is_ok());
    assert!(backend.status(&project).is_ok());
    assert!(backend.show(&project, "a-1").is_ok());
    assert!(backend.close(&project, "a-1").is_ok());
    assert!(backend.search(&project, "q").is_ok());
    assert!(backend.label_add(&project, "a-1", "l").is_ok());
    assert!(backend.label_remove(&project, "a-1", "l").is_ok());
    assert!(backend.delete(&project, "a-1").is_ok());
    assert!(backend.comment_add(&project, "a-1", "c").is_ok());
    assert!(backend
        .dep_add(&project, "a-1", "a-2", Some("blocks"))
        .is_ok());
    assert!(backend.dep_remove(&project, "a-1", "a-2").is_ok());
    assert_eq!(backend.relation_types().len(), 1);
    assert!(backend.sync(&project).is_ok());
}

// ---- BeadsError ---------------------------------------------------------------

/// Every variant with every field named (no `..`), so adding, removing or retyping a field
/// fails to compile, and its `code()` string.
#[test]
fn beads_error_variants_fields_and_codes() {
    let json_error = || serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    let errors = [
        BeadsError::Spawn {
            binary: "bd".to_string(),
            operation: Some("doctor".to_string()),
            source: std::io::Error::other("x"),
        },
        BeadsError::CommandFailed {
            binary: "bd".to_string(),
            status: Some(1),
            status_display: "exit status: 1".to_string(),
            stderr: String::new(),
        },
        BeadsError::SchemaMigration {
            binary: "bd".to_string(),
        },
        BeadsError::InvalidJson {
            context: "ctx".to_string(),
            source: json_error(),
        },
        BeadsError::UnexpectedShape {
            context: "ctx".to_string(),
            expected: ExpectedShape::ArrayOrEnvelope,
        },
        BeadsError::ParseFailed {
            target: ParseTarget::Issue,
            id: Some("a-1".to_string()),
            source: json_error(),
        },
        BeadsError::Unsupported {
            operation: "op",
            client: CliClient::Br,
        },
    ];
    let mut codes = Vec::new();
    for e in &errors {
        match e {
            BeadsError::Spawn {
                binary,
                operation,
                source,
            } => {
                let _: (&String, &Option<String>, &std::io::Error) = (binary, operation, source);
            }
            BeadsError::CommandFailed {
                binary,
                status,
                status_display,
                stderr,
            } => {
                let _: (&String, &Option<i32>, &String, &String) =
                    (binary, status, status_display, stderr);
            }
            BeadsError::SchemaMigration { binary } => {
                let _: &String = binary;
            }
            BeadsError::InvalidJson { context, source } => {
                let _: (&String, &serde_json::Error) = (context, source);
            }
            BeadsError::UnexpectedShape { context, expected } => {
                let _: (&String, &ExpectedShape) = (context, expected);
            }
            BeadsError::ParseFailed { target, id, source } => {
                let _: (&ParseTarget, &Option<String>, &serde_json::Error) = (target, id, source);
            }
            BeadsError::Unsupported { operation, client } => {
                let _: (&&'static str, &CliClient) = (operation, client);
            }
            _ => panic!("unpinned BeadsError variant: {e:?}"),
        }
        assert!(!e.remediation().is_empty());
        codes.push(e.code());
    }
    assert_eq!(
        codes,
        [
            "BTIT_BEADS_SPAWN",
            "BTIT_BEADS_COMMAND_FAILED",
            "BTIT_BEADS_SCHEMA_MIGRATION",
            "BTIT_BEADS_INVALID_JSON",
            "BTIT_BEADS_UNEXPECTED_SHAPE",
            "BTIT_BEADS_PARSE_FAILED",
            "BTIT_BEADS_UNSUPPORTED",
        ]
    );
}

#[test]
fn spawn_operation_is_optional_string() {
    let e = BeadsError::Spawn {
        binary: "bd".to_string(),
        operation: None::<String>,
        source: std::io::Error::other("boom"),
    };
    assert_eq!(e.to_string(), "Failed to execute bd: boom");
}

#[test]
fn shape_and_target_variants_are_exhaustive() {
    for shape in [ExpectedShape::Array, ExpectedShape::ArrayOrEnvelope] {
        match shape {
            ExpectedShape::Array | ExpectedShape::ArrayOrEnvelope => {}
        }
    }
    for target in [
        ParseTarget::Status,
        ParseTarget::Issue,
        ParseTarget::CreatedIssue,
        ParseTarget::UpdatedIssue,
        ParseTarget::UpdatedIssueFetch,
        ParseTarget::CloseResult,
        ParseTarget::SearchResults,
    ] {
        match target {
            ParseTarget::Status
            | ParseTarget::Issue
            | ParseTarget::CreatedIssue
            | ParseTarget::UpdatedIssue
            | ParseTarget::UpdatedIssueFetch
            | ParseTarget::CloseResult
            | ParseTarget::SearchResults => {}
        }
    }
}
