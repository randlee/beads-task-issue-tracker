use crate::attachments::issue_short_id;
use crate::cli::{execute_bd, get_cli_client_info, supports_delete_hard_flag, supports_list_all_flag};
use crate::issues::{parse_issues_tolerant, priority_to_number, transform_issue};
use crate::migration::sync_bd_database;
use crate::types::*;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[tauri::command]
pub(crate) async fn bd_list(options: ListOptions) -> Result<Vec<Issue>, String> {
    log_info!("[bd_list] cwd: {:?}", options.cwd);

    // Sync database before reading to ensure data is up-to-date
    sync_bd_database(options.cwd.as_deref());

    let mut args: Vec<String> = Vec::new();

    // --all flag only works correctly on bd >= 0.55; for older versions, fallback to 2 calls
    let use_all = options.include_all.unwrap_or(false);
    if use_all && !supports_list_all_flag() {
        // Fallback: fetch open + closed separately and merge
        log_info!("[bd_list] --all requested but bd < 0.55 — falling back to 2 calls");
        let mut fallback_args = args.clone();
        fallback_args.push("--limit=0".to_string());

        let open_output = execute_bd("list", &fallback_args, options.cwd.as_deref())?;
        let open_issues = parse_issues_tolerant(&open_output, "bd_list_open")?;

        fallback_args.push("--status=closed".to_string());
        let closed_output = execute_bd("list", &fallback_args, options.cwd.as_deref())?;
        let closed_issues = parse_issues_tolerant(&closed_output, "bd_list_closed")?;

        let mut all_issues = open_issues;
        all_issues.extend(closed_issues);
        log_info!("[bd_list] Found {} issues (fallback)", all_issues.len());
        return Ok(all_issues.into_iter().map(transform_issue).collect());
    }

    if use_all {
        args.push("--all".to_string());
    }
    if let Some(ref statuses) = options.status {
        if !statuses.is_empty() {
            args.push(format!("--status={}", statuses.join(",")));
        }
    }
    if let Some(ref types) = options.issue_type {
        if !types.is_empty() {
            args.push(format!("--type={}", types.join(",")));
        }
    }
    if let Some(ref priorities) = options.priority {
        if !priorities.is_empty() {
            let nums: Vec<String> = priorities.iter().map(|p| priority_to_number(p)).collect();
            args.push(format!("--priority={}", nums.join(",")));
        }
    }
    if let Some(ref assignee) = options.assignee {
        args.push(format!("--assignee={}", assignee));
    }

    // Always disable limit to get all issues (bd defaults to 50)
    args.push("--limit=0".to_string());

    let output = execute_bd("list", &args, options.cwd.as_deref())?;

    let raw_issues = parse_issues_tolerant(&output, "bd_list")?;

    log_info!("[bd_list] Found {} issues", raw_issues.len());
    Ok(raw_issues.into_iter().map(transform_issue).collect())
}

#[tauri::command]
pub(crate) async fn bd_count(options: CwdOptions) -> Result<CountResult, String> {
    // Sync database before reading to ensure data is up-to-date
    sync_bd_database(options.cwd.as_deref());

    // Fetch all issues: single --all call for bd >= 0.55, fallback to 2 calls for older versions
    let raw_issues = if supports_list_all_flag() {
        let all_output = execute_bd("list", &["--all".to_string(), "--limit=0".to_string()], options.cwd.as_deref())?;
        parse_issues_tolerant(&all_output, "bd_count_all")?
    } else {
        let open_output = execute_bd("list", &["--limit=0".to_string()], options.cwd.as_deref())?;
        let closed_output = execute_bd("list", &["--status=closed".to_string(), "--limit=0".to_string()], options.cwd.as_deref())?;
        let mut issues = parse_issues_tolerant(&open_output, "bd_count_open")?;
        issues.extend(parse_issues_tolerant(&closed_output, "bd_count_closed")?);
        issues
    };

    let mut by_type: HashMap<String, usize> = HashMap::new();
    by_type.insert("bug".to_string(), 0);
    by_type.insert("task".to_string(), 0);
    by_type.insert("feature".to_string(), 0);
    by_type.insert("epic".to_string(), 0);
    by_type.insert("chore".to_string(), 0);

    let mut by_priority: HashMap<String, usize> = HashMap::new();
    by_priority.insert("p0".to_string(), 0);
    by_priority.insert("p1".to_string(), 0);
    by_priority.insert("p2".to_string(), 0);
    by_priority.insert("p3".to_string(), 0);
    by_priority.insert("p4".to_string(), 0);

    let mut last_updated: Option<String> = None;

    for issue in &raw_issues {
        let issue_type = issue.issue_type.to_lowercase();
        if by_type.contains_key(&issue_type) {
            *by_type.get_mut(&issue_type).unwrap() += 1;
        }

        let priority_key = format!("p{}", issue.priority);
        if by_priority.contains_key(&priority_key) {
            *by_priority.get_mut(&priority_key).unwrap() += 1;
        }

        if last_updated.is_none() || issue.updated_at > *last_updated.as_ref().unwrap() {
            last_updated = Some(issue.updated_at.clone());
        }
    }

    Ok(CountResult {
        count: raw_issues.len(),
        by_type,
        by_priority,
        last_updated,
    })
}

