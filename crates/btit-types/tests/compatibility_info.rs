//! JSON shape pins for `CompatibilityInfo`, moved from `crates/btit-app/src/cli.rs`
//! (`compatibility_info_serializes_camel_case`, `compatibility_info_arrays_and_null_tuple_serialize`,
//! `compatibility_info_empty_arrays_serialize_as_empty_not_null`) so the frontend's wire contract
//! stays pinned even though the type now lives in `btit-types`.

use btit_types::CompatibilityInfo;

const MIN_SUPPORTED_BD_MAJOR: u32 = 1;

#[test]
fn compatibility_info_serializes_camel_case() {
    let info = CompatibilityInfo {
        binary: "bd".into(),
        found: true,
        version: "bd version 1.0.4".into(),
        client_type: "bd".into(),
        version_tuple: Some(vec![1, 0, 4]),
        legacy: false,
        min_supported_major: MIN_SUPPORTED_BD_MAJOR,
        supports_daemon_flag: false,
        uses_jsonl_files: false,
        uses_dolt_backend: true,
        supports_list_all_flag: true,
        searched_paths: vec!["/opt/homebrew/bin".into()],
        warnings: vec![],
    };
    let json = serde_json::to_value(&info).unwrap();
    for key in [
        "binary",
        "found",
        "version",
        "clientType",
        "versionTuple",
        "legacy",
        "minSupportedMajor",
        "supportsDaemonFlag",
        "usesJsonlFiles",
        "usesDoltBackend",
        "supportsListAllFlag",
        "searchedPaths",
        "warnings",
    ] {
        assert!(
            json.get(key).is_some(),
            "missing camelCase key {key}: {json}"
        );
    }
    assert!(json.get("client_type").is_none());
}

#[test]
fn compatibility_info_arrays_and_null_tuple_serialize() {
    let info = CompatibilityInfo {
        binary: "bd".into(),
        found: false,
        version: "bd not found".into(),
        client_type: "unknown".into(),
        version_tuple: None,
        legacy: false,
        min_supported_major: MIN_SUPPORTED_BD_MAJOR,
        supports_daemon_flag: false,
        uses_jsonl_files: false,
        uses_dolt_backend: false,
        supports_list_all_flag: false,
        searched_paths: vec!["/a".into(), "/b".into()],
        warnings: vec!["w1".into(), "w2".into()],
    };
    let json = serde_json::to_value(&info).unwrap();
    assert!(json["warnings"].is_array());
    assert_eq!(json["warnings"].as_array().unwrap().len(), 2);
    assert!(json["searchedPaths"].is_array());
    assert_eq!(json["searchedPaths"], serde_json::json!(["/a", "/b"]));
    // Present and null, not omitted: the frontend distinguishes "unknown" from "missing key".
    assert!(json.as_object().unwrap().contains_key("versionTuple"));
    assert!(json["versionTuple"].is_null());
    assert_eq!(json["found"], serde_json::json!(false));
    assert_eq!(json["clientType"], serde_json::json!("unknown"));
    assert_eq!(
        json["minSupportedMajor"],
        serde_json::json!(MIN_SUPPORTED_BD_MAJOR)
    );
}

#[test]
fn compatibility_info_empty_arrays_serialize_as_empty_not_null() {
    let info = CompatibilityInfo {
        binary: "bd".into(),
        found: true,
        version: "bd version 1.0.4".into(),
        client_type: "bd".into(),
        version_tuple: Some(vec![1, 0, 4]),
        legacy: false,
        min_supported_major: MIN_SUPPORTED_BD_MAJOR,
        supports_daemon_flag: false,
        uses_jsonl_files: false,
        uses_dolt_backend: true,
        supports_list_all_flag: true,
        searched_paths: vec![],
        warnings: vec![],
    };
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["warnings"], serde_json::json!([]));
    assert_eq!(json["searchedPaths"], serde_json::json!([]));
    assert_eq!(json["versionTuple"], serde_json::json!([1, 0, 4]));
}
