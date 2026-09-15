use crate::backend;
use std::process::Command;
use btit_types::{ListQuery, ProjectRef, PurgeResult};
use serde::Serialize;
use std::env;
use std::fs;
use std::path::PathBuf;

#[tauri::command]
pub(crate) async fn open_image_file(path: String) -> Result<(), String> {
    log_info!("[open_image_file] Opening: {}", path);

    // Security: Only allow image file extensions
    let allowed_extensions = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico", "tiff", "tif"];
    let path_lower = path.to_lowercase();
    let is_image = allowed_extensions.iter().any(|ext| path_lower.ends_with(&format!(".{}", ext)));

    if !is_image {
        return Err("Only image files are allowed".to_string());
    }

    // Verify file exists
    if !std::path::Path::new(&path).exists() {
        return Err(format!("File not found: {}", path));
    }

    // Security: Canonicalize to resolve symlinks/.. and verify inside .beads/attachments/
    let canonical = std::path::Path::new(&path).canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;
    let canonical_str = canonical.to_string_lossy();
    if !canonical_str.contains("/.beads/attachments/") {
        log_warn!("[open_image_file] Refusing to open file outside attachments: {} (resolved: {})", path, canonical_str);
        return Err("Can only open files inside .beads/attachments/".to_string());
    }

    // Use platform-specific command to open file with default application
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        // Fully qualified: a `use` for this Windows-only call site is "unused" on
        // other targets and gets stripped by cargo fix, which broke the Windows build.
        btit_cli::command::new_command("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    Ok(())
}

#[derive(Debug, Serialize)]
pub struct ImageData {
    pub base64: String,
    pub mime_type: String,
}

#[tauri::command]
pub(crate) async fn read_image_file(path: String) -> Result<ImageData, String> {
    log_info!("[read_image_file] Reading: {}", path);

    // Security: Only allow image file extensions
    let allowed_extensions: &[(&str, &str)] = &[
        ("png", "image/png"),
        ("jpg", "image/jpeg"),
        ("jpeg", "image/jpeg"),
        ("gif", "image/gif"),
        ("webp", "image/webp"),
        ("bmp", "image/bmp"),
        ("svg", "image/svg+xml"),
        ("ico", "image/x-icon"),
        ("tiff", "image/tiff"),
        ("tif", "image/tiff"),
    ];

    let path_lower = path.to_lowercase();
    let mime_type = allowed_extensions
        .iter()
        .find(|(ext, _)| path_lower.ends_with(&format!(".{}", ext)))
        .map(|(_, mime)| *mime);

    let mime_type = match mime_type {
        Some(m) => m.to_string(),
        None => return Err("Only image files are allowed".to_string()),
    };

    // Verify file exists
    if !std::path::Path::new(&path).exists() {
        return Err(format!("File not found: {}", path));
    }

    // Security: Canonicalize to resolve symlinks/.. and verify inside .beads/attachments/
    let canonical = std::path::Path::new(&path).canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;
    let canonical_str = canonical.to_string_lossy();
    if !canonical_str.contains("/.beads/attachments/") {
        log_warn!("[read_image_file] Refusing to read file outside attachments: {} (resolved: {})", path, canonical_str);
        return Err("Can only read files inside .beads/attachments/".to_string());
    }

    // Read file and encode as base64
    let data = fs::read(&path).map_err(|e| format!("Failed to read file: {}", e))?;
    let base64 = base64_encode(&data);

    Ok(ImageData { base64, mime_type })
}

pub(crate) fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        buf[..chunk.len()].copy_from_slice(chunk);

        let n = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);

        result.push(ALPHABET[(n >> 18) as usize & 0x3F] as char);
        result.push(ALPHABET[(n >> 12) as usize & 0x3F] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[(n >> 6) as usize & 0x3F] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[n as usize & 0x3F] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[tauri::command]
pub(crate) async fn purge_orphan_attachments(project_path: String) -> Result<PurgeResult, String> {
    log::info!("[purge_orphan_attachments] project: {}", project_path);

    // Calculate absolute project path (reusing pattern from bd_delete)
    let abs_project_path = if project_path == "." || project_path.is_empty() {
        env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?
    } else {
        let p = PathBuf::from(&project_path);
        if p.is_relative() {
            let cwd = env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?;
            cwd.join(&p)
        } else {
            p
        }
    };

    let abs_project_path = abs_project_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve project path: {}", e))?;

    // Build attachments directory path
    let attachments_dir = abs_project_path.join(".beads").join("attachments");

    // If attachments directory doesn't exist, nothing to purge
    if !attachments_dir.exists() || !attachments_dir.is_dir() {
        log::info!("[purge_orphan_attachments] No attachments directory found");
        return Ok(PurgeResult {
            deleted_count: 0,
            deleted_folders: vec![],
        });
    }

    // Get list of all existing issue IDs via bd list --all
    let existing_ids: std::collections::HashSet<String> = {
        let project = ProjectRef::local(Some(abs_project_path.to_string_lossy().to_string()));
        let all = ListQuery { include_all: Some(true), ..ListQuery::default() };
        let issues = backend::current().list(&project, &all).map_err(|e| e.to_string())?;
        issues.into_iter().map(|i| i.id).collect()
    };

    log::info!("[purge_orphan_attachments] Found {} existing issues", existing_ids.len());

    // List all subdirectories in attachments folder
    let entries = fs::read_dir(&attachments_dir)
        .map_err(|e| format!("Failed to read attachments directory: {}", e))?;

    let mut deleted_folders: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        // Check if this folder corresponds to an existing issue (folders use short IDs)
        let is_owned = existing_ids.iter().any(|id| issue_short_id(id) == folder_name);
        if !is_owned {
            log::info!("[purge_orphan_attachments] Deleting orphan folder: {}", folder_name);
            if let Err(e) = fs::remove_dir_all(&path) {
                log::warn!("[purge_orphan_attachments] Failed to delete {}: {}", folder_name, e);
            } else {
                deleted_folders.push(folder_name);
            }
        }
    }

    let deleted_count = deleted_folders.len();
    log::info!("[purge_orphan_attachments] Purged {} orphan folders", deleted_count);

    Ok(PurgeResult {
        deleted_count,
        deleted_folders,
    })
}