#[tauri::command]
pub(crate) async fn bd_ready(options: CwdOptions) -> Result<Vec<Issue>, String> {
    log_info!("[bd_ready] Called with cwd: {:?}", options.cwd);

    // Sync database before reading to ensure data is up-to-date
    sync_bd_database(options.cwd.as_deref());

    let output = execute_bd("ready", &[], options.cwd.as_deref())?;

    let raw_issues = parse_issues_tolerant(&output, "bd_ready")?;

    log_info!("[bd_ready] Found {} ready issues", raw_issues.len());
    Ok(raw_issues.into_iter().map(transform_issue).collect())
}

#[tauri::command]
pub(crate) async fn bd_status(options: CwdOptions) -> Result<serde_json::Value, String> {
    let output = execute_bd("status", &[], options.cwd.as_deref())?;

    serde_json::from_str(&output)
        .map_err(|e| format!("Failed to parse status: {}", e))
}

#[tauri::command]
pub(crate) async fn bd_show(id: String, options: CwdOptions) -> Result<Option<Issue>, String> {
    log_info!("[bd_show] Called for issue: {} with cwd: {:?}", id, options.cwd);

    // Sync database before reading to ensure data is up-to-date
    sync_bd_database(options.cwd.as_deref());

    let output = match execute_bd("show", std::slice::from_ref(&id), options.cwd.as_deref()) {
        Ok(output) => output,
        Err(e) => {
            // Handle "not found" errors gracefully (future bd versions may use non-zero exit)
            let err_lower = e.to_lowercase();
            if err_lower.contains("no issue found") || err_lower.contains("not found") {
                log_info!("[bd_show] Issue {} not found (error from bd): {}", id, e);
                return Ok(None);
            }
            return Err(e);
        }
    };

    // Handle empty output (current bd behavior for missing issues: exit 0, empty stdout)
    let trimmed = output.trim();
    if trimmed.is_empty() {
        log_info!("[bd_show] Issue {} not found (empty output from bd)", id);
        return Ok(None);
    }

    // bd show can return either a single object or an array
    let result: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|e| {
            log_error!("[bd_show] Failed to parse JSON for {}: {}", id, e);
            format!("Failed to parse issue: {}", e)
        })?;

    let candidate = if result.is_array() {
        result.as_array().and_then(|arr| arr.first()).cloned()
    } else {
        Some(result)
    };
    // Do not swallow deserialization errors: a schema mismatch used to make an
    // existing issue look "not found" (see #7).
    let raw_issue: Option<BdRawIssue> = match candidate {
        None => None,
        Some(v) => match serde_json::from_value::<BdRawIssue>(v) {
            Ok(issue) => Some(issue),
            Err(e) => {
                log_error!("[bd_show] Issue {} returned by bd but failed to deserialize: {}", id, e);
                return Err(format!("Failed to parse issue {}: {}", id, e));
            }
        },
    };

    log_info!("[bd_show] Issue {} found: {}", id, raw_issue.is_some());
    Ok(raw_issue.map(transform_issue))
}

