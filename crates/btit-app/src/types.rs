use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CliClient {
    Bd,  // Original Go-based beads CLI
    Br,  // beads_rust — frozen at classic SQLite+JSONL architecture, no daemon
    Unknown,
}


// ============================================================================
// Types
// ============================================================================

/// Dependency relationship as returned by bd CLI
/// Format: {"issue_id": "...", "depends_on_id": "...", "type": "blocks", "created_at": "...", "created_by": "..."}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BdRawDependency {
    pub id: Option<String>,
    pub issue_id: Option<String>,
    pub depends_on_id: Option<String>,
    #[serde(rename = "type", alias = "dependency_type")]
    pub dependency_type: Option<String>,
    pub created_at: Option<String>,
    pub created_by: Option<String>,
}

/// Dependent info (for parent-child relationships with full issue info)
/// Some bd versions may return this format instead
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BdRawDependent {
    pub id: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i32>,
    pub issue_type: Option<String>,
    pub dependency_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BdRawIssue {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: i32,
    pub issue_type: String,
    pub owner: Option<String>,
    pub assignee: Option<String>,
    pub labels: Option<Vec<String>>,
    pub created_at: String,
    pub created_by: Option<String>,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub close_reason: Option<String>,
    pub blocked_by: Option<Vec<String>>,
    pub blocks: Option<Vec<String>>,
    pub comments: Option<Vec<BdRawComment>>,
    pub external_ref: Option<String>,
    pub estimate: Option<i32>,
    pub design: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub notes: Option<String>,
    pub parent: Option<String>,
    pub dependents: Option<Vec<BdRawDependent>>,
    pub dependencies: Option<Vec<BdRawDependency>>,
    pub dependency_count: Option<i32>,
    pub dependent_count: Option<i32>,
    /// bd emits a JSON object (json.RawMessage); older data may carry a JSON string
    pub metadata: Option<serde_json::Value>,
    pub spec_id: Option<String>,
    pub comment_count: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BdRawComment {
    pub id: serde_json::Value,
    pub issue_id: Option<String>,
    pub author: String,
    pub text: Option<String>,
    pub content: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "type")]
    pub issue_type: String,
    pub status: String,
    pub priority: String,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "closedAt")]
    pub closed_at: Option<String>,
    pub comments: Vec<Comment>,
    #[serde(rename = "blockedBy")]
    pub blocked_by: Option<Vec<String>>,
    pub blocks: Option<Vec<String>>,
    #[serde(rename = "externalRef")]
    pub external_ref: Option<String>,
    #[serde(rename = "estimateMinutes")]
    pub estimate_minutes: Option<i32>,
    #[serde(rename = "designNotes")]
    pub design_notes: Option<String>,
    #[serde(rename = "acceptanceCriteria")]
    pub acceptance_criteria: Option<String>,
    #[serde(rename = "workingNotes")]
    pub working_notes: Option<String>,
    pub parent: Option<ParentIssue>,
    pub children: Option<Vec<ChildIssue>>,
    pub relations: Option<Vec<Relation>>,
    /// Normalized: a JSON object when present, `None` when absent/null/empty
    pub metadata: Option<serde_json::Value>,
    #[serde(rename = "specId")]
    pub spec_id: Option<String>,
    #[serde(rename = "commentCount")]
    pub comment_count: Option<i32>,
    #[serde(rename = "dependencyCount")]
    pub dependency_count: Option<i32>,
    #[serde(rename = "dependentCount")]
    pub dependent_count: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub content: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChildIssue {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParentIssue {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    #[serde(rename = "relationType")]
    pub relation_type: String,
    pub direction: String, // "dependency" or "dependent"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CountResult {
    pub count: usize,
    #[serde(rename = "byType")]
    pub by_type: HashMap<String, usize>,
    #[serde(rename = "byPriority")]
    pub by_priority: HashMap<String, usize>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "isDirectory")]
    pub is_directory: bool,
    #[serde(rename = "hasBeads")]
    pub has_beads: bool,
    #[serde(rename = "usesDolt")]
    pub uses_dolt: bool,
}

#[derive(Debug, Serialize)]
pub struct PurgeResult {
    #[serde(rename = "deletedCount")]
    pub deleted_count: usize,
    #[serde(rename = "deletedFolders")]
    pub deleted_folders: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FsListResult {
    #[serde(rename = "currentPath")]
    pub current_path: String,
    #[serde(rename = "hasBeads")]
    pub has_beads: bool,
    #[serde(rename = "usesDolt")]
    pub uses_dolt: bool,
    pub entries: Vec<DirectoryEntry>,
}

// ============================================================================
// Options structs for commands
// ============================================================================

#[derive(Debug, Deserialize, Default)]
pub struct ListOptions {
    pub status: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub issue_type: Option<Vec<String>>,
    pub priority: Option<Vec<String>>,
    pub assignee: Option<String>,
    #[serde(rename = "includeAll")]
    pub include_all: Option<bool>,
    pub cwd: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CwdOptions {
    pub cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePayload {
    pub title: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub issue_type: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub labels: Option<Vec<String>>,
    #[serde(rename = "externalRef")]
    pub external_ref: Option<String>,
    #[serde(rename = "estimateMinutes")]
    pub estimate_minutes: Option<i32>,
    #[serde(rename = "designNotes")]
    pub design_notes: Option<String>,
    #[serde(rename = "acceptanceCriteria")]
    pub acceptance_criteria: Option<String>,
    #[serde(rename = "workingNotes")]
    pub working_notes: Option<String>,
    pub parent: Option<String>, // Parent epic ID for hierarchical child
    #[serde(rename = "specId")]
    pub spec_id: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePayload {
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub issue_type: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub labels: Option<Vec<String>>,
    #[serde(rename = "externalRef")]
    pub external_ref: Option<String>,
    #[serde(rename = "estimateMinutes")]
    pub estimate_minutes: Option<i32>,
    #[serde(rename = "designNotes")]
    pub design_notes: Option<String>,
    #[serde(rename = "acceptanceCriteria")]
    pub acceptance_criteria: Option<String>,
    #[serde(rename = "workingNotes")]
    pub working_notes: Option<String>,
    pub parent: Option<String>, // Some("") to detach, Some("id") to attach
    pub metadata: Option<String>,
    #[serde(rename = "specId")]
    pub spec_id: Option<String>,
    pub cwd: Option<String>,
}


