//! Issue operations: one function per CLI operation, generic over `&dyn CliInvoker`.
//!
//! The bodies are the app's former Tauri command bodies (`issue_commands.rs`, the
//! `sync` invocation of `migration.rs`). Every former read of process-global client
//! state is an invoker accessor or an argument here: `list` reads
//! [`CliInvoker::capabilities`], `close` takes `suggest_next`, `delete` takes `hard`,
//! `sync` takes `no_daemon`, and `--no-daemon` on JSON calls is added by the
//! invoker's [`run_json`](CliInvoker::run_json). The operations return raw CLI types
//! ([`BdRawIssue`]); normalization for the frontend stays with the caller.

use btit_beads::error::{BeadsError, ParseTarget};
use btit_beads::issues::priority_to_number;
use btit_beads::parse::parse_issues_tolerant;
use btit_beads::{log_error, log_info};
use btit_types::{
    BdRawIssue, CliClient, CreatePayload, ListQuery, ProjectRef, RelationType, UpdatePayload,
};

use crate::runner::CliInvoker;

/// `list` with the query's filters and `--limit=0`.
///
/// With `include_all` on a CLI whose `supports_list_all_flag` is `false`, runs two
/// calls instead of `--all` (`list --limit=0`, then `list --limit=0 --status=closed`)
/// and returns the open issues followed by the closed ones; the filters are not
/// applied on that path.
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_json`], and
/// [`parse_issues_tolerant`]'s errors.
#[expect(
    clippy::uninlined_format_args,
    reason = "body moved verbatim from btit-app issue_commands.rs (bd_list) in b-4"
)]
pub fn list(
    inv: &dyn CliInvoker,
    project: &ProjectRef,
    query: &ListQuery,
) -> Result<Vec<BdRawIssue>, BeadsError> {
    let mut args: Vec<String> = Vec::new();

    // --all flag only works correctly on bd >= 0.55; for older versions, fallback to 2 calls
    let use_all = query.include_all.unwrap_or(false);
    if use_all && !inv.capabilities().supports_list_all_flag {
        // Fallback: fetch open + closed separately and merge
        log_info!("[bd_list] --all requested but bd < 0.55 — falling back to 2 calls");
        let mut fallback_args = args.clone();
        fallback_args.push("--limit=0".to_string());

        let open_output = inv.run_json(project, "list", &fallback_args)?;
        let open_issues = parse_issues_tolerant(&open_output, "list_open")?;

        fallback_args.push("--status=closed".to_string());
        let closed_output = inv.run_json(project, "list", &fallback_args)?;
        let closed_issues = parse_issues_tolerant(&closed_output, "list_closed")?;

        let mut all_issues = open_issues;
        all_issues.extend(closed_issues);
        log_info!("[bd_list] Found {} issues (fallback)", all_issues.len());
        return Ok(all_issues);
    }

    if use_all {
        args.push("--all".to_string());
    }
    if let Some(ref statuses) = query.status {
        if !statuses.is_empty() {
            args.push(format!("--status={}", statuses.join(",")));
        }
    }
    if let Some(ref types) = query.issue_type {
        if !types.is_empty() {
            args.push(format!("--type={}", types.join(",")));
        }
    }
    if let Some(ref priorities) = query.priority {
        if !priorities.is_empty() {
            let nums: Vec<String> = priorities.iter().map(|p| priority_to_number(p)).collect();
            args.push(format!("--priority={}", nums.join(",")));
        }
    }
    if let Some(ref assignee) = query.assignee {
        args.push(format!("--assignee={}", assignee));
    }

    // Always disable limit to get all issues (bd defaults to 50)
    args.push("--limit=0".to_string());

    let output = inv.run_json(project, "list", &args)?;

    let raw_issues = parse_issues_tolerant(&output, "list")?;

    log_info!("[bd_list] Found {} issues", raw_issues.len());
    Ok(raw_issues)
}

/// `ready`: the issues with no open blockers.
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_json`], and
/// [`parse_issues_tolerant`]'s errors.
pub fn ready(inv: &dyn CliInvoker, project: &ProjectRef) -> Result<Vec<BdRawIssue>, BeadsError> {
    let output = inv.run_json(project, "ready", &[])?;

    parse_issues_tolerant(&output, "ready")
}