#[tauri::command]
pub(crate) async fn bd_create(payload: CreatePayload) -> Result<Option<Issue>, String> {
    log_info!("[bd_create] Creating issue: {:?}", payload.title);
    let mut args: Vec<String> = vec![payload.title.clone()];

    if let Some(ref desc) = payload.description {
        args.push("--description".to_string());
        args.push(desc.clone());
    }
    if let Some(ref t) = payload.issue_type {
        args.push("--type".to_string());
        args.push(t.clone());
    }
    if let Some(ref p) = payload.priority {
        args.push("--priority".to_string());
        args.push(priority_to_number(p));
    }
    if let Some(ref a) = payload.assignee {
        args.push("--assignee".to_string());
        args.push(a.clone());
    }
    if let Some(ref labels) = payload.labels {
        if !labels.is_empty() {
            args.push("--labels".to_string());
            args.push(labels.join(","));
        }
    }
    if let Some(ref ext) = payload.external_ref {
        args.push("--external-ref".to_string());
        args.push(ext.clone());
    }
    if let Some(est) = payload.estimate_minutes {
        args.push("--estimate".to_string());
        args.push(est.to_string());
    }
    if let Some(ref design) = payload.design_notes {
        args.push("--design".to_string());
        args.push(design.clone());
    }
    if let Some(ref acc) = payload.acceptance_criteria {
        args.push("--acceptance".to_string());
        args.push(acc.clone());
    }
    if let Some(ref notes) = payload.working_notes {
        args.push("--notes".to_string());
        args.push(notes.clone());
    }
    if let Some(ref parent) = payload.parent {
        if !parent.is_empty() {
            args.push("--parent".to_string());
            args.push(parent.clone());
        }
    }
    if let Some(ref spec_id) = payload.spec_id {
        if !spec_id.is_empty() {
            args.push("--spec-id".to_string());
            args.push(spec_id.clone());
        }
    }

    let output = execute_bd("create", &args, payload.cwd.as_deref())?;

    let raw_issue: BdRawIssue = serde_json::from_str(&output)
        .map_err(|e| format!("Failed to parse created issue: {}", e))?;

    Ok(Some(transform_issue(raw_issue)))
}