/// Sanitize a filename for safe storage and br JSONL compatibility.
/// Converts to kebab-case, strips diacritics, removes unsafe chars.
/// Example: "Screenshot 2026-02-24 à 10.30.png" → "screenshot-2026-02-24-a-10-30.png"
pub(crate) fn sanitize_filename(filename: &str) -> String {
    // Split into stem and extension
    let (stem, ext) = match filename.rfind('.') {
        Some(pos) => (&filename[..pos], &filename[pos..]),
        None => (filename, ""),
    };

    // Strip diacritics by replacing common accented chars, then lowercase + kebab-case
    let mut sanitized = String::with_capacity(stem.len());
    for c in stem.chars() {
        let replacement = match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "a",
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => "e",
            'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => "i",
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => "o",
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => "u",
            'ñ' | 'Ñ' => "n",
            'ç' | 'Ç' => "c",
            'ß' => "ss",
            'æ' | 'Æ' => "ae",
            'œ' | 'Œ' => "oe",
            'ý' | 'ÿ' | 'Ý' => "y",
            'A'..='Z' => { sanitized.push((c as u8 + 32) as char); continue; },
            'a'..='z' | '0'..='9' => { sanitized.push(c); continue; },
            '-' => { sanitized.push('-'); continue; },
            ' ' | '_' | '.' => { sanitized.push('-'); continue; },
            _ => "-",
        };
        sanitized.push_str(replacement);
    }

    // Collapse multiple consecutive dashes and trim
    let mut result = String::with_capacity(sanitized.len());
    let mut prev_dash = false;
    for c in sanitized.chars() {
        if c == '-' {
            if !prev_dash {
                result.push('-');
            }
            prev_dash = true;
        } else {
            result.push(c);
            prev_dash = false;
        }
    }
    let result = result.trim_matches('-');

    let ext_lower = ext.to_lowercase();
    if result.is_empty() {
        format!("file{}", ext_lower)
    } else {
        format!("{}{}", result, ext_lower)
    }
}

