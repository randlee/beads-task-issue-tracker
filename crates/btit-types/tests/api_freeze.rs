//! Compile-time pin of `btit-types`' public API.
//!
//! Later sprints (b-3..b-8) consume these types through traits; this file fails to
//! compile if a field, variant, or derive this crate depends on is renamed or
//! removed. It is not exhaustive documentation of the crate — it is a contract
//! check.

use btit_types::{
    BackendCapabilities, BdRawComment, BdRawDependency, BdRawDependent, BdRawIssue, ChildIssue,
    CliClient, CliOutput, CliProbe, CliVersion, Comment, CompatibilityInfo, CountResult,
    CreatePayload, CwdOptions, DirectoryEntry, DoltOpResult, FsListResult, Issue, ListOptions,
    ListQuery, ParentIssue, ProjectRef, PurgeResult, Relation, RelationType, ReleaseSource,
    UpdatePayload,
};

// ---- cli.rs -----------------------------------------------------------------

#[test]
fn cli_client_variants_and_derives() {
    let clients = [CliClient::Bd, CliClient::Br, CliClient::Unknown];
    for c in clients {
        // Copy, Eq, Hash
        let copy = c;
        assert_eq!(copy, c);
        let mut set = std::collections::HashSet::new();
        set.insert(c);
        assert!(set.contains(&c));
    }
    // Debug
    assert!(!format!("{:?}", CliClient::Bd).is_empty());
}

#[test]
fn cli_version_fields_ordering_and_conversions() {
    let v = CliVersion {
        major: 1,
        minor: 2,
        patch: 3,
    };
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.to_string(), "1.2.3");
    let tuple: (u32, u32, u32) = v.into();
    assert_eq!(tuple, (1, 2, 3));
    let back: CliVersion = tuple.into();
    assert_eq!(back, v);
    assert!(
        CliVersion {
            major: 1,
            minor: 0,
            patch: 0
        } < CliVersion {
            major: 2,
            minor: 0,
            patch: 0
        }
    );
}

#[test]
fn cli_probe_fields() {
    let _: fn(CliProbe) -> CliClient = |p| p.client;
    let _: fn(CliProbe) -> Option<CliVersion> = |p| p.version;
    let _: fn(CliProbe) -> String = |p| p.raw;
    let p = CliProbe {
        client: CliClient::Bd,
        version: Some(CliVersion {
            major: 1,
            minor: 0,
            patch: 0,
        }),
        raw: "bd version 1.0.0".to_string(),
    };
    let cloned = p.clone();
    assert_eq!(cloned, p);
}

#[test]
fn backend_capabilities_default_and_fields() {
    let caps = BackendCapabilities::default();
    assert!(!caps.supports_daemon_flag);
    assert!(!caps.uses_jsonl_files);
    assert!(!caps.uses_dolt_backend);
    assert!(!caps.supports_list_all_flag);
    assert!(!caps.supports_delete_hard_flag);
    let _: fn(BackendCapabilities) -> bool = |c| c.supports_daemon_flag;
    let _: fn(BackendCapabilities) -> bool = |c| c.uses_jsonl_files;
    let _: fn(BackendCapabilities) -> bool = |c| c.uses_dolt_backend;
    let _: fn(BackendCapabilities) -> bool = |c| c.supports_list_all_flag;
    let _: fn(BackendCapabilities) -> bool = |c| c.supports_delete_hard_flag;
}

#[test]
fn release_source_fields() {
    let r = ReleaseSource {
        api_url: "https://api.example/releases/latest",
        releases_url: "https://example/releases",
    };
    assert!(r.api_url.starts_with("https://"));
    assert!(r.releases_url.starts_with("https://"));
}

#[test]
fn cli_output_fields() {
    let o = CliOutput {
        status: Some(0),
        success: true,
        stdout: "ok".into(),
        stderr: String::new(),
    };
    assert_eq!(o.status, Some(0));
    assert!(o.success);
    assert_eq!(o.stdout, "ok");
    assert!(o.stderr.is_empty());
}

#[test]
fn compatibility_info_fields_and_camel_case_json() {
    let info = CompatibilityInfo {
        binary: "bd".into(),
        found: true,
        version: "bd version 1.0.4".into(),
        client_type: "bd".into(),
        version_tuple: Some(vec![1, 0, 4]),
        legacy: false,
        min_supported_major: 1,
        supports_daemon_flag: false,
        uses_jsonl_files: false,
        uses_dolt_backend: true,
        supports_list_all_flag: true,
        searched_paths: vec!["/opt/homebrew/bin".into()],
        warnings: vec![],
    };
    let json = serde_json::to_value(&info).unwrap();
    assert!(json.get("clientType").is_some());
    assert!(json.get("client_type").is_none());
}