/// `status`: the CLI's status JSON, unparsed beyond `serde_json::Value`.
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_json`], and
/// [`BeadsError::ParseFailed`] (`ParseTarget::Status`) when the output is not JSON.
pub fn status(inv: &dyn CliInvoker, project: &ProjectRef) -> Result<serde_json::Value, BeadsError> {
    let output = inv.run_json(project, "status", &[])?;

    serde_json::from_str(&output).map_err(|e| BeadsError::ParseFailed {
        target: ParseTarget::Status,
        id: None,
        source: e,
    })
}

/// `show <id>`: the issue, or `None` when the CLI reports it missing.
///
/// "Missing" is an invocation error whose text contains `no issue found` or
/// `not found` (case-insensitive), an empty stdout, or an empty JSON array. The CLI
/// may return a single object or an array (the first element is used).
///
/// # Errors
///
/// Other invocation errors from [`CliInvoker::run_json`];
/// [`BeadsError::ParseFailed`] (`ParseTarget::Issue`, `id: None`) when the output is
/// not JSON, and (`id: Some(id)`) when the issue JSON does not deserialize.
pub fn show(
    inv: &dyn CliInvoker,
    project: &ProjectRef,
    id: &str,
) -> Result<Option<BdRawIssue>, BeadsError> {
    let output = match inv.run_json(project, "show", &[id.to_string()]) {
        Ok(output) => output,
        Err(e) => {
            // Handle "not found" errors gracefully (future bd versions may use non-zero exit)
            let err_lower = e.to_string().to_lowercase();
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
    let result: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
        log_error!("[bd_show] Failed to parse JSON for {}: {}", id, e);
        BeadsError::ParseFailed {
            target: ParseTarget::Issue,
            id: None,
            source: e,
        }
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
                log_error!(
                    "[bd_show] Issue {} returned by bd but failed to deserialize: {}",
                    id,
                    e
                );
                return Err(BeadsError::ParseFailed {
                    target: ParseTarget::Issue,
                    id: Some(id.to_string()),
                    source: e,
                });
            }
        },
    };

    log_info!("[bd_show] Issue {} found: {}", id, raw_issue.is_some());
    Ok(raw_issue)
}

/// `create <title>` with every set payload field as a flag; the created issue.
///
/// `payload.cwd` is ignored: the project is `project`.
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_json`], and
/// [`BeadsError::ParseFailed`] (`ParseTarget::CreatedIssue`) when the output does not
/// deserialize.
pub fn create(
    inv: &dyn CliInvoker,
    project: &ProjectRef,
    payload: &CreatePayload,
) -> Result<BdRawIssue, BeadsError> {
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

    let output = inv.run_json(project, "create", &args)?;

    let raw_issue: BdRawIssue =
        serde_json::from_str(&output).map_err(|e| BeadsError::ParseFailed {
            target: ParseTarget::CreatedIssue,
            id: None,
            source: e,
        })?;

    Ok(raw_issue)
}

/// `update <id>` with every set field as a flag; the updated issue when it parses.
///
/// When the CLI prints nothing, the issue is fetched with `show <id>`. A result (or
/// fetched issue) that is JSON but does not deserialize as an issue yields `Ok(None)`.
/// `updates.cwd` is ignored: the project is `project`. Logs through `log` directly
/// (ungated), as the app did.
///
/// # Errors
///
/// Invocation errors from either [`CliInvoker::run_json`] call;
/// [`BeadsError::ParseFailed`] with `ParseTarget::UpdatedIssue` when the update output
/// is not JSON, or `ParseTarget::UpdatedIssueFetch` when the fallback `show` output is not.
#[expect(
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    reason = "body moved verbatim from btit-app issue_commands.rs (bd_update) in b-4"
)]
pub fn update(
    inv: &dyn CliInvoker,
    project: &ProjectRef,
    id: &str,
    updates: &UpdatePayload,
) -> Result<Option<BdRawIssue>, BeadsError> {
    let mut args: Vec<String> = vec![id.to_string()];

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
    let output = inv.run_json(project, "update", &args)?;

    log::info!(
        "[bd_update] Raw output: {}",
        output.chars().take(500).collect::<String>()
    );

    // Handle empty output from bd CLI (some updates return empty response)
    let trimmed_output = output.trim();
    if trimmed_output.is_empty() {
        log::info!(
            "[bd_update] Empty response from bd, fetching issue {} to get updated data",
            id
        );
        // Fetch the updated issue directly
        let show_output = inv.run_json(project, "show", &[id.to_string()])?;
        let show_result: serde_json::Value = serde_json::from_str(&show_output).map_err(|e| {
            log::error!("[bd_update] Failed to parse show JSON: {}", e);
            BeadsError::ParseFailed {
                target: ParseTarget::UpdatedIssueFetch,
                id: None,
                source: e,
            }
        })?;

        let raw_issue: Option<BdRawIssue> = if show_result.is_array() {
            show_result
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|v| serde_json::from_value(v.clone()).ok())
        } else {
            serde_json::from_value(show_result).ok()
        };

        return Ok(raw_issue);
    }

    // bd update can return either a single object or an array
    let result: serde_json::Value = serde_json::from_str(trimmed_output).map_err(|e| {
        log::error!("[bd_update] Failed to parse JSON: {}", e);
        BeadsError::ParseFailed {
            target: ParseTarget::UpdatedIssue,
            id: None,
            source: e,
        }
    })?;

    let raw_issue: Option<BdRawIssue> = if result.is_array() {
        log::info!("[bd_update] Result is array");
        result
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    } else {
        log::info!("[bd_update] Result is object");
        serde_json::from_value(result.clone())
            .map_err(|e| {
                log::error!("[bd_update] Failed to parse issue from result: {}", e);
                e
            })
            .ok()
    };

    if let Some(ref issue) = raw_issue {
        log::info!(
            "[bd_update] Updated issue {} - new status: {}",
            id,
            issue.status
        );
    } else {
        log::warn!("[bd_update] Could not parse updated issue from response");
    }

    Ok(raw_issue)
}

