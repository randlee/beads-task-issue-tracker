use crate::types::{BdRawIssue, ChildIssue, Comment, Issue, ParentIssue, Relation};

pub(crate) fn priority_to_string(priority: i32) -> String {
    let p = if (0..=4).contains(&priority) { priority } else { 3 };
    format!("p{}", p)
}

pub(crate) fn priority_to_number(priority: &str) -> String {
    if let Some(caps) = priority.strip_prefix('p') {
        if caps.len() == 1 && caps.chars().next().unwrap_or('x').is_ascii_digit() {
            return caps.to_string();
        }
    }
    "3".to_string()
}

pub(crate) fn normalize_issue_type(issue_type: &str) -> String {
    let valid_types = ["bug", "task", "feature", "epic", "chore"];
    if valid_types.contains(&issue_type) {
        issue_type.to_string()
    } else {
        "task".to_string()
    }
}

pub(crate) fn normalize_issue_status(status: &str) -> String {
    let valid_statuses = ["open", "in_progress", "blocked", "closed", "deferred", "tombstone", "pinned", "hooked"];
    if valid_statuses.contains(&status) {
        status.to_string()
    } else {
        "open".to_string()
    }
}

pub(crate) fn transform_issue(raw: BdRawIssue) -> Issue {
    // Parent info - dependencies array now contains relationship info, not full issue details
    // For now, we just use the parent ID if available
    let parent = raw.parent.as_ref().map(|parent_id| {
        ParentIssue {
            id: parent_id.clone(),
            title: String::new(), // Not available in dependency format
            status: "open".to_string(),
            priority: "p3".to_string(),
        }
    });

    // Extract children from dependents array (with dependency_type: "parent-child")
    let children: Option<Vec<ChildIssue>> = raw.dependents.as_ref().map(|deps| {
        deps.iter()
            .filter(|d| d.dependency_type.as_deref() == Some("parent-child") && d.id.is_some())
            .map(|c| ChildIssue {
                id: c.id.clone().unwrap_or_default(),
                title: c.title.clone().unwrap_or_default(),
                status: normalize_issue_status(&c.status.clone().unwrap_or_else(|| "open".to_string())),
                priority: priority_to_string(c.priority.unwrap_or(3)),
            })
            .collect()
    }).filter(|v: &Vec<ChildIssue>| !v.is_empty());

    // Extract non-blocking relations (everything except "blocks" and "parent-child")
    let structural_types = ["blocks", "parent-child"];
    let mut relations: Vec<Relation> = Vec::new();
    let mut seen_relations: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

    // From dependencies array (these are issues the current issue depends on)
    if let Some(ref deps) = raw.dependencies {
        for dep in deps {
            if let Some(ref dep_type) = dep.dependency_type {
                if structural_types.contains(&dep_type.as_str()) {
                    continue;
                }
                let id = dep.id.clone().or_else(|| dep.depends_on_id.clone()).unwrap_or_default();
                if id.is_empty() {
                    continue;
                }
                let key = (id.clone(), dep_type.clone());
                if !seen_relations.contains(&key) {
                    seen_relations.insert(key);
                    relations.push(Relation {
                        id,
                        title: String::new(),
                        status: String::new(),
                        priority: String::new(),
                        relation_type: dep_type.clone(),
                        direction: "dependency".to_string(),
                    });
                }
            }
        }
    }

    // From dependents array (these are issues that depend on the current issue — has full metadata)
    if let Some(ref dependents) = raw.dependents {
        for dep in dependents {
            if let Some(ref dep_type) = dep.dependency_type {
                if structural_types.contains(&dep_type.as_str()) {
                    continue;
                }
                let id = dep.id.clone().unwrap_or_default();
                if id.is_empty() {
                    continue;
                }
                let key = (id.clone(), dep_type.clone());
                if seen_relations.contains(&key) {
                    // Replace existing entry from dependencies if this one has more metadata
                    if dep.title.is_some() {
                        if let Some(existing) = relations.iter_mut().find(|r| r.id == id && r.relation_type == *dep_type) {
                            existing.title = dep.title.clone().unwrap_or_default();
                            existing.status = normalize_issue_status(&dep.status.clone().unwrap_or_else(|| "open".to_string()));
                            existing.priority = priority_to_string(dep.priority.unwrap_or(3));
                            existing.direction = "dependent".to_string();
                        }
                    }
                } else {
                    seen_relations.insert(key);
                    relations.push(Relation {
                        id,
                        title: dep.title.clone().unwrap_or_default(),
                        status: normalize_issue_status(&dep.status.clone().unwrap_or_else(|| "open".to_string())),
                        priority: priority_to_string(dep.priority.unwrap_or(3)),
                        relation_type: dep_type.clone(),
                        direction: "dependent".to_string(),
                    });
                }
            }
        }
    }

    // Compute comment_count before consuming raw.comments
    let comment_count = raw.comment_count.or_else(|| {
        raw.comments.as_ref().map(|c| c.len() as i32)
    });

    Issue {
        id: raw.id,
        title: raw.title,
        description: raw.description.unwrap_or_default(),
        issue_type: normalize_issue_type(&raw.issue_type),
        status: normalize_issue_status(&raw.status),
        priority: priority_to_string(raw.priority),
        assignee: raw.assignee,
        labels: raw.labels.unwrap_or_default(),
        created_at: raw.created_at,
        updated_at: raw.updated_at,
        closed_at: raw.closed_at,
        comments: raw.comments.unwrap_or_default().into_iter().map(|c| {
            Comment {
                id: match c.id {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s,
                    _ => "0".to_string(),
                },
                author: c.author,
                content: c.text.or(c.content).unwrap_or_default(),
                created_at: c.created_at,
            }
        }).collect(),
        blocked_by: {
            // Try raw.blocked_by first (if bd ever populates it directly)
            let mut bb = raw.blocked_by.unwrap_or_default();
            // Extract from dependencies array (bd show: objects with dependency_type "blocks" = blockers)
            if let Some(ref deps) = raw.dependencies {
                // bd show format: [{id, dependency_type: "blocks"}] — these block the current issue
                for dep in deps {
                    if let (Some(ref dep_type), Some(ref id)) = (&dep.dependency_type, &dep.id) {
                        if dep_type == "blocks" && !bb.contains(id) {
                            bb.push(id.clone());
                        }
                    }
                    // bd list format: [{issue_id, depends_on_id, type: "blocks"}]
                    if let (Some(ref dep_type), Some(ref depends_on_id), Some(ref _issue_id)) = (&dep.dependency_type, &dep.depends_on_id, &dep.issue_id) {
                        if dep_type == "blocks" && !bb.contains(depends_on_id) {
                            bb.push(depends_on_id.clone());
                        }
                    }
                }
            }
            if bb.is_empty() { None } else { Some(bb) }
        },
        blocks: {
            let mut bl = raw.blocks.unwrap_or_default();
            // Extract from dependents array (bd show: objects with dependency_type "blocks" = issues blocked by current)
            // Filter to only "blocks" type — exclude "parent-child" which are children, not dependencies
            if let Some(ref dependents) = raw.dependents {
                for dep in dependents {
                    if let (Some(ref dep_type), Some(ref id)) = (&dep.dependency_type, &dep.id) {
                        if dep_type == "blocks" && !bl.contains(id) {
                            bl.push(id.clone());
                        }
                    }
                }
            }
            if bl.is_empty() { None } else { Some(bl) }
        },
        external_ref: raw.external_ref,
        estimate_minutes: raw.estimate,
        design_notes: raw.design,
        acceptance_criteria: raw.acceptance_criteria,
        working_notes: raw.notes,
        parent,
        children,
        relations: if relations.is_empty() { None } else { Some(relations) },
        metadata: normalize_metadata(raw.metadata),
        spec_id: raw.spec_id,
        comment_count,
        dependency_count: raw.dependency_count.or_else(|| {
            raw.dependencies.as_ref().map(|d| d.len() as i32)
        }),
        dependent_count: raw.dependent_count.or_else(|| {
            raw.dependents.as_ref().map(|d| d.len() as i32)
        }),
    }
}

