//! Tolerant parsing of the CLI's issue-list JSON.

use btit_types::BdRawIssue;

use crate::error::{BeadsError, ExpectedShape};

/// Parse issues with tolerance for malformed entries
/// Returns all successfully parsed issues and logs failures
///
/// Accepts a flat JSON array or br's paginated `{"issues": [...]}` envelope; entries
/// that fail to deserialize are skipped and logged under `context`.
///
/// # Errors
///
/// [`BeadsError::InvalidJson`] when `output` is not JSON, and
/// [`BeadsError::UnexpectedShape`] when it is neither an array nor an envelope.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "moved verbatim from btit-app in b-3; behaviour and body edits belong to b-9"
)]
pub fn parse_issues_tolerant(output: &str, context: &str) -> Result<Vec<BdRawIssue>, BeadsError> {
    // First try strict parsing
    if let Ok(issues) = serde_json::from_str::<Vec<BdRawIssue>>(output) {
        return Ok(issues);
    }

    // If strict parsing fails, try tolerant parsing
    log_warn!(
        "[{}] Strict parsing failed, attempting tolerant parsing",
        context
    );

    let value: serde_json::Value = serde_json::from_str(output).map_err(|e| {
        log_error!("[{}] JSON is completely invalid: {}", context, e);
        BeadsError::InvalidJson {
            context: context.to_string(),
            source: e,
        }
    })?;

    // br >= 0.1.30 wraps `list` output in a paginated envelope:
    // {"issues": [...], "total": N, "offset": N, "limit": N, "has_more": bool}
    // Unwrap the envelope if present, otherwise expect a flat array.
    let arr_value;
    let arr = if let Some(obj) = value.as_object() {
        if let Some(issues) = obj.get("issues").and_then(|v| v.as_array()) {
            log_info!(
                "[{}] Unwrapped paginated envelope ({} issues)",
                context,
                issues.len()
            );
            arr_value = issues.clone();
            &arr_value
        } else {
            log_error!(
                "[{}] Expected array or envelope with 'issues' key, got object: {:?}",
                context,
                obj.keys().collect::<Vec<_>>()
            );
            return Err(BeadsError::UnexpectedShape {
                context: context.to_string(),
                expected: ExpectedShape::ArrayOrEnvelope,
            });
        }
    } else {
        value.as_array().ok_or_else(|| {
            log_error!("[{}] Expected array, got: {:?}", context, value);
            BeadsError::UnexpectedShape {
                context: context.to_string(),
                expected: ExpectedShape::Array,
            }
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
                    let required = [
                        "id",
                        "title",
                        "status",
                        "priority",
                        "issue_type",
                        "created_at",
                        "updated_at",
                    ];
                    let missing: Vec<&&str> =
                        required.iter().filter(|k| !keys.contains(*k)).collect();
                    if !missing.is_empty() {
                        log_error!(
                            "[{}] Issue {} missing required fields: {:?}",
                            context,
                            i,
                            missing
                        );
                    }
                }
            }
        }
    }

    if failed_count > 0 {
        log_warn!(
            "[{}] Parsed {} issues, skipped {} malformed entries",
            context,
            issues.len(),
            failed_count
        );
    }

    Ok(issues)
}

#[cfg(test)]
#[expect(
    clippy::uninlined_format_args,
    reason = "tests moved verbatim from btit-app in b-3; bodies change only for CliVersion conversions"
)]
mod tests {
    use super::*;
    use crate::test_support::*;

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
        let json = format!(
            r#"{{"issues":[{},{}],"total":2,"offset":0,"limit":50,"has_more":false}}"#,
            good, bad
        );
        let result = parse_issues_tolerant(&json, "test_tolerant_envelope");
        assert!(result.is_ok());
        let issues = result.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "abc-123");
    }
}