#[tauri::command]
pub(crate) async fn bd_update(id: String, updates: UpdatePayload) -> Result<Option<Issue>, String> {
    // Always log update calls for debugging (regardless of LOGGING_ENABLED)
    log::info!("[bd_update] Updating issue: {} with cwd: {:?}", id, updates.cwd);
    log::info!("[bd_update] Updates: status={:?}, title={:?}, type={:?}", updates.status, updates.title, updates.issue_type);

    let mut args: Vec<String> = vec![id.clone()];

    if let Some(ref title) = updates.title {
        args.push("--title".to_string());
        args.push(title.clone());
    }
    if let Some(ref desc) = updates.description {
        args.push("--description".to_string());
        args.push(desc.clone());
    }
    if let Some(ref t) = updates.issue_type {
        args.push("--type".to_string());
        args.push(t.clone());
    }
    if let Some(ref s) = updates.status {
        args.push("--status".to_string());
        args.push(s.clone());
    }
    if let Some(ref p) = updates.priority {
        args.push("--priority".to_string());
        args.push(priority_to_number(p));
    }
    if let Some(ref a) = updates.assignee {
        args.push("--assignee".to_string());
        args.push(a.clone());
    }
    if let Some(ref labels) = updates.labels {
        args.push("--set-labels".to_string());
        args.push(labels.join(","));
    }
    if let Some(ref ext) = updates.external_ref {
        args.push("--external-ref".to_string());
        args.push(ext.clone());
    }
    if let Some(est) = updates.estimate_minutes {
        args.push("--estimate".to_string());
        args.push(est.to_string());
    }
    if let Some(ref design) = updates.design_notes {
        args.push("--design".to_string());
        args.push(design.clone());
    }
    if let Some(ref acc) = updates.acceptance_criteria {
        args.push("--acceptance".to_string());
        args.push(acc.clone());
    }
    if let Some(ref notes) = updates.working_notes {
        args.push("--notes".to_string());
        args.push(notes.clone());
    }
    if let Some(ref metadata) = updates.metadata {
        args.push("--metadata".to_string());
        args.push(metadata.clone());
    }
    if let Some(ref spec_id) = updates.spec_id {
        args.push("--spec-id".to_string());
        args.push(spec_id.clone());
    }
    if let Some(ref parent) = updates.parent {
        args.push("--parent".to_string());
        args.push(parent.clone());
    }

    log::info!("[bd_update] Executing: bd update {}", args.join(" "));
    let output = execute_bd("update", &args, updates.cwd.as_deref())?;

    log::info!("[bd_update] Raw output: {}", output.chars().take(500).collect::<String>());

    // Handle empty output from bd CLI (some updates return empty response)
    let trimmed_output = output.trim();
    if trimmed_output.is_empty() {
        log::info!("[bd_update] Empty response from bd, fetching issue {} to get updated data", id);
        // Fetch the updated issue directly
        let show_output = execute_bd("show", std::slice::from_ref(&id), updates.cwd.as_deref())?;
        let show_result: serde_json::Value = serde_json::from_str(&show_output)
            .map_err(|e| {
                log::error!("[bd_update] Failed to parse show JSON: {}", e);
                format!("Failed to fetch updated issue: {}", e)
            })?;

        let raw_issue: Option<BdRawIssue> = if show_result.is_array() {
            show_result.as_array()
                .and_then(|arr| arr.first())
                .and_then(|v| serde_json::from_value(v.clone()).ok())
        } else {
            serde_json::from_value(show_result).ok()
        };

        return Ok(raw_issue.map(transform_issue));
    }

    // bd update can return either a single object or an array
    let result: serde_json::Value = serde_json::from_str(trimmed_output)
        .map_err(|e| {
            log::error!("[bd_update] Failed to parse JSON: {}", e);
            format!("Failed to parse updated issue: {}", e)
        })?;

    let raw_issue: Option<BdRawIssue> = if result.is_array() {
        log::info!("[bd_update] Result is array");
        result.as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    } else {
        log::info!("[bd_update] Result is object");
        serde_json::from_value(result.clone()).map_err(|e| {
            log::error!("[bd_update] Failed to parse issue from result: {}", e);
            e
        }).ok()
    };

    if let Some(ref issue) = raw_issue {
        log::info!("[bd_update] Updated issue {} - new status: {}", id, issue.status);
    } else {
        log::warn!("[bd_update] Could not parse updated issue from response");
    }

    Ok(raw_issue.map(transform_issue))
}

#[tauri::command]
pub(crate) async fn bd_close(id: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    log_info!("[bd_close] Closing issue: {} with cwd: {:?}", id, options.cwd);

    let mut args = vec![id.clone()];
    // br supports --suggest-next for showing newly unblocked issues
    if matches!(get_cli_client_info(), Some((CliClient::Br, _, _, _))) {
        args.push("--suggest-next".to_string());
    }

    let output = execute_bd("close", &args, options.cwd.as_deref())?;

    log_info!("[bd_close] Raw output: {}", output.chars().take(500).collect::<String>());

    let result: serde_json::Value = serde_json::from_str(&output)
        .map_err(|e| {
            log_error!("[bd_close] Failed to parse JSON: {}", e);
            format!("Failed to parse close result: {}", e)
        })?;

    log_info!("[bd_close] Issue {} closed successfully", id);
    Ok(result)
}

#[tauri::command]
pub(crate) async fn bd_search(query: String, options: CwdOptions) -> Result<Vec<Issue>, String> {
    log_info!("[bd_search] Searching for: {} with cwd: {:?}", query, options.cwd);

    let args = vec![query];
    let output = execute_bd("search", &args, options.cwd.as_deref())?;

    log_info!("[bd_search] Raw output: {}", output.chars().take(500).collect::<String>());

    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Ok(vec![]);
    }

    let raw: Vec<BdRawIssue> = serde_json::from_str(trimmed)
        .map_err(|e| {
            log_error!("[bd_search] Failed to parse JSON: {}", e);
            format!("Failed to parse search results: {}", e)
        })?;

    Ok(raw.into_iter().map(transform_issue).collect())
}

