use crate::backend;
use btit_types::{DirectoryEntry, FsListResult, ProjectRef};
use std::path::PathBuf;

#[tauri::command]
pub(crate) async fn fs_exists(path: String) -> Result<bool, String> {
    Ok(std::path::Path::new(&path).exists())
}

#[tauri::command]
pub(crate) async fn fs_list(path: Option<String>) -> Result<FsListResult, String> {
    use std::fs;

    let be = backend::current();

    let target_path = match path {
        Some(p) if p == "~" => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        Some(p) => PathBuf::from(p),
        None => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
    };

    let target_path = target_path.canonicalize()
        .map_err(|e| format!("Cannot resolve path: {e}"))?;

    let entries = fs::read_dir(&target_path)
        .map_err(|e| format!("Cannot read directory: {e}"))?;

    let mut directories: Vec<DirectoryEntry> = Vec::new();

    for entry in entries {
        let Ok(entry) = entry else { continue };

        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files
        if name.starts_with('.') {
            continue;
        }

        let Ok(metadata) = entry.metadata() else { continue };

        if metadata.is_dir() {
            let full_path = entry.path();
            let beads_path = full_path.join(".beads");
            let has_beads = beads_path.is_dir();
            let uses_dolt = has_beads
                && be.project_uses_dolt(&ProjectRef::local(Some(full_path.to_string_lossy().into_owned())));

            directories.push(DirectoryEntry {
                name,
                path: full_path.to_string_lossy().to_string(),
                is_directory: true,
                has_beads,
                uses_dolt,
            });
        }
    }

    // Sort: beads projects first, then alphabetically
    directories.sort_by(|a, b| {
        match (a.has_beads, b.has_beads) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    let current_beads_path = target_path.join(".beads");
    let current_has_beads = current_beads_path.is_dir();
    let current_uses_dolt = current_has_beads
        && be.project_uses_dolt(&ProjectRef::local(Some(target_path.to_string_lossy().into_owned())));

    Ok(FsListResult {
        current_path: target_path.to_string_lossy().to_string(),
        has_beads: current_has_beads,
        uses_dolt: current_uses_dolt,
        entries: directories,
    })
}

// File watcher commands removed - replaced by frontend polling for lower CPU usage


