use crate::attachments::issue_short_id;
use crate::migration::ensure_refs_migrated_v3;
use std::env;
use std::path::PathBuf;

// ============================================================================
// Attachment Refs Migration v3 — filesystem-only
// ============================================================================

/// Returns `true` if `t` starts with a URL scheme (`^[A-Za-z][A-Za-z0-9+.-]*://`).
fn has_url_scheme(t: &str) -> bool {
    let Some(colon) = t.find(':') else { return false };
    let scheme = &t[..colon];
    let mut chars = scheme.chars();
    let Some(first) = chars.next() else { return false };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '.' || c == '-') {
        return false;
    }
    t[colon..].starts_with("://")
}

/// Returns `true` if `t` is a Windows absolute path (`X:\`, `X:/`, or `\\server\share`).
fn is_windows_absolute(t: &str) -> bool {
    if t.starts_with("\\\\") {
        return true;
    }
    let mut chars = t.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(drive), Some(':'), Some(sep)) => drive.is_ascii_alphabetic() && (sep == '\\' || sep == '/'),
        _ => false,
    }
}

/// Check if a ref is a "real" external reference (Redmine, GitHub, or other URL/ID).
/// Returns false for att: refs, local file paths, cleared: sentinels.
pub(crate) fn is_real_external_ref(r: &str) -> bool {
    let trimmed = r.trim();
    if trimmed.is_empty() { return false; }
    if trimmed.starts_with("cleared:") { return false; }
    if trimmed.starts_with("att:") { return false; }
    // A URL scheme is real, even if it happens to contain "/attachments/" or "/.beads/".
    if has_url_scheme(trimmed) {
        return true;
    }
    // Windows absolute paths (drive-letter or UNC) are local.
    if is_windows_absolute(trimmed) {
        return false;
    }
    // Local file paths (absolute or relative .beads/)
    if trimmed.starts_with('/') { return false; }
    if trimmed.starts_with(".beads/") { return false; }
    // Anything with path separators inside .beads or attachments is local
    if trimmed.contains("/attachments/") || trimmed.contains("/.beads/") { return false; }
    true
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RefsMigrationStatus {
    needs_migration: bool,
    ref_count: u32,
    just_migrated: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MigrateRefsResult {
    success: bool,
    refs_updated: u32,
}

/// Check if a project needs attachment refs migration v3.
/// v3 strips all attachment refs (att:, local paths) from external_ref,
/// keeping only real external refs (Redmine, GitHub, URLs).
/// Returns quickly if the .migrated-attachments marker file exists.
#[tauri::command]
pub(crate) async fn check_refs_migration(cwd: Option<String>) -> Result<RefsMigrationStatus, String> {
    let working_dir = cwd
        .or_else(|| env::var("BEADS_PATH").ok())
        .unwrap_or_else(|| {
            env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });

    let beads_dir = PathBuf::from(&working_dir).join(".beads");
    if !beads_dir.exists() {
        return Ok(RefsMigrationStatus { needs_migration: false, ref_count: 0, just_migrated: false });
    }

    // Already migrated to v3?
    if beads_dir.join(".migrated-attachments").exists() {
        // Check if auto-migration just ran (notify signal)
        let notify_path = beads_dir.join(".migrated-attachments-notify");
        let just_migrated = notify_path.exists();
        if just_migrated {
            let _ = std::fs::remove_file(&notify_path);
        }
        return Ok(RefsMigrationStatus { needs_migration: false, ref_count: 0, just_migrated });
    }

    let jsonl_path = beads_dir.join("issues.jsonl");
    if !jsonl_path.exists() {
        let _ = std::fs::write(beads_dir.join(".migrated-attachments"), "");
        return Ok(RefsMigrationStatus { needs_migration: false, ref_count: 0, just_migrated: false });
    }

    // Scan JSONL for refs that need cleanup (non-real external refs)
    let content = std::fs::read_to_string(&jsonl_path)
        .map_err(|e| format!("Failed to read issues.jsonl: {}", e))?;

    let mut ref_count: u32 = 0;

    for line in content.lines() {
        if line.trim().is_empty() { continue; }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(ext_ref) = v.get("external_ref").and_then(|r| r.as_str()) {
            if ext_ref.is_empty() { continue; }
            let refs: Vec<&str> = ext_ref.split(|c: char| c == '\n' || c == '|').collect();
            for r in &refs {
                let trimmed = r.trim();
                if !trimmed.is_empty() && !is_real_external_ref(trimmed) {
                    ref_count += 1;
                    break; // One bad ref per issue is enough to flag it
                }
            }
        }
    }

    // Also check if attachment folders need renaming (full-id → short-id)
    let mut folder_work_count: u32 = 0;
    let attachments_dir = beads_dir.join("attachments");
    if attachments_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&attachments_dir) {
            for entry in entries.flatten() {
                if !entry.path().is_dir() { continue; }
                let name = entry.file_name().to_string_lossy().to_string();
                if issue_short_id(&name) != name {
                    folder_work_count += 1;
                }
            }
        }
    }

    let total = ref_count + folder_work_count;
    if total == 0 {
        let _ = std::fs::write(beads_dir.join(".migrated-attachments"), "");
        return Ok(RefsMigrationStatus { needs_migration: false, ref_count: 0, just_migrated: false });
    }

    log_info!("[refs_migration_v3] Project needs migration: {} ref(s) to clean, {} folder(s) to update", ref_count, folder_work_count);
    Ok(RefsMigrationStatus { needs_migration: true, ref_count: total, just_migrated: false })
}

