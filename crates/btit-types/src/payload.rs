//! Command payload types: filters and create/update DTOs.

use serde::Deserialize;

/// Filters for `BeadsBackend::list`. Same wire names as today.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ListQuery {
    /// Status filter.
    pub status: Option<Vec<String>>,
    /// Issue type filter.
    #[serde(rename = "type")]
    pub issue_type: Option<Vec<String>>,
    /// Priority strings as the frontend sends them (`"p0"`..); the backend converts with `priority_to_number`.
    pub priority: Option<Vec<String>>,
    /// Assignee filter.
    pub assignee: Option<String>,
    /// When true, includes both open and closed issues.
    #[serde(rename = "includeAll")]
    pub include_all: Option<bool>,
}

/// The `bd_list` payload: the query plus the project directory. Flattened, so the JSON is unchanged.
#[derive(Debug, Deserialize, Default)]
pub struct ListOptions {
    /// The list filters.
    #[serde(flatten)]
    pub query: ListQuery,
    /// Project working directory.
    pub cwd: Option<String>,
}

/// Project working directory payload shared by most commands.
#[derive(Debug, Deserialize, Default)]
pub struct CwdOptions {
    /// Project working directory.
    pub cwd: Option<String>,
}

/// Payload for creating a new issue.
#[derive(Debug, Deserialize)]
pub struct CreatePayload {
    /// Issue title.
    pub title: String,
    /// Issue description.
    pub description: Option<String>,
    /// Issue type (e.g. "bug", "task").
    #[serde(rename = "type")]
    pub issue_type: Option<String>,
    /// Priority as a string (e.g. "p1").
    pub priority: Option<String>,
    /// Assignee.
    pub assignee: Option<String>,
    /// Labels.
    pub labels: Option<Vec<String>>,
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
    /// Parent epic id for a hierarchical child.
    pub parent: Option<String>,
    /// Spec id, when linked.
    #[serde(rename = "specId")]
    pub spec_id: Option<String>,
    /// Project working directory.
    pub cwd: Option<String>,
}

/// Payload for updating an existing issue.
#[derive(Debug, Deserialize)]
pub struct UpdatePayload {
    /// New title.
    pub title: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// New issue type.
    #[serde(rename = "type")]
    pub issue_type: Option<String>,
    /// New status.
    pub status: Option<String>,
    /// New priority.
    pub priority: Option<String>,
    /// New assignee.
    pub assignee: Option<String>,
    /// New labels.
    pub labels: Option<Vec<String>>,
    /// Reserved for real external references (see `docs/attachments.md`).
    #[serde(rename = "externalRef")]
    pub external_ref: Option<String>,
    /// New estimate, in minutes.
    #[serde(rename = "estimateMinutes")]
    pub estimate_minutes: Option<i32>,
    /// New design notes.
    #[serde(rename = "designNotes")]
    pub design_notes: Option<String>,
    /// New acceptance criteria.
    #[serde(rename = "acceptanceCriteria")]
    pub acceptance_criteria: Option<String>,
    /// New working notes.
    #[serde(rename = "workingNotes")]
    pub working_notes: Option<String>,
    /// `Some("")` to detach, `Some("id")` to attach.
    pub parent: Option<String>,
    /// New metadata (JSON string).
    pub metadata: Option<String>,
    /// New spec id.
    #[serde(rename = "specId")]
    pub spec_id: Option<String>,
    /// Project working directory.
    pub cwd: Option<String>,
}