#[tauri::command]
pub(crate) async fn bd_label_add(id: String, label: String, options: CwdOptions) -> Result<(), String> {
    log_info!("[bd_label_add] Adding label '{}' to issue {}", label, id);
    let args = vec![id, label];
    execute_bd("label add", &args, options.cwd.as_deref())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn bd_label_remove(id: String, label: String, options: CwdOptions) -> Result<(), String> {
    log_info!("[bd_label_remove] Removing label '{}' from issue {}", label, id);
    let args = vec![id, label];
    execute_bd("label remove", &args, options.cwd.as_deref())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn bd_delete(id: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    let mut args = vec![id.clone(), "--force".to_string()];
    if supports_delete_hard_flag() {
        args.push("--hard".to_string());
    }
    log::info!("[bd_delete] Deleting issue: {} with args: {:?}", id, args);
    execute_bd("delete", &args, options.cwd.as_deref())?;

    // Sync after delete to push deletion to remote and prevent resurrection
    sync_bd_database(options.cwd.as_deref());

    // Clean up attachments folder for this issue
    let project_path = options.cwd.as_deref().unwrap_or(".");
    let abs_project_path = if project_path == "." || project_path.is_empty() {
        env::current_dir().ok()
    } else {
        let p = PathBuf::from(project_path);
        if p.is_relative() {
            env::current_dir().ok().map(|cwd| cwd.join(&p))
        } else {
            Some(p)
        }
    };

    if let Some(path) = abs_project_path {
        if let Ok(abs_path) = path.canonicalize() {
            let att_dir = abs_path.join(".beads").join("attachments").join(issue_short_id(&id));
            if att_dir.exists() && att_dir.is_dir() {
                if let Err(e) = fs::remove_dir_all(&att_dir) {
                    log::warn!("[bd_delete] Failed to remove attachments folder: {}", e);
                } else {
                    log::info!("[bd_delete] Removed attachments folder: {:?}", att_dir);
                }
            }
        }
    }

    Ok(serde_json::json!({ "success": true, "id": id }))
}

#[tauri::command]
pub(crate) async fn bd_comments_add(id: String, content: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    let args = vec![id, content];

    execute_bd("comments add", &args, options.cwd.as_deref())?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub(crate) async fn bd_dep_add(issue_id: String, blocker_id: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    let args = vec![issue_id, blocker_id];

    execute_bd("dep add", &args, options.cwd.as_deref())?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub(crate) async fn bd_dep_remove(issue_id: String, blocker_id: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    let args = vec![issue_id, blocker_id];

    execute_bd("dep remove", &args, options.cwd.as_deref())?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub(crate) async fn bd_dep_add_relation(id1: String, id2: String, relation_type: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    let args = vec![id1, id2, "--type".to_string(), relation_type];

    execute_bd("dep add", &args, options.cwd.as_deref())?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub(crate) async fn bd_dep_remove_relation(id1: String, id2: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    let args = vec![id1, id2];

    execute_bd("dep remove", &args, options.cwd.as_deref())?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub(crate) async fn bd_available_relation_types() -> Vec<serde_json::Value> {
    let common: Vec<(&str, &str)> = vec![
        ("relates-to", "Relates To"),
        ("related", "Related"),
        ("discovered-from", "Discovered From"),
        ("duplicates", "Duplicates"),
        ("supersedes", "Supersedes"),
        ("caused-by", "Caused By"),
        ("replies-to", "Replies To"),
    ];
    let bd_only: Vec<(&str, &str)> = vec![
        ("tracks", "Tracks"),
        ("until", "Until"),
        ("validates", "Validates"),
    ];

    let types = match get_cli_client_info() {
        Some((CliClient::Br, _, _, _)) => common,
        _ => {
            let mut all = common;
            all.extend(bd_only);
            all
        }
    };

    types.into_iter().map(|(v, l)| serde_json::json!({ "value": v, "label": l })).collect()
}