/// Image file extensions supported for attachment preview
pub(crate) const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico", "tiff", "tif"];

/// Markdown file extensions supported for attachment preview
pub(crate) const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown"];

/// Extract the short ID from a full issue ID by stripping the project prefix.
/// e.g. "beads-manager-2qk" → "2qk", "kybio-1pxe" → "1pxe",
///      "kybio-front-nuxt-4-466d" → "466d", "beads-manager-02e.1" → "02e.1"
/// Falls back to the full ID if no prefix separator is found.
pub(crate) fn issue_short_id(full_id: &str) -> &str {
    // The short ID is after the last '-' that isn't followed by another segment
    // containing only digits (which would be part of the prefix like "nuxt-4").
    // Strategy: find the last '-' where everything after it matches [a-z0-9.]+ (the short ID).
    // But "kybio-front-nuxt-4-466d": after last '-' is "466d" ✓
    // "kybio-front-nuxt-4": after last '-' is "4" which could be a short ID or prefix part.
    // Since we only call this with real issue IDs (not project names), the last segment is always the short ID.
    match full_id.rfind('-') {
        Some(pos) => &full_id[pos + 1..],
        None => full_id,
    }
}

/// Resolve the attachment directory for an issue.
/// Always uses short ID: .beads/attachments/{short_id}/
pub(crate) fn resolve_attachment_dir(attachments_dir: &std::path::Path, issue_id: &str) -> PathBuf {
    attachments_dir.join(issue_short_id(issue_id))
}

/// Classify a filename as "image", "markdown", or "other"
pub(crate) fn classify_attachment(filename: &str) -> &'static str {
    let lower = filename.to_lowercase();
    if IMAGE_EXTENSIONS.iter().any(|ext| lower.ends_with(&format!(".{}", ext))) {
        "image"
    } else if MARKDOWN_EXTENSIONS.iter().any(|ext| lower.ends_with(&format!(".{}", ext))) {
        "markdown"
    } else {
        "other"
    }
}

/// Resolve a duplicate filename: image.png → image-1.png → image-2.png
pub(crate) fn resolve_duplicate_filename(dir: &std::path::Path, name: &str) -> String {
    if !dir.join(name).exists() {
        return name.to_string();
    }
    let (stem, ext) = match name.rfind('.') {
        Some(pos) => (&name[..pos], &name[pos..]),
        None => (name, ""),
    };
    for i in 1..1000 {
        let candidate = format!("{}-{}{}", stem, i, ext);
        if !dir.join(&candidate).exists() {
            return candidate;
        }
    }
    // Fallback with timestamp
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}-{}{}", stem, ts, ext)
}


