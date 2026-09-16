//! Filesystem browsing and purge result types.

use serde::{Deserialize, Serialize};

/// One entry in a directory listing.
#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryEntry {
    /// File or directory name.
    pub name: String,
    /// Absolute path.
    pub path: String,
    /// True when the entry is a directory.
    #[serde(rename = "isDirectory")]
    pub is_directory: bool,
    /// True when the directory contains a `.beads/` project.
    #[serde(rename = "hasBeads")]
    pub has_beads: bool,
    /// True when the `.beads/` project uses the Dolt backend.
    #[serde(rename = "usesDolt")]
    pub uses_dolt: bool,
}

/// Result of a purge operation.
#[derive(Debug, Serialize)]
pub struct PurgeResult {
    /// Number of items deleted.
    #[serde(rename = "deletedCount")]
    pub deleted_count: usize,
    /// Paths of the deleted folders.
    #[serde(rename = "deletedFolders")]
    pub deleted_folders: Vec<String>,
}

/// Result of listing a directory.
#[derive(Debug, Serialize, Deserialize)]
pub struct FsListResult {
    /// The listed directory's absolute path.
    #[serde(rename = "currentPath")]
    pub current_path: String,
    /// True when the directory contains a `.beads/` project.
    #[serde(rename = "hasBeads")]
    pub has_beads: bool,
    /// True when the `.beads/` project uses the Dolt backend.
    #[serde(rename = "usesDolt")]
    pub uses_dolt: bool,
    /// Directory entries.
    pub entries: Vec<DirectoryEntry>,
}
