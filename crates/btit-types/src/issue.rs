//! Raw and normalized issue types shared between the CLI backends and the frontend.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dependency relationship as returned by bd CLI.
/// Format: `{"issue_id": "...", "depends_on_id": "...", "type": "blocks", "created_at": "...", "created_by": "..."}`
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BdRawDependency {
    /// Row id, when the CLI includes one.
    pub id: Option<String>,
    /// The issue that has the dependency.
    pub issue_id: Option<String>,
    /// The issue it depends on.
    pub depends_on_id: Option<String>,
    /// Relation kind (e.g. "blocks").
    #[serde(rename = "type", alias = "dependency_type")]
    pub dependency_type: Option<String>,
    /// Creation timestamp.
    pub created_at: Option<String>,
    /// Creator.
    pub created_by: Option<String>,
}

/// Dependent info (for parent-child relationships with full issue info).
/// Some bd versions may return this format instead.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BdRawDependent {
    /// Dependent issue id.
    pub id: Option<String>,
    /// Dependent issue title.
    pub title: Option<String>,
    /// Dependent issue status.
    pub status: Option<String>,
    /// Dependent issue priority (numeric).
    pub priority: Option<i32>,
    /// Dependent issue type.
    pub issue_type: Option<String>,
    /// Relation kind (e.g. "blocks").
    pub dependency_type: Option<String>,
}

/// Raw issue as returned by the `bd`/`br` CLI JSON output.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BdRawIssue {
    /// Issue id.
    pub id: String,
    /// Issue title.
    pub title: String,
    /// Issue description.
    pub description: Option<String>,
    /// Issue status (e.g. "open").
    pub status: String,
    /// Numeric priority.
    pub priority: i32,
    /// Issue type (e.g. "bug", "task").
    pub issue_type: String,
    /// Legacy owner field.
    pub owner: Option<String>,
    /// Assignee.
    pub assignee: Option<String>,
    /// Labels.
    pub labels: Option<Vec<String>>,
    /// Creation timestamp.
    pub created_at: String,
    /// Creator.
    pub created_by: Option<String>,
    /// Last update timestamp.
    pub updated_at: String,
    /// Close timestamp, when closed.
    pub closed_at: Option<String>,
    /// Close reason, when closed.
    pub close_reason: Option<String>,
    /// Ids of blocking issues.
    pub blocked_by: Option<Vec<String>>,
    /// Ids of issues this one blocks.
    pub blocks: Option<Vec<String>>,
    /// Comments attached to the issue.
    pub comments: Option<Vec<BdRawComment>>,
    /// Reserved for real external references (see `docs/attachments.md`).
    pub external_ref: Option<String>,
    /// Estimate, in minutes.
    pub estimate: Option<i32>,
    /// Design notes.
    pub design: Option<String>,
    /// Acceptance criteria.
    pub acceptance_criteria: Option<String>,
    /// Working notes.
    pub notes: Option<String>,
    /// Parent issue id.
    pub parent: Option<String>,
    /// Dependent issues.
    pub dependents: Option<Vec<BdRawDependent>>,
    /// Dependency issues.
    pub dependencies: Option<Vec<BdRawDependency>>,
    /// Dependency count, when the CLI precomputes it.
    pub dependency_count: Option<i32>,
    /// Dependent count, when the CLI precomputes it.
    pub dependent_count: Option<i32>,
    /// bd emits a JSON object (`json.RawMessage`); older data may carry a JSON string.
    pub metadata: Option<serde_json::Value>,
    /// Spec id, when linked.
    pub spec_id: Option<String>,
    /// Comment count, when the CLI precomputes it.
    pub comment_count: Option<i32>,
}

/// Raw comment as returned by the CLI JSON output.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BdRawComment {
    /// Comment id; some CLI versions emit a number, others a string.
    pub id: serde_json::Value,
    /// Owning issue id.
    pub issue_id: Option<String>,
    /// Comment author.
    pub author: String,
    /// Comment text (newer field name).
    pub text: Option<String>,
    /// Comment content (older field name).
    pub content: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
}