/// Normalize the `metadata` blob bd returns.
/// - bd 0.49+ and 1.x emit a JSON object (`json.RawMessage`), which is passed through
/// - a JSON *string* (legacy payloads written via `--metadata '<json>'` on old versions,
///   or data round-tripped through the app) is parsed if it holds an object
/// - `null`, empty objects, and empty/whitespace strings become `None` so the UI can
///   use presence as "has custom fields"
pub(crate) fn normalize_metadata(value: Option<serde_json::Value>) -> Option<serde_json::Value> {
    use serde_json::Value;
    match value {
        None | Some(Value::Null) => None,
        Some(Value::Object(map)) if map.is_empty() => None,
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return None;
            }
            match serde_json::from_str::<Value>(trimmed) {
                Ok(Value::Object(map)) if map.is_empty() => None,
                Ok(parsed @ Value::Object(_)) => Some(parsed),
                // Not an object: keep the original string so nothing is silently dropped
                _ => Some(Value::String(s)),
            }
        }
        other => other,
    }
}

/// Parse issues with tolerance for malformed entries
/// Returns all successfully parsed issues and logs failures
pub(crate) fn parse_issues_tolerant(output: &str, context: &str) -> Result<Vec<BdRawIssue>, String> {
    // First try strict parsing
    if let Ok(issues) = serde_json::from_str::<Vec<BdRawIssue>>(output) {
        return Ok(issues);
    }

    // If strict parsing fails, try tolerant parsing
    log_warn!("[{}] Strict parsing failed, attempting tolerant parsing", context);

    let value: serde_json::Value = serde_json::from_str(output)
        .map_err(|e| {
            log_error!("[{}] JSON is completely invalid: {}", context, e);
            format!("Invalid JSON: {}", e)
        })?;

    // br >= 0.1.30 wraps `list` output in a paginated envelope:
    // {"issues": [...], "total": N, "offset": N, "limit": N, "has_more": bool}
    // Unwrap the envelope if present, otherwise expect a flat array.
    let arr_value;
    let arr = if let Some(obj) = value.as_object() {
        if let Some(issues) = obj.get("issues").and_then(|v| v.as_array()) {
            log_info!("[{}] Unwrapped paginated envelope ({} issues)", context, issues.len());
            arr_value = issues.clone();
            &arr_value
        } else {
            log_error!("[{}] Expected array or envelope with 'issues' key, got object: {:?}", context, obj.keys().collect::<Vec<_>>());
            return Err("Expected JSON array or paginated envelope".to_string());
        }
    } else {
        value.as_array().ok_or_else(|| {
            log_error!("[{}] Expected array, got: {:?}", context, value);
            "Expected JSON array".to_string()
        })?
    };

    let mut issues = Vec::new();
    let mut failed_count = 0;

    for (i, obj) in arr.iter().enumerate() {
        let obj_str = serde_json::to_string(obj).unwrap_or_default();
        match serde_json::from_str::<BdRawIssue>(&obj_str) {
            Ok(issue) => issues.push(issue),
            Err(e) => {
                failed_count += 1;
                let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                log_error!("[{}] Skipping issue {} (id={}): {}", context, i, id, e);

                // Log which fields are present/missing
                if let Some(obj_map) = obj.as_object() {
                    let keys: Vec<&str> = obj_map.keys().map(|s| s.as_str()).collect();
                    log_error!("[{}] Issue {} has keys: {:?}", context, i, keys);

                    // Check for common missing required fields
                    let required = ["id", "title", "status", "priority", "issue_type", "created_at", "updated_at"];
                    let missing: Vec<&&str> = required.iter().filter(|k| !keys.contains(*k)).collect();
                    if !missing.is_empty() {
                        log_error!("[{}] Issue {} missing required fields: {:?}", context, i, missing);
                    }
                }
            }
        }
    }

    if failed_count > 0 {
        log_warn!("[{}] Parsed {} issues, skipped {} malformed entries", context, issues.len(), failed_count);
    }

    Ok(issues)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    // ---- metadata normalization (#7) ----------------------------------------

    #[test]
    fn issue_with_object_metadata_is_not_skipped() {
        // bd emits metadata as a JSON object; this used to fail Option<String> and drop the issue
        let json = format!(
            "[{}]",
            issue_json_with_metadata("m-1", r#"{"project":"gold","refs":["[[Note]]"],"nested":{"a":1},"percent_complete":56}"#)
        );
        let issues = parse_issues_tolerant(&json, "test").unwrap();
        assert_eq!(issues.len(), 1, "issue with object metadata must be kept");
        let issue = transform_issue(issues.into_iter().next().unwrap());
        let meta = issue.metadata.expect("metadata present");
        assert_eq!(meta["project"], "gold");
        assert_eq!(meta["refs"][0], "[[Note]]");
        assert_eq!(meta["nested"]["a"], 1);
        assert_eq!(meta["percent_complete"], 56);
    }

    #[test]
    fn bd_show_shape_with_object_metadata_deserializes() {
        let v: serde_json::Value = serde_json::from_str(&issue_json_with_metadata("m-2", r#"{"k":"v"}"#)).unwrap();
        let raw: BdRawIssue = serde_json::from_value(v).expect("bd show payload with object metadata");
        assert_eq!(raw.metadata.unwrap()["k"], "v");
    }

    #[test]
    fn string_metadata_holding_json_object_is_parsed() {
        // metadata is a JSON *string* whose content is an object (legacy payload shape)
        let json = format!("[{}]", issue_json_with_metadata("m-3", r#""{\"project\":\"iron\"}""#));
        let issue = transform_issue(parse_issues_tolerant(&json, "test").unwrap().remove(0));
        assert_eq!(issue.metadata.unwrap()["project"], "iron");
    }

    #[test]
    fn null_empty_and_blank_metadata_normalize_to_none() {
        use serde_json::json;
        assert_eq!(normalize_metadata(None), None);
        assert_eq!(normalize_metadata(Some(json!(null))), None);
        assert_eq!(normalize_metadata(Some(json!({}))), None);
        assert_eq!(normalize_metadata(Some(json!(""))), None);
        assert_eq!(normalize_metadata(Some(json!("   "))), None);
        assert_eq!(normalize_metadata(Some(json!("{}"))), None);
    }

    #[test]
    fn non_object_metadata_is_preserved_not_dropped() {
        use serde_json::json;
        assert_eq!(normalize_metadata(Some(json!("free text"))), Some(json!("free text")));
        assert_eq!(normalize_metadata(Some(json!("[1,2]"))), Some(json!("[1,2]")));
        assert_eq!(normalize_metadata(Some(json!([1, 2]))), Some(json!([1, 2])));
        assert_eq!(normalize_metadata(Some(json!(42))), Some(json!(42)));
    }

    #[test]
    fn normalized_issue_serializes_metadata_as_object() {
        let json = format!("[{}]", issue_json_with_metadata("m-4", r#"{"a":1}"#));
        let issue = transform_issue(parse_issues_tolerant(&json, "test").unwrap().remove(0));
        let out = serde_json::to_value(&issue).unwrap();
        assert!(out["metadata"].is_object(), "frontend must receive an object, got {}", out["metadata"]);
        // Absent metadata serializes as null
        let issue = transform_issue(parse_issues_tolerant(&format!("[{}]", minimal_issue_json("m-5", "x")), "test").unwrap().remove(0));
        assert!(serde_json::to_value(&issue).unwrap()["metadata"].is_null());
    }

    #[test]
    fn parse_flat_array() {
        let json = format!("[{}]", minimal_issue_json("abc-123", "Bug fix"));
        let result = parse_issues_tolerant(&json, "test_flat");
        assert!(result.is_ok());
        let issues = result.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "abc-123");
        assert_eq!(issues[0].title, "Bug fix");
    }

    #[test]
    fn parse_paginated_envelope() {
        let json = format!(
            r#"{{"issues":[{},{}],"total":2,"offset":0,"limit":50,"has_more":false}}"#,
            minimal_issue_json("abc-123", "First"),
            minimal_issue_json("def-456", "Second")
        );
        let result = parse_issues_tolerant(&json, "test_envelope");
        assert!(result.is_ok());
        let issues = result.unwrap();
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].id, "abc-123");
        assert_eq!(issues[1].id, "def-456");
    }

    #[test]
    fn parse_empty_flat_array() {
        let result = parse_issues_tolerant("[]", "test_empty_flat");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn parse_empty_envelope() {
        let json = r#"{"issues":[],"total":0,"offset":0,"limit":50,"has_more":false}"#;
        let result = parse_issues_tolerant(json, "test_empty_envelope");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn parse_object_without_issues_key_fails() {
        let json = r#"{"error":"something went wrong"}"#;
        let result = parse_issues_tolerant(json, "test_bad_object");
        assert!(result.is_err());
    }

    #[test]
    fn parse_invalid_json_fails() {
        let result = parse_issues_tolerant("not json at all", "test_invalid");
        assert!(result.is_err());
    }

    #[test]
    fn parse_real_br_envelope() {
        // Matches the shape from br 0.1.30+ (`br list --json --limit 1`):
        // br omits many optional fields (owner, assignee, labels, etc.) and includes extra
        // fields (source_repo, compaction_level). serde_json defaults missing Option<T> to None
        // and ignores unknown fields, so this parses correctly.
        let json = r#"{"issues":[{"id":"proj-abc","title":"Example bug report","description":"A test description","status":"open","priority":2,"issue_type":"bug","created_at":"2025-06-15T09:30:00.000000000Z","updated_at":"2025-06-15T10:45:00.000000000Z","source_repo":".","compaction_level":0,"dependency_count":0,"dependent_count":0}],"total":1,"limit":1,"offset":0,"has_more":true}"#;
        let result = parse_issues_tolerant(json, "test_real_br");
        assert!(result.is_ok());
        let issues = result.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "proj-abc");
        assert_eq!(issues[0].issue_type, "bug");
        assert_eq!(issues[0].priority, 2);
    }

    #[test]
    fn parse_envelope_skips_malformed_entries() {
        let good = minimal_issue_json("abc-123", "Good");
        let bad = r#"{"id":"bad-456","title":"Bad"}"#; // missing required fields
        let json = format!(r#"{{"issues":[{},{}],"total":2,"offset":0,"limit":50,"has_more":false}}"#, good, bad);
        let result = parse_issues_tolerant(&json, "test_tolerant_envelope");
        assert!(result.is_ok());
        let issues = result.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "abc-123");
    }

    // ---- Issue normalizers (#1) -------------------------------------------------

    #[test]
    fn priority_to_string_normalizes_valid_range() {
        assert_eq!(priority_to_string(0), "p0");
        assert_eq!(priority_to_string(1), "p1");
        assert_eq!(priority_to_string(2), "p2");
        assert_eq!(priority_to_string(3), "p3");
        assert_eq!(priority_to_string(4), "p4");
    }

    #[test]
    fn priority_to_string_defaults_out_of_range() {
        assert_eq!(priority_to_string(-1), "p3");
        assert_eq!(priority_to_string(5), "p3");
        assert_eq!(priority_to_string(100), "p3");
        assert_eq!(priority_to_string(i32::MIN), "p3");
        assert_eq!(priority_to_string(i32::MAX), "p3");
    }

    #[test]
    fn priority_to_number_round_trips_valid_strings() {
        assert_eq!(priority_to_number("p0"), "0");
        assert_eq!(priority_to_number("p1"), "1");
        assert_eq!(priority_to_number("p2"), "2");
        assert_eq!(priority_to_number("p3"), "3");
        assert_eq!(priority_to_number("p4"), "4");
    }

    #[test]
    fn priority_to_number_defaults_invalid_inputs() {
        assert_eq!(priority_to_number(""), "3");
        assert_eq!(priority_to_number("p"), "3");
        assert_eq!(priority_to_number("p10"), "3");
        assert_eq!(priority_to_number("pa"), "3");
        assert_eq!(priority_to_number("priority"), "3");
        assert_eq!(priority_to_number("unknown"), "3");
        assert_eq!(priority_to_number("5"), "3");
    }

    #[test]
    fn priority_round_trip() {
        for i in 0..=4 {
            let as_string = priority_to_string(i);
            let back = priority_to_number(&as_string);
            assert_eq!(back, i.to_string(), "priority {} should round-trip", i);
        }
    }

    #[test]
    fn normalize_issue_type_accepts_valid_types() {
        assert_eq!(normalize_issue_type("bug"), "bug");
        assert_eq!(normalize_issue_type("task"), "task");
        assert_eq!(normalize_issue_type("feature"), "feature");
        assert_eq!(normalize_issue_type("epic"), "epic");
        assert_eq!(normalize_issue_type("chore"), "chore");
    }

    #[test]
    fn normalize_issue_type_defaults_unknown() {
        assert_eq!(normalize_issue_type(""), "task");
        assert_eq!(normalize_issue_type("unknown"), "task");
        assert_eq!(normalize_issue_type("Bug"), "task"); // case-sensitive
        assert_eq!(normalize_issue_type("improvement"), "task");
    }

    #[test]
    fn normalize_issue_status_accepts_valid_statuses() {
        assert_eq!(normalize_issue_status("open"), "open");
        assert_eq!(normalize_issue_status("in_progress"), "in_progress");
        assert_eq!(normalize_issue_status("blocked"), "blocked");
        assert_eq!(normalize_issue_status("closed"), "closed");
        assert_eq!(normalize_issue_status("deferred"), "deferred");
        assert_eq!(normalize_issue_status("tombstone"), "tombstone");
        assert_eq!(normalize_issue_status("pinned"), "pinned");
        assert_eq!(normalize_issue_status("hooked"), "hooked");
    }

    #[test]
    fn normalize_issue_status_defaults_unknown() {
        assert_eq!(normalize_issue_status(""), "open");
        assert_eq!(normalize_issue_status("unknown"), "open");
        assert_eq!(normalize_issue_status("Closed"), "open"); // case-sensitive
        assert_eq!(normalize_issue_status("in-progress"), "open");
    }

    // ---- transform_issue branches (#2) ------------------------------------------

    #[test]
    fn transform_issue_extracts_blocked_by_from_dependencies() {
        let json = minimal_issue_json("test-1", "Test");
        let json = json.replace(
            r#""dependencies":null"#,
            r#""dependencies":[{"id":"test-2","dependency_type":"blocks"}]"#
        );
        let issues = parse_issues_tolerant(&format!("[{}]", json), "test").unwrap();
        let issue = transform_issue(issues.into_iter().next().unwrap());
        assert_eq!(issue.blocked_by, Some(vec!["test-2".to_string()]));
    }

    #[test]
    fn transform_issue_extracts_blocks_from_dependents() {
        let json = minimal_issue_json("test-1", "Test");
        let json = json.replace(
            r#""dependents":null"#,
            r#""dependents":[{"id":"test-2","dependency_type":"blocks","status":"open","priority":3}]"#
        );
        let issues = parse_issues_tolerant(&format!("[{}]", json), "test").unwrap();
        let issue = transform_issue(issues.into_iter().next().unwrap());
        assert_eq!(issue.blocks, Some(vec!["test-2".to_string()]));
    }

    #[test]
    fn transform_issue_extracts_children_from_parent_child_dependents() {
        let json = minimal_issue_json("test-1", "Test");
        let json = json.replace(
            r#""dependents":null"#,
            r#""dependents":[{"id":"test-child","title":"Child Issue","dependency_type":"parent-child","status":"open","priority":2}]"#
        );
        let issues = parse_issues_tolerant(&format!("[{}]", json), "test").unwrap();
        let issue = transform_issue(issues.into_iter().next().unwrap());
        assert!(issue.children.is_some());
        let children = issue.children.unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, "test-child");
        assert_eq!(children[0].title, "Child Issue");
    }

    #[test]
    fn transform_issue_extracts_parent() {
        let json = minimal_issue_json("test-1", "Test");
        let json = json.replace(r#""parent":null"#, r#""parent":"test-parent""#);
        let issues = parse_issues_tolerant(&format!("[{}]", json), "test").unwrap();
        let issue = transform_issue(issues.into_iter().next().unwrap());
        assert!(issue.parent.is_some());
        let parent = issue.parent.unwrap();
        assert_eq!(parent.id, "test-parent");
    }

    #[test]
    fn transform_issue_extracts_comment_count_from_array() {
        let json = minimal_issue_json("test-1", "Test");
        let json = json.replace(
            r#""comments":null"#,
            r#""comments":[{"id":1,"author":"user1","content":"comment1","created_at":"2025-01-01T00:00:00Z"}]"#
        );
        let issues = parse_issues_tolerant(&format!("[{}]", json), "test").unwrap();
        let issue = transform_issue(issues.into_iter().next().unwrap());
        assert_eq!(issue.comment_count, Some(1));
    }

    #[test]
    fn transform_issue_uses_explicit_comment_count_over_array() {
        let json = minimal_issue_json("test-1", "Test");
        let json = json.replace(r#""comment_count":null"#, r#""comment_count":5"#);
        let json = json.replace(
            r#""comments":null"#,
            r#""comments":[{"id":1,"author":"user1","content":"comment1","created_at":"2025-01-01T00:00:00Z"}]"#
        );
        let issues = parse_issues_tolerant(&format!("[{}]", json), "test").unwrap();
        let issue = transform_issue(issues.into_iter().next().unwrap());
        assert_eq!(issue.comment_count, Some(5));
    }

    #[test]
    fn transform_issue_dedupes_blocked_by() {
        let json = minimal_issue_json("test-1", "Test");
        let json = json.replace(
            r#""blocked_by":null"#,
            r#""blocked_by":["test-2"]"#
        );
        let json = json.replace(
            r#""dependencies":null"#,
            r#""dependencies":[{"id":"test-2","dependency_type":"blocks"}]"#
        );
        let issues = parse_issues_tolerant(&format!("[{}]", json), "test").unwrap();
        let issue = transform_issue(issues.into_iter().next().unwrap());
        // Should not duplicate test-2
        assert_eq!(issue.blocked_by, Some(vec!["test-2".to_string()]));
    }

    #[test]
    fn transform_issue_extracts_relations() {
        let json = minimal_issue_json("test-1", "Test");
        let json = json.replace(
            r#""dependencies":null"#,
            r#""dependencies":[{"id":"test-2","dependency_type":"related-to"}]"#
        );
        let issues = parse_issues_tolerant(&format!("[{}]", json), "test").unwrap();
        let issue = transform_issue(issues.into_iter().next().unwrap());
        assert!(issue.relations.is_some());
        let relations = issue.relations.unwrap();
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].id, "test-2");
        assert_eq!(relations[0].relation_type, "related-to");
    }


}