#[tauri::command]
pub(crate) async fn copy_file_to_attachments(
    project_path: String,
    source_path: String,
    issue_id: String,
) -> Result<String, String> {
    log::info!(
        "[copy_file_to_attachments] project: {}, source: {}, issue: {}",
        project_path,
        source_path,
        issue_id
    );

    // Validate file extension (images + markdown)
    let source_lower = source_path.to_lowercase();
    let is_allowed = IMAGE_EXTENSIONS.iter().chain(MARKDOWN_EXTENSIONS.iter())
        .any(|ext| source_lower.ends_with(&format!(".{}", ext)));

    if !is_allowed {
        return Err("Only image and markdown files are allowed".to_string());
    }

    // Verify source file exists
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err(format!("Source file not found: {}", source_path));
    }

    // Calculate absolute project path
    let abs_project_path = if project_path == "." || project_path.is_empty() {
        env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?
    } else {
        let p = PathBuf::from(&project_path);
        if p.is_relative() {
            let cwd = env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?;
            cwd.join(&p)
        } else {
            p
        }
    };

    let abs_project_path = abs_project_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve project path: {}", e))?;

    // Build destination directory: {project}/.beads/attachments/{short_id}/
    let attachments_dir = abs_project_path.join(".beads").join("attachments");
    let dest_dir = resolve_attachment_dir(&attachments_dir, &issue_id);

    // Create directory if needed
    fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Failed to create attachments directory: {}", e))?;

    // Sanitize the original filename and handle duplicates
    let raw_filename = source
        .file_name()
        .ok_or_else(|| "Invalid source filename".to_string())?
        .to_string_lossy()
        .to_string();
    let sanitized = sanitize_filename(&raw_filename);
    let dest_filename = resolve_duplicate_filename(&dest_dir, &sanitized);
    let dest_path = dest_dir.join(&dest_filename);

    // Copy the file
    fs::copy(&source, &dest_path).map_err(|e| format!("Failed to copy file: {}", e))?;

    log::info!("[copy_file_to_attachments] Copied to: {}", dest_path.display());

    // Return just the filename (frontend doesn't need to store it in external_ref)
    Ok(dest_filename)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentFile {
    pub filename: String,
    pub file_type: String,   // "image" or "markdown"
    pub path: String,        // absolute path
    pub modified: u64,       // mtime in epoch seconds (for sorting)
}

/// List all attachments for an issue by reading the filesystem directly.
/// Returns images and markdown files sorted by modification time (newest first).
#[tauri::command]
pub(crate) async fn list_attachments(project_path: String, issue_id: String) -> Result<Vec<AttachmentFile>, String> {
    let abs_project_path = if project_path == "." || project_path.is_empty() {
        env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?
    } else {
        let p = PathBuf::from(&project_path);
        if p.is_relative() {
            let cwd = env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?;
            cwd.join(&p)
        } else {
            p
        }
    };

    let attachments_dir = abs_project_path.join(".beads").join("attachments");
    let issue_dir = resolve_attachment_dir(&attachments_dir, &issue_id);

    if !issue_dir.exists() || !issue_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut files: Vec<AttachmentFile> = Vec::new();

    let entries = fs::read_dir(&issue_dir)
        .map_err(|e| format!("Failed to read attachment directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }

        let name = entry.file_name().to_string_lossy().to_string();
        // Skip legacy index.json files
        if name == "index.json" { continue; }

        let file_type = classify_attachment(&name);
        // Only return images and markdown
        if file_type == "other" { continue; }

        let modified = entry.metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        files.push(AttachmentFile {
            filename: name,
            file_type: file_type.to_string(),
            path: path.to_string_lossy().to_string(),
            modified,
        });
    }

    // Sort by mtime descending (newest first)
    files.sort_by(|a, b| b.modified.cmp(&a.modified));

    Ok(files)
}

/// Delete an attachment file by filename within an issue's attachment directory.
#[tauri::command]
pub(crate) async fn delete_attachment(project_path: String, issue_id: String, filename: String) -> Result<(), String> {
    log::info!("[delete_attachment] project: {}, issue: {}, file: {}", project_path, issue_id, filename);

    // Security: reject path traversal
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err("Invalid filename".to_string());
    }

    let abs_project_path = if project_path == "." || project_path.is_empty() {
        env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?
    } else {
        let p = PathBuf::from(&project_path);
        if p.is_relative() {
            let cwd = env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?;
            cwd.join(&p)
        } else {
            p
        }
    };

    let abs_project_path = abs_project_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve project path: {}", e))?;

    let attachments_dir = abs_project_path.join(".beads").join("attachments");
    let issue_dir = resolve_attachment_dir(&attachments_dir, &issue_id);
    let file_path = issue_dir.join(&filename);

    if !file_path.exists() {
        log::info!("[delete_attachment] File does not exist: {:?}", file_path);
        return Ok(());
    }

    // Security: verify file is inside .beads/attachments/
    let canonical = file_path.canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;
    let canonical_str = canonical.to_string_lossy();
    if !canonical_str.contains("/.beads/attachments/") {
        return Err("Can only delete files inside .beads/attachments/".to_string());
    }

    fs::remove_file(&file_path)
        .map_err(|e| format!("Failed to delete file: {}", e))?;

    log::info!("[delete_attachment] Deleted: {:?}", file_path);

    // Cleanup empty folder (issue_dir already resolved above via resolve_attachment_dir)
    if issue_dir.exists() {
        if let Ok(entries) = fs::read_dir(&issue_dir) {
            // Count non-index.json entries
            let count = entries.flatten()
                .filter(|e| e.file_name().to_string_lossy() != "index.json")
                .count();
            if count == 0 {
                // Remove index.json if present, then the directory
                let _ = fs::remove_file(issue_dir.join("index.json"));
                let _ = fs::remove_dir(&issue_dir);
                log::info!("[delete_attachment] Cleaned up empty folder: {:?}", issue_dir);
            }
        }
    }

    Ok(())
}