/// `close <id>`, with `--suggest-next` when `suggest_next` (a br flag); the CLI's JSON.
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_json`], and
/// [`BeadsError::ParseFailed`] (`ParseTarget::CloseResult`) when the output is not JSON.
pub fn close(
    inv: &dyn CliInvoker,
    project: &ProjectRef,
    id: &str,
    suggest_next: bool,
) -> Result<serde_json::Value, BeadsError> {
    let mut args = vec![id.to_string()];
    // br supports --suggest-next for showing newly unblocked issues
    if suggest_next {
        args.push("--suggest-next".to_string());
    }

    let output = inv.run_json(project, "close", &args)?;

    log_info!(
        "[bd_close] Raw output: {}",
        output.chars().take(500).collect::<String>()
    );

    let result: serde_json::Value = serde_json::from_str(&output).map_err(|e| {
        log_error!("[bd_close] Failed to parse JSON: {}", e);
        BeadsError::ParseFailed {
            target: ParseTarget::CloseResult,
            id: None,
            source: e,
        }
    })?;

    log_info!("[bd_close] Issue {} closed successfully", id);
    Ok(result)
}

/// `search <query>`; an empty output or `[]` is an empty result.
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_json`], and
/// [`BeadsError::ParseFailed`] (`ParseTarget::SearchResults`) when the output is not an
/// issue array.
pub fn search(
    inv: &dyn CliInvoker,
    project: &ProjectRef,
    query: &str,
) -> Result<Vec<BdRawIssue>, BeadsError> {
    let args = vec![query.to_string()];
    let output = inv.run_json(project, "search", &args)?;

    log_info!(
        "[bd_search] Raw output: {}",
        output.chars().take(500).collect::<String>()
    );

    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Ok(vec![]);
    }

    let raw: Vec<BdRawIssue> = serde_json::from_str(trimmed).map_err(|e| {
        log_error!("[bd_search] Failed to parse JSON: {}", e);
        BeadsError::ParseFailed {
            target: ParseTarget::SearchResults,
            id: None,
            source: e,
        }
    })?;

    Ok(raw)
}

/// `label add <id> <label>`.
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_json`].
pub fn label_add(
    inv: &dyn CliInvoker,
    project: &ProjectRef,
    id: &str,
    label: &str,
) -> Result<(), BeadsError> {
    let args = vec![id.to_string(), label.to_string()];
    inv.run_json(project, "label add", &args)?;
    Ok(())
}

/// `label remove <id> <label>`.
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_json`].
pub fn label_remove(
    inv: &dyn CliInvoker,
    project: &ProjectRef,
    id: &str,
    label: &str,
) -> Result<(), BeadsError> {
    let args = vec![id.to_string(), label.to_string()];
    inv.run_json(project, "label remove", &args)?;
    Ok(())
}

