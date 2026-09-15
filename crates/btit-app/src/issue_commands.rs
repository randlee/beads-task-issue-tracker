use crate::attachments::issue_short_id;
use crate::cli::{supports_delete_hard_flag, AppInvoker};
use btit_beads::issues::transform_issue;
use btit_cli::{ops, CliInvoker};
use crate::migration::sync_bd_database;
use btit_types::{CliClient, CountResult, CreatePayload, CwdOptions, Issue, ListOptions, ListQuery, ProjectRef, UpdatePayload};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[tauri::command]
pub(crate) async fn bd_list(options: ListOptions) -> Result<Vec<Issue>, String> {
    log_info!("[bd_list] cwd: {:?}", options.cwd);

    // Sync database before reading to ensure data is up-to-date
    sync_bd_database(options.cwd.as_deref());

    let raw_issues = ops::list(&AppInvoker, &ProjectRef::local(options.cwd), &options.query).map_err(|e| e.to_string())?;
    Ok(raw_issues.into_iter().map(transform_issue).collect())
}

#[tauri::command]
pub(crate) async fn bd_count(options: CwdOptions) -> Result<CountResult, String> {
    // Sync database before reading to ensure data is up-to-date
    sync_bd_database(options.cwd.as_deref());

    // Fetch all issues: single --all call for bd >= 0.55, fallback to 2 calls for older versions
    let all = ListQuery { include_all: Some(true), ..ListQuery::default() };
    let raw_issues = ops::list(&AppInvoker, &ProjectRef::local(options.cwd), &all).map_err(|e| e.to_string())?;

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

    let raw_issues = ops::ready(&AppInvoker, &ProjectRef::local(options.cwd)).map_err(|e| e.to_string())?;

    log_info!("[bd_ready] Found {} ready issues", raw_issues.len());
    Ok(raw_issues.into_iter().map(transform_issue).collect())
}

#[tauri::command]
pub(crate) async fn bd_status(options: CwdOptions) -> Result<serde_json::Value, String> {
    ops::status(&AppInvoker, &ProjectRef::local(options.cwd)).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn bd_show(id: String, options: CwdOptions) -> Result<Option<Issue>, String> {
    log_info!("[bd_show] Called for issue: {} with cwd: {:?}", id, options.cwd);

    // Sync database before reading to ensure data is up-to-date
    sync_bd_database(options.cwd.as_deref());

    let raw_issue = ops::show(&AppInvoker, &ProjectRef::local(options.cwd), &id).map_err(|e| e.to_string())?;
    Ok(raw_issue.map(transform_issue))
}

#[tauri::command]
pub(crate) async fn bd_create(payload: CreatePayload) -> Result<Option<Issue>, String> {
    log_info!("[bd_create] Creating issue: {:?}", payload.title);
    let raw_issue = ops::create(&AppInvoker, &ProjectRef::local(payload.cwd.clone()), &payload).map_err(|e| e.to_string())?;

    Ok(Some(transform_issue(raw_issue)))
}

#[tauri::command]
pub(crate) async fn bd_update(id: String, updates: UpdatePayload) -> Result<Option<Issue>, String> {
    // Always log update calls for debugging (regardless of LOGGING_ENABLED)
    log::info!("[bd_update] Updating issue: {} with cwd: {:?}", id, updates.cwd);
    log::info!("[bd_update] Updates: status={:?}, title={:?}, type={:?}", updates.status, updates.title, updates.issue_type);

    let raw_issue = ops::update(&AppInvoker, &ProjectRef::local(updates.cwd.clone()), &id, &updates).map_err(|e| e.to_string())?;

    Ok(raw_issue.map(transform_issue))
}

#[tauri::command]
pub(crate) async fn bd_close(id: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    log_info!("[bd_close] Closing issue: {} with cwd: {:?}", id, options.cwd);

    // br supports --suggest-next for showing newly unblocked issues
    let suggest_next = AppInvoker.client() == CliClient::Br;
    ops::close(&AppInvoker, &ProjectRef::local(options.cwd), &id, suggest_next).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn bd_search(query: String, options: CwdOptions) -> Result<Vec<Issue>, String> {
    log_info!("[bd_search] Searching for: {} with cwd: {:?}", query, options.cwd);

    let raw = ops::search(&AppInvoker, &ProjectRef::local(options.cwd), &query).map_err(|e| e.to_string())?;

    Ok(raw.into_iter().map(transform_issue).collect())
}

#[tauri::command]
pub(crate) async fn bd_label_add(id: String, label: String, options: CwdOptions) -> Result<(), String> {
    log_info!("[bd_label_add] Adding label '{}' to issue {}", label, id);
    ops::label_add(&AppInvoker, &ProjectRef::local(options.cwd), &id, &label).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn bd_label_remove(id: String, label: String, options: CwdOptions) -> Result<(), String> {
    log_info!("[bd_label_remove] Removing label '{}' from issue {}", label, id);
    ops::label_remove(&AppInvoker, &ProjectRef::local(options.cwd), &id, &label).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn bd_delete(id: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    ops::delete(&AppInvoker, &ProjectRef::local(options.cwd.clone()), &id, supports_delete_hard_flag()).map_err(|e| e.to_string())?;

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
    ops::comment_add(&AppInvoker, &ProjectRef::local(options.cwd), &id, &content).map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub(crate) async fn bd_dep_add(issue_id: String, blocker_id: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    ops::dep_add(&AppInvoker, &ProjectRef::local(options.cwd), &issue_id, &blocker_id, None).map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub(crate) async fn bd_dep_remove(issue_id: String, blocker_id: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    ops::dep_remove(&AppInvoker, &ProjectRef::local(options.cwd), &issue_id, &blocker_id).map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub(crate) async fn bd_dep_add_relation(id1: String, id2: String, relation_type: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    ops::dep_add(&AppInvoker, &ProjectRef::local(options.cwd), &id1, &id2, Some(&relation_type)).map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub(crate) async fn bd_dep_remove_relation(id1: String, id2: String, options: CwdOptions) -> Result<serde_json::Value, String> {
    ops::dep_remove(&AppInvoker, &ProjectRef::local(options.cwd), &id1, &id2).map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub(crate) async fn bd_available_relation_types() -> Vec<serde_json::Value> {
    ops::relation_types(AppInvoker.client())
        .into_iter()
        .map(|t| serde_json::json!({ "value": t.value, "label": t.label }))
        .collect()
}