// ---- issue.rs -----------------------------------------------------------------

#[test]
fn bd_raw_dependency_round_trips() {
    let json = r#"{"id":"1","issue_id":"a","depends_on_id":"b","type":"blocks","created_at":"t","created_by":"u"}"#;
    let d: BdRawDependency = serde_json::from_str(json).unwrap();
    assert_eq!(d.dependency_type.as_deref(), Some("blocks"));
    let _ = d.clone();
    let _: fn(BdRawDependency) -> Option<String> = |d| d.id;
}

#[test]
fn bd_raw_dependent_round_trips() {
    let json = r#"{"id":"1","title":"t","status":"open","priority":1,"issue_type":"bug","dependency_type":"blocks"}"#;
    let d: BdRawDependent = serde_json::from_str(json).unwrap();
    assert_eq!(d.priority, Some(1));
    let _ = d.clone();
}

#[test]
fn bd_raw_comment_round_trips() {
    let json =
        r#"{"id":1,"issue_id":"a","author":"me","text":"hi","content":null,"created_at":"t"}"#;
    let c: BdRawComment = serde_json::from_str(json).unwrap();
    assert_eq!(c.author, "me");
    let _ = c.clone();
}

#[test]
fn bd_raw_issue_round_trips() {
    let json = r#"{"id":"a","title":"t","description":null,"status":"open","priority":1,"issue_type":"bug","owner":null,"assignee":null,"labels":null,"created_at":"t","created_by":null,"updated_at":"t","closed_at":null,"close_reason":null,"blocked_by":null,"blocks":null,"comments":null,"external_ref":null,"estimate":null,"design":null,"acceptance_criteria":null,"notes":null,"parent":null,"dependents":null,"dependencies":null,"dependency_count":null,"dependent_count":null,"metadata":null,"spec_id":null,"comment_count":null}"#;
    let issue: BdRawIssue = serde_json::from_str(json).unwrap();
    assert_eq!(issue.id, "a");
    let _ = issue.clone();
}

#[test]
fn issue_camel_case_json() {
    let issue = Issue {
        id: "a".into(),
        title: "t".into(),
        description: String::new(),
        issue_type: "bug".into(),
        status: "open".into(),
        priority: "p1".into(),
        assignee: None,
        labels: vec![],
        created_at: "t".into(),
        updated_at: "t".into(),
        closed_at: None,
        comments: vec![],
        blocked_by: None,
        blocks: None,
        external_ref: None,
        estimate_minutes: None,
        design_notes: None,
        acceptance_criteria: None,
        working_notes: None,
        parent: None,
        children: None,
        relations: None,
        metadata: None,
        spec_id: None,
        comment_count: None,
        dependency_count: None,
        dependent_count: None,
    };
    let json = serde_json::to_value(&issue).unwrap();
    for key in [
        "createdAt",
        "updatedAt",
        "type",
        "externalRef",
        "estimateMinutes",
    ] {
        assert!(json.get(key).is_some(), "missing {key}");
    }
}

#[test]
fn comment_child_parent_relation_fields() {
    let c = Comment {
        id: "1".into(),
        author: "me".into(),
        content: "hi".into(),
        created_at: "t".into(),
    };
    assert_eq!(serde_json::to_value(&c).unwrap()["createdAt"], "t");

    let child = ChildIssue {
        id: "a".into(),
        title: "t".into(),
        status: "open".into(),
        priority: "p1".into(),
    };
    assert_eq!(child.id, "a");

    let parent = ParentIssue {
        id: "a".into(),
        title: "t".into(),
        status: "open".into(),
        priority: "p1".into(),
    };
    assert_eq!(parent.id, "a");

    let rel = Relation {
        id: "a".into(),
        title: "t".into(),
        status: "open".into(),
        priority: "p1".into(),
        relation_type: "blocks".into(),
        direction: "dependency".into(),
    };
    assert_eq!(
        serde_json::to_value(&rel).unwrap()["relationType"],
        "blocks"
    );
}