#[derive(Debug, Serialize)]
pub struct TextData {
    pub content: String,
}

#[tauri::command]
pub(crate) async fn read_text_file(path: String) -> Result<TextData, String> {
    log_info!("[read_text_file] Reading: {}", path);

    // Security: Only allow markdown file extensions
    let path_lower = path.to_lowercase();
    let is_markdown = path_lower.ends_with(".md") || path_lower.ends_with(".markdown");

    if !is_markdown {
        return Err("Only markdown files are allowed".to_string());
    }

    // Verify file exists
    if !std::path::Path::new(&path).exists() {
        return Err(format!("File not found: {}", path));
    }

    // Security: Canonicalize to resolve symlinks/.. and verify inside .beads/attachments/
    let canonical = std::path::Path::new(&path).canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;
    let canonical_str = canonical.to_string_lossy();
    if !canonical_str.contains("/.beads/attachments/") {
        log_warn!("[read_text_file] Refusing to read file outside attachments: {} (resolved: {})", path, canonical_str);
        return Err("Can only read files inside .beads/attachments/".to_string());
    }

    // Read file as UTF-8
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    Ok(TextData { content })
}

#[tauri::command]
pub(crate) async fn write_text_file(path: String, content: String) -> Result<(), String> {
    log_info!("[write_text_file] Writing: {}", path);

    // Security: Only allow markdown file extensions
    let path_lower = path.to_lowercase();
    let is_markdown = path_lower.ends_with(".md") || path_lower.ends_with(".markdown");

    if !is_markdown {
        return Err("Only markdown files are allowed".to_string());
    }

    // Verify file exists (no creation of new files)
    if !std::path::Path::new(&path).exists() {
        return Err(format!("File not found: {}", path));
    }

    // Security: Canonicalize to resolve symlinks/.. and verify inside .beads/attachments/
    let canonical = std::path::Path::new(&path).canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;
    let canonical_str = canonical.to_string_lossy();
    if !canonical_str.contains("/.beads/attachments/") {
        log_warn!("[write_text_file] Refusing to write file outside attachments: {} (resolved: {})", path, canonical_str);
        return Err("Can only write files inside .beads/attachments/".to_string());
    }

    // Write content to file
    fs::write(&path, &content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    log_info!("[write_text_file] Written {} bytes to {}", content.len(), path);
    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;

    // ---- Attachment helpers (#5) ------------------------------------------------

    #[test]
    fn base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn base64_encode_single_byte() {
        assert_eq!(base64_encode(b"f"), "Zg==");
    }

    #[test]
    fn base64_encode_two_bytes() {
        assert_eq!(base64_encode(b"fo"), "Zm8=");
    }

    #[test]
    fn base64_encode_three_bytes() {
        assert_eq!(base64_encode(b"foo"), "Zm9v");
    }

    #[test]
    fn base64_encode_longer_string() {
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn sanitize_filename_keeps_alphanumeric() {
        assert_eq!(sanitize_filename("hello123.txt"), "hello123.txt");
    }

    #[test]
    fn sanitize_filename_converts_to_lowercase() {
        assert_eq!(sanitize_filename("Hello.TXT"), "hello.txt");
    }

    #[test]
    fn sanitize_filename_replaces_spaces() {
        assert_eq!(sanitize_filename("hello world.txt"), "hello-world.txt");
    }

    #[test]
    fn sanitize_filename_strips_diacritics() {
        assert_eq!(sanitize_filename("café.txt"), "cafe.txt");
    }

    #[test]
    fn sanitize_filename_removes_unsafe_chars() {
        let result = sanitize_filename("file<name>.txt");
        // < and > should be removed
        assert!(!result.contains('<'));
        assert!(!result.contains('>'));
    }

    #[test]
    fn sanitize_filename_collapses_dashes() {
        assert_eq!(sanitize_filename("hello  world.txt"), "hello-world.txt");
    }

    #[test]
    fn sanitize_filename_fallback_for_empty_stem() {
        let result = sanitize_filename(".txt");
        assert!(!result.is_empty());
        assert!(result.ends_with(".txt"));
    }

    #[test]
    fn issue_short_id_extracts_after_last_dash() {
        assert_eq!(issue_short_id("proj-abc"), "abc");
        assert_eq!(issue_short_id("proj-abc-def"), "def");
        assert_eq!(issue_short_id("long-project-name-xyz"), "xyz");
    }

    #[test]
    fn issue_short_id_returns_full_id_without_dash() {
        assert_eq!(issue_short_id("nodash"), "nodash");
    }

    #[test]
    fn issue_short_id_handles_dots() {
        assert_eq!(issue_short_id("proj-abc.1"), "abc.1");
    }

    #[test]
    fn classify_attachment_images() {
        assert_eq!(classify_attachment("photo.png"), "image");
        assert_eq!(classify_attachment("image.jpg"), "image");
        assert_eq!(classify_attachment("picture.jpeg"), "image");
        assert_eq!(classify_attachment("graphic.webp"), "image");
        assert_eq!(classify_attachment("pic.gif"), "image");
    }

    #[test]
    fn classify_attachment_images_case_insensitive() {
        assert_eq!(classify_attachment("PHOTO.PNG"), "image");
        assert_eq!(classify_attachment("Image.JPG"), "image");
    }

    #[test]
    fn classify_attachment_markdown() {
        assert_eq!(classify_attachment("notes.md"), "markdown");
        assert_eq!(classify_attachment("document.markdown"), "markdown");
    }

    #[test]
    fn classify_attachment_other() {
        assert_eq!(classify_attachment("file.txt"), "other");
        assert_eq!(classify_attachment("data.json"), "other");
        assert_eq!(classify_attachment("unknown.xyz"), "other");
    }

    #[test]
    fn resolve_duplicate_filename_no_clash() {
        let temp_dir = std::env::temp_dir().join(format!("beads_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let result = resolve_duplicate_filename(&temp_dir, "newfile.txt");
        assert_eq!(result, "newfile.txt");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn resolve_duplicate_filename_with_clash() {
        let temp_dir = std::env::temp_dir().join(format!("beads_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
        let _ = std::fs::create_dir_all(&temp_dir);

        // Create a file
        let _ = std::fs::write(temp_dir.join("test.txt"), "");

        let result = resolve_duplicate_filename(&temp_dir, "test.txt");
        assert_eq!(result, "test-1.txt");

        // Create the -1 version
        let _ = std::fs::write(temp_dir.join(&result), "");

        let result2 = resolve_duplicate_filename(&temp_dir, "test.txt");
        assert_eq!(result2, "test-2.txt");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn resolve_attachment_dir_uses_short_id() {
        let base = std::path::PathBuf::from("/base");
        let result = resolve_attachment_dir(&base, "proj-abc");
        assert_eq!(result, base.join("abc"));
    }

    #[test]
    fn resolve_attachment_dir_long_prefix() {
        let base = std::path::PathBuf::from("/base");
        let result = resolve_attachment_dir(&base, "long-prefix-name-xyz");
        assert_eq!(result, base.join("xyz"));
    }


}