/// Issue as normalized for the frontend (camelCase wire format).
#[derive(Debug, Serialize, Deserialize)]
pub struct Issue {
    /// Issue id.
    pub id: String,
    /// Issue title.
    pub title: String,
    /// Issue description.
    pub description: String,
    /// Issue type (e.g. "bug", "task").
    #[serde(rename = "type")]
    pub issue_type: String,
    /// Issue status.
    pub status: String,
    /// Priority as a string (e.g. "p1").
    pub priority: String,
    /// Assignee.
    pub assignee: Option<String>,
    /// Labels.
    pub labels: Vec<String>,
    /// Creation timestamp.
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Last update timestamp.
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    /// Close timestamp, when closed.
    #[serde(rename = "closedAt")]
    pub closed_at: Option<String>,
    /// Normalized comments.
    pub comments: Vec<Comment>,
    /// Ids of blocking issues.
    #[serde(rename = "blockedBy")]
    pub blocked_by: Option<Vec<String>>,
    /// Ids of issues this one blocks.
    pub blocks: Option<Vec<String>>,
    /// Reserved for real external references (see `docs/attachments.md`).
    #[serde(rename = "externalRef")]
    pub external_ref: Option<String>,
    /// Estimate, in minutes.
    #[serde(rename = "estimateMinutes")]
    pub estimate_minutes: Option<i32>,
    /// Design notes.
    #[serde(rename = "designNotes")]
    pub design_notes: Option<String>,
    /// Acceptance criteria.
    #[serde(rename = "acceptanceCriteria")]
    pub acceptance_criteria: Option<String>,
    /// Working notes.
    #[serde(rename = "workingNotes")]
    pub working_notes: Option<String>,
    /// Parent issue, when linked.
    pub parent: Option<ParentIssue>,
    /// Child issues, when linked.
    pub children: Option<Vec<ChildIssue>>,
    /// Dependency/dependent relations.
    pub relations: Option<Vec<Relation>>,
    /// Normalized: a JSON object when present, `None` when absent/null/empty.
    pub metadata: Option<serde_json::Value>,
    /// Spec id, when linked.
    #[serde(rename = "specId")]
    pub spec_id: Option<String>,
    /// Comment count.
    #[serde(rename = "commentCount")]
    pub comment_count: Option<i32>,
    /// Dependency count.
    #[serde(rename = "dependencyCount")]
    pub dependency_count: Option<i32>,
    /// Dependent count.
    #[serde(rename = "dependentCount")]
    pub dependent_count: Option<i32>,
}

/// Normalized comment.
#[derive(Debug, Serialize, Deserialize)]
pub struct Comment {
    /// Comment id.
    pub id: String,
    /// Comment author.
    pub author: String,
    /// Comment content.
    pub content: String,
    /// Creation timestamp.
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// Minimal child issue summary, as embedded in `Issue::children`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChildIssue {
    /// Child issue id.
    pub id: String,
    /// Child issue title.
    pub title: String,
    /// Child issue status.
    pub status: String,
    /// Child issue priority.
    pub priority: String,
}

/// Minimal parent issue summary, as embedded in `Issue::parent`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ParentIssue {
    /// Parent issue id.
    pub id: String,
    /// Parent issue title.
    pub title: String,
    /// Parent issue status.
    pub status: String,
    /// Parent issue priority.
    pub priority: String,
}

/// A dependency or dependent relation, as embedded in `Issue::relations`.
#[derive(Debug, Serialize, Deserialize)]
pub struct Relation {
    /// Related issue id.
    pub id: String,
    /// Related issue title.
    pub title: String,
    /// Related issue status.
    pub status: String,
    /// Related issue priority.
    pub priority: String,
    /// Relation kind (e.g. "blocks").
    #[serde(rename = "relationType")]
    pub relation_type: String,
    /// "dependency" or "dependent".
    pub direction: String,
}

/// Aggregate issue counts, as returned by `bd_count`.
#[derive(Debug, Serialize, Deserialize)]
pub struct CountResult {
    /// Total issue count.
    pub count: usize,
    /// Counts keyed by issue type.
    #[serde(rename = "byType")]
    pub by_type: HashMap<String, usize>,
    /// Counts keyed by priority.
    #[serde(rename = "byPriority")]
    pub by_priority: HashMap<String, usize>,
    /// Timestamp of the most recently updated issue, if any.
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<String>,
}