#[test]
fn count_result_camel_case_json() {
    let cr = CountResult {
        count: 1,
        by_type: std::collections::HashMap::new(),
        by_priority: std::collections::HashMap::new(),
        last_updated: None,
    };
    let json = serde_json::to_value(&cr).unwrap();
    assert!(json.get("byType").is_some());
    assert!(json.get("byPriority").is_some());
    assert!(json.get("lastUpdated").is_some());
}

// ---- fs.rs -----------------------------------------------------------------

#[test]
fn directory_entry_and_fs_list_result_camel_case_json() {
    let entry = DirectoryEntry {
        name: "n".into(),
        path: "/p".into(),
        is_directory: true,
        has_beads: false,
        uses_dolt: false,
    };
    let json = serde_json::to_value(&entry).unwrap();
    assert!(json.get("isDirectory").is_some());
    assert!(json.get("hasBeads").is_some());
    assert!(json.get("usesDolt").is_some());

    let list = FsListResult {
        current_path: "/p".into(),
        has_beads: false,
        uses_dolt: false,
        entries: vec![],
    };
    let json = serde_json::to_value(&list).unwrap();
    assert!(json.get("currentPath").is_some());
}

#[test]
fn purge_result_camel_case_json() {
    let p = PurgeResult {
        deleted_count: 2,
        deleted_folders: vec!["/a".into()],
    };
    let json = serde_json::to_value(&p).unwrap();
    assert!(json.get("deletedCount").is_some());
    assert!(json.get("deletedFolders").is_some());
}

// ---- payload.rs -----------------------------------------------------------------

#[test]
fn list_options_flatten_round_trip_matches_wire_shape() {
    let json = r#"{"status":["open"],"type":["bug"],"priority":["p1"],"assignee":"a","includeAll":true,"cwd":"/p"}"#;
    let options: ListOptions = serde_json::from_str(json).unwrap();
    assert_eq!(options.query.status, Some(vec!["open".to_string()]));
    assert_eq!(options.query.issue_type, Some(vec!["bug".to_string()]));
    assert_eq!(options.query.priority, Some(vec!["p1".to_string()]));
    assert_eq!(options.query.assignee.as_deref(), Some("a"));
    assert_eq!(options.query.include_all, Some(true));
    assert_eq!(options.cwd.as_deref(), Some("/p"));
}

#[test]
fn list_options_without_include_all_is_none() {
    let json = r#"{"cwd":"/p"}"#;
    let options: ListOptions = serde_json::from_str(json).unwrap();
    assert_eq!(options.query.include_all, None);
    assert_eq!(options.query.status, None);
}

#[test]
fn list_query_default_is_all_none() {
    let q = ListQuery::default();
    assert_eq!(
        q,
        ListQuery {
            status: None,
            issue_type: None,
            priority: None,
            assignee: None,
            include_all: None
        }
    );
}

#[test]
fn cwd_options_default_and_field() {
    let o = CwdOptions::default();
    assert_eq!(o.cwd, None);
    let _: fn(CwdOptions) -> Option<String> = |o| o.cwd;
}

#[test]
fn create_payload_fields_deserialize() {
    let json = r#"{"title":"t","cwd":null}"#;
    let p: CreatePayload = serde_json::from_str(json).unwrap();
    assert_eq!(p.title, "t");
    let _: fn(CreatePayload) -> Option<String> = |p| p.spec_id;
}

#[test]
fn update_payload_fields_deserialize() {
    let json = r#"{"title":"t"}"#;
    let p: UpdatePayload = serde_json::from_str(json).unwrap();
    assert_eq!(p.title.as_deref(), Some("t"));
    let _: fn(UpdatePayload) -> Option<String> = |p| p.metadata;
}

// ---- backend.rs -----------------------------------------------------------------

#[test]
fn project_ref_local_constructor() {
    let r = ProjectRef::local(Some("/p".into()));
    if let ProjectRef::Local { cwd } = r {
        assert_eq!(cwd.as_deref(), Some("/p"));
    } else {
        panic!("ProjectRef only has the Local variant today");
    }
}

#[test]
fn relation_type_fields_and_json() {
    let rt = RelationType {
        value: "blocks",
        label: "Blocks",
    };
    let json = serde_json::to_value(&rt).unwrap();
    assert_eq!(json["value"], "blocks");
    assert_eq!(json["label"], "Blocks");
}

#[test]
fn dolt_op_result_fields() {
    let r = DoltOpResult {
        success: true,
        message: "ok".into(),
        detail: String::new(),
    };
    assert!(r.success);
    assert_eq!(r.message, "ok");
    assert!(r.detail.is_empty());
}
