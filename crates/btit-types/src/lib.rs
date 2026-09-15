//! Shared data types for the Beads Task-Issue Tracker.
//!
//! This crate holds every data type shared between the app, the backend contract
//! and the backends: CLI probe/capability value types, raw and normalized issue
//! types, filesystem browsing types, and command payload DTOs. Data only: no I/O,
//! no process spawning, no Tauri, no logging.

#![deny(missing_docs)]

mod backend;
mod cli;
mod fs;
mod issue;
mod payload;

#[doc(inline)]
pub use backend::{DoltOpResult, ProjectRef, RelationType};
#[doc(inline)]
pub use cli::{
    BackendCapabilities, CliClient, CliOutput, CliProbe, CliVersion, CompatibilityInfo,
    ReleaseSource,
};
#[doc(inline)]
pub use fs::{DirectoryEntry, FsListResult, PurgeResult};
#[doc(inline)]
pub use issue::{
    BdRawComment, BdRawDependency, BdRawDependent, BdRawIssue, ChildIssue, Comment, CountResult,
    Issue, ParentIssue, Relation,
};
#[doc(inline)]
pub use payload::{CreatePayload, CwdOptions, ListOptions, ListQuery, UpdatePayload};