/// Perform the attachment refs migration v3 (filesystem-only).
/// Delegates to ensure_refs_migrated_v3 which handles backup, cleanup, dedup, and marker.
/// The br sync is NOT called here — it will happen naturally after via sync_bd_database.
#[tauri::command]
pub(crate) async fn migrate_attachment_refs(cwd: Option<String>) -> Result<MigrateRefsResult, String> {
    let working_dir = cwd
        .or_else(|| env::var("BEADS_PATH").ok())
        .unwrap_or_else(|| {
            env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });

    let beads_dir = PathBuf::from(&working_dir).join(".beads");
    ensure_refs_migrated_v3(&beads_dir, &working_dir);
    Ok(MigrateRefsResult { success: true, refs_updated: 0 })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_real_external_ref_accepts_urls() {
        assert!(is_real_external_ref("https://example.com/issue/123"));
        assert!(is_real_external_ref("http://redmine.local/issues/456"));
    }

    #[test]
    fn is_real_external_ref_rejects_empty() {
        assert!(!is_real_external_ref(""));
        assert!(!is_real_external_ref("   "));
    }

    #[test]
    fn is_real_external_ref_rejects_att_refs() {
        assert!(!is_real_external_ref("att:abc123"));
    }

    #[test]
    fn is_real_external_ref_rejects_cleared_sentinels() {
        assert!(!is_real_external_ref("cleared:previous"));
    }

    #[test]
    fn is_real_external_ref_rejects_local_paths() {
        assert!(!is_real_external_ref("/absolute/path"));
        assert!(!is_real_external_ref(".beads/attachments/abc"));
    }

    #[test]
    fn is_real_external_ref_rejects_beads_refs() {
        assert!(!is_real_external_ref("proj/.beads/something"));
        assert!(!is_real_external_ref("path/attachments/file"));
    }

    #[test]
    fn is_real_external_ref_accepts_urls_containing_attachments_segment() {
        assert!(is_real_external_ref("https://redmine.example/attachments/download/1"));
    }

    #[test]
    fn is_real_external_ref_rejects_windows_absolute_paths() {
        assert!(!is_real_external_ref(r"C:\proj\.beads\attachments\x.png"));
        assert!(!is_real_external_ref(r"\\server\share\.beads\x"));
    }
}