/// `delete <id> --force`, plus `--hard` when `hard` (bd < 0.50).
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_json`].
#[expect(
    clippy::uninlined_format_args,
    reason = "body moved verbatim from btit-app issue_commands.rs (bd_delete) in b-4"
)]
pub fn delete(
    inv: &dyn CliInvoker,
    project: &ProjectRef,
    id: &str,
    hard: bool,
) -> Result<(), BeadsError> {
    let mut args = vec![id.to_string(), "--force".to_string()];
    if hard {
        args.push("--hard".to_string());
    }
    log::info!("[bd_delete] Deleting issue: {} with args: {:?}", id, args);
    inv.run_json(project, "delete", &args)?;
    Ok(())
}

/// `comments add <id> <content>`.
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_json`].
pub fn comment_add(
    inv: &dyn CliInvoker,
    project: &ProjectRef,
    id: &str,
    content: &str,
) -> Result<(), BeadsError> {
    let args = vec![id.to_string(), content.to_string()];

    inv.run_json(project, "comments add", &args)?;

    Ok(())
}

/// `dep add <issue_id> <depends_on_id>`, plus `--type <relation_type>` when given.
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_json`].
pub fn dep_add(
    inv: &dyn CliInvoker,
    project: &ProjectRef,
    issue_id: &str,
    depends_on_id: &str,
    relation_type: Option<&str>,
) -> Result<(), BeadsError> {
    let mut args = vec![issue_id.to_string(), depends_on_id.to_string()];
    if let Some(relation_type) = relation_type {
        args.push("--type".to_string());
        args.push(relation_type.to_string());
    }

    inv.run_json(project, "dep add", &args)?;

    Ok(())
}

/// `dep remove <issue_id> <depends_on_id>`.
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_json`].
pub fn dep_remove(
    inv: &dyn CliInvoker,
    project: &ProjectRef,
    issue_id: &str,
    depends_on_id: &str,
) -> Result<(), BeadsError> {
    let args = vec![issue_id.to_string(), depends_on_id.to_string()];

    inv.run_json(project, "dep remove", &args)?;

    Ok(())
}

/// The dependency relation types `client` offers: the seven common ones, plus
/// `tracks`, `until` and `validates` for every client except br.
#[must_use]
pub fn relation_types(client: CliClient) -> Vec<RelationType> {
    let common: Vec<(&'static str, &'static str)> = vec![
        ("relates-to", "Relates To"),
        ("related", "Related"),
        ("discovered-from", "Discovered From"),
        ("duplicates", "Duplicates"),
        ("supersedes", "Supersedes"),
        ("caused-by", "Caused By"),
        ("replies-to", "Replies To"),
    ];
    let bd_only: Vec<(&'static str, &'static str)> = vec![
        ("tracks", "Tracks"),
        ("until", "Until"),
        ("validates", "Validates"),
    ];

    let types = if client == CliClient::Br {
        common
    } else {
        let mut all = common;
        all.extend(bd_only);
        all
    };

    types
        .into_iter()
        .map(|(value, label)| RelationType { value, label })
        .collect()
}

/// `sync`, plus `--no-daemon` when `no_daemon`: a raw invocation (no `--json`, no lock).
///
/// # Errors
///
/// Invocation errors from [`CliInvoker::run_raw`] ([`BeadsError::Spawn`] with
/// `operation: Some("sync")`), and [`BeadsError::CommandFailed`] carrying the captured
/// stderr when `sync` exits unsuccessfully.
pub fn sync(inv: &dyn CliInvoker, project: &ProjectRef, no_daemon: bool) -> Result<(), BeadsError> {
    let mut sync_args = vec!["sync"];
    if no_daemon {
        sync_args.push("--no-daemon");
    }
    let output = inv.run_raw(project, &sync_args)?;
    if output.success {
        Ok(())
    } else {
        Err(BeadsError::CommandFailed {
            binary: inv.binary(),
            status: output.status,
            status_display: exit_status_display(output.status),
            stderr: output.stderr,
        })
    }
}

/// `std::process::ExitStatus`'s `Display` text for an exit code, which `CliOutput` does
/// not carry (`None` is a signal termination on Unix).
fn exit_status_display(status: Option<i32>) -> String {
    match status {
        #[cfg(target_os = "windows")]
        Some(code) => format!("exit code: {code}"),
        #[cfg(not(target_os = "windows"))]
        Some(code) => format!("exit status: {code}"),
        None => "terminated by signal".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_status_display_matches_std_for_exit_codes() {
        #[cfg(not(target_os = "windows"))]
        {
            use std::os::unix::process::ExitStatusExt;
            let status = std::process::ExitStatus::from_raw(3 << 8);
            assert_eq!(exit_status_display(status.code()), status.to_string());
        }
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::ExitStatusExt;
            let status = std::process::ExitStatus::from_raw(3);
            assert_eq!(exit_status_display(status.code()), status.to_string());
        }
        assert_eq!(exit_status_display(None), "terminated by signal");
    }
}
