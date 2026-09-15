use crate::attachment_refs::is_real_external_ref;
use btit_types::CliClient;
use crate::attachments::issue_short_id;
use crate::cli::{get_cli_client_info, project_uses_dolt, supports_daemon_flag, uses_jsonl_files, AppInvoker};
use btit_beads::error::BeadsError;
use btit_cli::{command::new_command, ops, path::get_extended_path};
use btit_types::ProjectRef;
use crate::config::get_cli_binary;
use std::env;
use std::sync::Mutex;
use std::time::Instant;

// Sync cooldown: skip redundant syncs within 10 seconds
pub(crate) static LAST_SYNC_TIME: Mutex<Option<Instant>> = Mutex::new(None);
pub(crate) const SYNC_COOLDOWN_SECS: u64 = 10;

pub(crate) fn ensure_refs_migrated_v3(beads_dir: &std::path::Path, working_dir: &str) {
    if beads_dir.join(".migrated-attachments").exists() {
        return;
    }
    let jsonl_path = beads_dir.join("issues.jsonl");
    if !jsonl_path.exists() {
        let _ = std::fs::write(beads_dir.join(".migrated-attachments"), "");
        return;
    }

    let content = match std::fs::read_to_string(&jsonl_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Quick scan: does any line have non-real external refs?
    let mut needs_migration = false;
    for line in content.lines() {
        if line.trim().is_empty() { continue; }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ext_ref = v.get("external_ref").and_then(|r| r.as_str()).unwrap_or("");
        // Non-real ref (att:, paths, cleared: sentinels, etc.)
        if ext_ref.is_empty() { continue; }
        for r in ext_ref.split(|c: char| c == '\n' || c == '|') {
            let trimmed = r.trim();
            if !trimmed.is_empty() && !is_real_external_ref(trimmed) {
                needs_migration = true;
                break;
            }
        }
        if needs_migration { break; }
    }

    // Also check if attachment folders need renaming
    let attachments_dir_check = beads_dir.join("attachments");
    let mut needs_folder_work = false;
    if attachments_dir_check.exists() {
        if let Ok(entries) = std::fs::read_dir(&attachments_dir_check) {
            for entry in entries.flatten() {
                if !entry.path().is_dir() { continue; }
                let name = entry.file_name().to_string_lossy().to_string();
                if issue_short_id(&name) != name {
                    needs_folder_work = true;
                    break;
                }
            }
        }
    }

    if !needs_migration && !needs_folder_work {
        let _ = std::fs::write(beads_dir.join(".migrated-attachments"), "");
        return;
    }

    log_info!("[sync] Auto-migrating v3 (refs={}, folders={}) for: {}", needs_migration, needs_folder_work, working_dir);

    // Backup
    let backup_path = beads_dir.join("issues.jsonl.bak-refs-v3-migration");
    if std::fs::copy(&jsonl_path, &backup_path).is_err() {
        log_error!("[sync] Failed to backup JSONL for v3 migration, skipping");
        return;
    }

    // Migrate: strip non-real refs, deduplicate
    let mut refs_updated: u32 = 0;
    let mut output_lines: Vec<String> = Vec::new();
    let mut seen_refs: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            output_lines.push(line.to_string());
            continue;
        }
        let mut v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => { output_lines.push(line.to_string()); continue; }
        };

        let issue_id = v.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();

        let ext_ref = v.get("external_ref").and_then(|r| r.as_str()).unwrap_or("").to_string();

        // Parse existing refs, keep only real external ones
        let real_refs: Vec<String> = if ext_ref.is_empty() {
            vec![]
        } else {
            ext_ref.split(|c: char| c == '\n' || c == '|')
                .map(|r| r.trim())
                .filter(|r| is_real_external_ref(r))
                .map(String::from)
                .collect()
        };

        let mut new_ref = if real_refs.is_empty() {
            String::new()
        } else {
            real_refs.join("|")
        };

        // Deduplicate: if another issue already has this exact ref, clear it
        if !new_ref.is_empty() && seen_refs.contains(&new_ref) {
            log_info!("[sync] Duplicate external_ref '{}' for issue {}, clearing", new_ref, issue_id);
            new_ref = String::new();
        }
        if !new_ref.is_empty() {
            seen_refs.insert(new_ref.clone());
        }

        if new_ref != ext_ref {
            v["external_ref"] = serde_json::Value::String(new_ref);
            refs_updated += 1;
            output_lines.push(serde_json::to_string(&v).unwrap_or_else(|_| line.to_string()));
            continue;
        }

        // Track existing refs that weren't modified too
        if let Some(ext_ref) = v.get("external_ref").and_then(|r| r.as_str()) {
            seen_refs.insert(ext_ref.to_string());
        }

        output_lines.push(line.to_string());
    }

    if refs_updated > 0 {
        let new_content = output_lines.join("\n");
        if std::fs::write(&jsonl_path, &new_content).is_err() {
            log_error!("[sync] Failed to write migrated JSONL");
            return;
        }
        log_info!("[sync] Refs v3 migration: {} ref(s) cleaned", refs_updated);
    }

    // Rename attachment folders: {full-id}/ → {short-id}/
    let attachments_dir = beads_dir.join("attachments");
    if attachments_dir.exists() {
        let mut renamed = 0u32;
        if let Ok(entries) = std::fs::read_dir(&attachments_dir) {
            let dirs: Vec<_> = entries.flatten()
                .filter(|e| e.path().is_dir())
                .collect();
            for entry in dirs {
                let folder_name = entry.file_name().to_string_lossy().to_string();
                let short = issue_short_id(&folder_name);
                if short != folder_name {
                    let target = attachments_dir.join(short);
                    if target.exists() {
                        log_warn!("[sync] Cannot rename '{}' → '{}': target already exists", folder_name, short);
                        continue;
                    }
                    if std::fs::rename(entry.path(), &target).is_ok() {
                        renamed += 1;
                    } else {
                        log_warn!("[sync] Failed to rename '{}' → '{}'", folder_name, short);
                    }
                }
            }
        }
        if renamed > 0 {
            log_info!("[sync] Renamed {} attachment folder(s) to short IDs", renamed);
        }
    }

    let _ = std::fs::write(beads_dir.join(".migrated-attachments"), "");
    // Signal for the frontend to show a notification
    let _ = std::fs::write(beads_dir.join(".migrated-attachments-notify"), "");
    log_info!("[sync] Migration v3 complete (refs cleaned + folders renamed)");
}

/// Sync the beads database before read operations to ensure data is up-to-date
/// Uses bidirectional sync to preserve local changes while getting remote updates
/// Has a cooldown to avoid redundant syncs within the same poll cycle
pub(crate) fn sync_bd_database(cwd: Option<&str>) {
    let working_dir = cwd
        .map(String::from)
        .or_else(|| env::var("BEADS_PATH").ok())
        .unwrap_or_else(|| {
            env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });

    // Dolt backend handles its own sync via git — skip bd sync
    let beads_dir = std::path::Path::new(&working_dir).join(".beads");
    if project_uses_dolt(&beads_dir) {
        log_info!("[sync] Skipping — Dolt backend handles sync via git");
        return;
    }

    // Check cooldown — skip if synced recently
    {
        let last = LAST_SYNC_TIME.lock().unwrap();
        if let Some(t) = *last {
            if t.elapsed().as_secs() < SYNC_COOLDOWN_SECS {
                log_info!("[sync] Skipping — cooldown active ({:.1}s ago)", t.elapsed().as_secs_f32());
                return;
            }
        }
    }

    log_info!("[sync] Starting bidirectional sync for: {}", working_dir);

    // Auto-migrate refs v3 before sync if needed (prevents UNIQUE constraint errors)
    ensure_refs_migrated_v3(&beads_dir, &working_dir);

    // Run bd sync (bidirectional - exports local changes AND imports remote changes)
    let binary = get_cli_binary();
    match ops::sync(&AppInvoker, &ProjectRef::local(Some(working_dir.clone())), supports_daemon_flag()) {
        Ok(()) => {
            log_info!("[sync] Sync completed successfully");
            // Update cooldown timestamp
            let mut last = LAST_SYNC_TIME.lock().unwrap();
            *last = Some(Instant::now());
        }
        Err(BeadsError::CommandFailed { stderr, .. }) => {
            log_warn!(
                "[sync] {} sync failed: {}",
                binary,
                stderr
            );
        }
        Err(BeadsError::Spawn { source, .. }) => {
            log_error!("[sync] Failed to run {} sync: {}", binary, source);
        }
        Err(e) => {
            log_error!("[sync] Failed to run {} sync: {}", binary, e);
        }
    }
}


// ============================================================================
// Tauri Commands
// ============================================================================

#[tauri::command]
pub(crate) async fn bd_sync(cwd: Option<String>) -> Result<(), String> {
    let working_dir = cwd
        .or_else(|| env::var("BEADS_PATH").ok())
        .unwrap_or_else(|| {
            env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });

    // Dolt backend handles its own sync via git — skip bd sync
    let beads_dir = std::path::Path::new(&working_dir).join(".beads");
    if project_uses_dolt(&beads_dir) {
        log_info!("[bd_sync] Skipping — Dolt backend handles sync via git");
        return Ok(());
    }

    let binary = get_cli_binary();
    log_info!("[bd_sync] Manual sync requested for: {}", working_dir);

    match ops::sync(&AppInvoker, &ProjectRef::local(Some(working_dir.clone())), supports_daemon_flag()) {
        Ok(()) => {}
        Err(BeadsError::CommandFailed { stderr, .. }) => {
            log_error!("[bd_sync] Sync failed: {}", stderr.trim());
            return Err(format!("Sync failed: {}", stderr.trim()));
        }
        Err(BeadsError::Spawn { source, .. }) => {
            return Err(format!("Failed to run {} sync: {}", binary, source));
        }
        Err(e) => return Err(e.to_string()),
    }

    log_info!("[bd_sync] Sync completed successfully");
    // Reset cooldown so subsequent reads pick up the fresh sync
    let mut last = LAST_SYNC_TIME.lock().unwrap();
    *last = Some(Instant::now());
    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct RepairResult {
    success: bool,
    message: String,
    backup_path: Option<String>,
}

#[tauri::command]
pub(crate) async fn bd_repair_database(cwd: Option<String>) -> Result<RepairResult, String> {
    let working_dir = cwd
        .or_else(|| env::var("BEADS_PATH").ok())
        .unwrap_or_else(|| {
            env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });

    log_info!("[bd_repair] Starting database repair for: {}", working_dir);

    let beads_dir = std::path::Path::new(&working_dir).join(".beads");

    // Check if .beads directory exists
    if !beads_dir.exists() {
        return Err("No .beads directory found in this project".to_string());
    }

    // Dolt backend: use `bd doctor --fix --yes`
    if project_uses_dolt(&beads_dir) {
        log_info!("[bd_repair] Using Dolt-based repair strategy (bd >= 0.50.0): bd doctor --fix --yes");
        let binary = get_cli_binary();
        let output = new_command(&binary)
            .args(&["doctor", "--fix", "--yes"])
            .current_dir(&working_dir)
            .env("PATH", get_extended_path())
            .env("BEADS_PATH", &working_dir)
            .output()
            .map_err(|e| format!("Failed to run bd doctor: {}", e))?;

        return if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            log_info!("[bd_repair] Dolt repair successful: {}", stdout.trim());
            Ok(RepairResult {
                success: true,
                message: format!("Database repaired via bd doctor. {}", stdout.trim()),
                backup_path: None,
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log_error!("[bd_repair] Dolt repair failed: {}", stderr.trim());
            Err(format!("Repair failed: {}", stderr.trim()))
        };
    }

    // SQLite backend: original repair logic
    let db_path = beads_dir.join("beads.db");
    let jsonl_path = beads_dir.join("issues.jsonl");
    let backup_path = beads_dir.join("beads.db.backup");

    // Check if database exists
    if !db_path.exists() {
        return Ok(RepairResult {
            success: true,
            message: "No database to repair - it will be created on next operation".to_string(),
            backup_path: None,
        });
    }

    // For bd < 0.50.0: require issues.jsonl for repair (db is rebuilt from JSONL)
    if uses_jsonl_files() {
        let jsonl_size = std::fs::metadata(&jsonl_path)
            .map(|m| m.len())
            .unwrap_or(0);

        if !jsonl_path.exists() || jsonl_size == 0 {
            return Err("Cannot repair: issues.jsonl is missing or empty. Your data would be lost.".to_string());
        }
        log_info!("[bd_repair] Using JSONL-based repair strategy (bd < 0.50.0)");
    } else {
        log_info!("[bd_repair] Using repair strategy for unknown version");
    }

    // Create backup of current database
    if let Err(e) = std::fs::copy(&db_path, &backup_path) {
        log_error!("[bd_repair] Failed to create backup: {}", e);
        return Err(format!("Failed to create backup: {}", e));
    }
    log_info!("[bd_repair] Backup created at: {:?}", backup_path);

    // Remove database files
    std::fs::remove_file(&db_path).ok();
    std::fs::remove_file(beads_dir.join("beads.db-shm")).ok();
    std::fs::remove_file(beads_dir.join("beads.db-wal")).ok();
    log_info!("[bd_repair] Removed old database files");

    // Test that bd can now work (it will recreate the database)
    let mut test_args = vec!["list", "--limit=1"];
    if supports_daemon_flag() {
        test_args.push("--no-daemon");
    }
    test_args.push("--json");
    let test_output = new_command(&get_cli_binary())
        .args(&test_args)
        .current_dir(&working_dir)
        .env("PATH", get_extended_path())
        .env("BEADS_PATH", &working_dir)
        .output();

    match test_output {
        Ok(output) if output.status.success() => {
            log_info!("[bd_repair] Repair successful - database recreated");
            Ok(RepairResult {
                success: true,
                message: "Database repaired successfully. Your issues have been restored from the backup file.".to_string(),
                backup_path: Some(backup_path.to_string_lossy().to_string()),
            })
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log_error!("[bd_repair] Repair verification failed: {}", stderr);
            Err(format!("Repair failed during verification: {}", stderr))
        }
        Err(e) => {
            log_error!("[bd_repair] Failed to verify repair: {}", e);
            Err(format!("Failed to verify repair: {}", e))
        }
    }
}

// ============================================================================
// Dolt Migration
// ============================================================================

#[derive(Debug, serde::Serialize)]
pub(crate) struct MigrateResult {
    success: bool,
    message: String,
}

/// Remove orphaned Dolt lock files that block database access.
///
/// Uses `lsof` to check if any process actually holds the lock file open.
/// - If no process has it open → orphaned lock from a crashed/finished bd → safe to remove.
/// - If a process has it open → active agent (Claude Code, Gastown, etc.) → leave it alone.
///
/// This is the only reliable way to distinguish a stale lock from an active one,
/// regardless of timing. bd 0.55+ in embedded Dolt mode leaves noms/LOCK behind
/// after every command, so these accumulate and block subsequent operations.

#[derive(Debug, serde::Serialize)]
pub(crate) struct CleanupResult {
    removed: Vec<String>,
}

/// Stale lock cleanup — currently a no-op.
///
/// bd 0.55 in embedded Dolt mode leaves lock files (dolt-access.lock, noms/LOCK)
/// after every command. These locks are NOT safe to remove externally:
/// - Removing noms/LOCK causes Dolt SIGSEGV (nil pointer dereference) on next bd call
/// - Removing dolt-access.lock also triggers the same Dolt crash
///
/// This is a bd/Dolt bug that needs to be fixed upstream. The command is kept as a
/// no-op so the frontend call doesn't need to change when a fix becomes available.
#[tauri::command]
pub(crate) async fn bd_cleanup_stale_locks(cwd: Option<String>) -> Result<CleanupResult, String> {
    let _ = cwd; // suppress unused warning
    Ok(CleanupResult { removed: vec![] })
}

/// Check if a project needs Dolt migration.
/// Returns true when bd >= 0.50, project has .beads/, but is not fully migrated to Dolt.
/// Detects both "never migrated" and "partially migrated" (dolt/ dir exists but .dolt marker missing).
#[derive(Debug, serde::Serialize)]
pub(crate) struct MigrationStatus {
    needs_migration: bool,
    reason: String,
}

#[tauri::command]
pub(crate) async fn bd_check_needs_migration(cwd: Option<String>) -> Result<MigrationStatus, String> {
    let working_dir = cwd
        .or_else(|| env::var("BEADS_PATH").ok())
        .unwrap_or_else(|| {
            env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });

    let beads_dir = std::path::Path::new(&working_dir).join(".beads");

    if !beads_dir.exists() {
        return Ok(MigrationStatus {
            needs_migration: false,
            reason: "No .beads directory".to_string(),
        });
    }

    // Check bd version — only bd >= 0.50 requires Dolt
    match get_cli_client_info() {
        Some((CliClient::Bd, major, minor, _)) if major > 0 || minor >= 50 => {
            // bd >= 0.50: check if project is fully migrated
        }
        _ => {
            return Ok(MigrationStatus {
                needs_migration: false,
                reason: "bd version does not require Dolt".to_string(),
            });
        }
    }

    // Already fully using Dolt? (.beads/.dolt exists)
    if project_uses_dolt(&beads_dir) {
        return Ok(MigrationStatus {
            needs_migration: false,
            reason: "Already using Dolt backend".to_string(),
        });
    }

    // Check for partial migration (dolt/ dir exists but not complete)
    let dolt_dir = beads_dir.join("dolt");
    if dolt_dir.exists() {
        return Ok(MigrationStatus {
            needs_migration: true,
            reason: "Partial migration detected (dolt/ exists but migration incomplete)".to_string(),
        });
    }

    // Has JSONL data but no Dolt — needs migration
    let jsonl_path = beads_dir.join("issues.jsonl");
    if jsonl_path.exists() {
        let jsonl_size = std::fs::metadata(&jsonl_path).map(|m| m.len()).unwrap_or(0);
        if jsonl_size > 0 {
            return Ok(MigrationStatus {
                needs_migration: true,
                reason: "SQLite/JSONL project needs Dolt migration".to_string(),
            });
        }
    }

    // Has SQLite db but no Dolt
    let db_path = beads_dir.join("beads.db");
    if db_path.exists() {
        return Ok(MigrationStatus {
            needs_migration: true,
            reason: "SQLite project needs Dolt migration".to_string(),
        });
    }

    // Empty project — no migration needed (bd init will create Dolt directly)
    Ok(MigrationStatus {
        needs_migration: false,
        reason: "Empty project".to_string(),
    })
}

/// Re-prefix an issue ID if it uses a non-target prefix
pub(crate) fn reprefix_id(id: &str, target_prefix: &str, prefix_counts: &std::collections::HashMap<String, usize>) -> String {
    if let Some(last_dash) = id.rfind('-') {
        let current_prefix = &id[..last_dash];
        if current_prefix != target_prefix && prefix_counts.contains_key(current_prefix) {
            return format!("{}{}", target_prefix, &id[last_dash..]);
        }
    }
    id.to_string()
}

#[tauri::command]
pub(crate) async fn bd_migrate_to_dolt(cwd: Option<String>) -> Result<MigrateResult, String> {
    let working_dir = cwd
        .or_else(|| env::var("BEADS_PATH").ok())
        .unwrap_or_else(|| {
            env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });

    log_info!("[bd_migrate] Starting Dolt migration for: {}", working_dir);

    let beads_dir = std::path::Path::new(&working_dir).join(".beads");

    // Check if .beads directory exists
    if !beads_dir.exists() {
        return Err("No .beads directory found in this project".to_string());
    }

    // Already using Dolt?
    if project_uses_dolt(&beads_dir) {
        return Ok(MigrateResult {
            success: true,
            message: "Project already uses the Dolt backend.".to_string(),
        });
    }

    // Verify bd >= 0.50
    if let Some((_, major, minor, _)) = get_cli_client_info() {
        if major == 0 && minor < 50 {
            return Err(format!(
                "bd version 0.50+ is required for Dolt migration (current: {}.{})",
                major, minor
            ));
        }
    } else {
        return Err("Could not determine bd version".to_string());
    }

    // Clean up partial migration if dolt/ directory exists
    let dolt_dir = beads_dir.join("dolt");
    if dolt_dir.exists() {
        log_info!("[bd_migrate] Removing partial dolt/ directory for re-migration");
        std::fs::remove_dir_all(&dolt_dir)
            .map_err(|e| format!("Failed to remove partial dolt/ directory: {}", e))?;
    }

    // Remove dolt-access.lock if present
    let dolt_lock = beads_dir.join("dolt-access.lock");
    if dolt_lock.exists() {
        std::fs::remove_file(&dolt_lock).ok();
    }

    // Try `bd migrate --to-dolt --yes` first
    let binary = get_cli_binary();
    let output = new_command(&binary)
        .args(&["migrate", "--to-dolt", "--yes"])
        .current_dir(&working_dir)
        .env("PATH", get_extended_path())
        .env("BEADS_PATH", &working_dir)
        .output()
        .map_err(|e| format!("Failed to run bd migrate: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        log_info!("[bd_migrate] Migration via bd migrate successful: {}", stdout.trim());
        return Ok(MigrateResult {
            success: true,
            message: format!("Migration to Dolt completed successfully. {}", stdout.trim()),
        });
    }

    // bd migrate failed (typically: corrupt SQLite, missing table, etc.)
    // Fallback: bd init + bd import from JSONL
    let stderr_migrate = String::from_utf8_lossy(&output.stderr);
    log_info!("[bd_migrate] bd migrate failed ({}), trying init+import fallback", stderr_migrate.trim());

    let jsonl_path = beads_dir.join("issues.jsonl");
    if !jsonl_path.exists() || std::fs::metadata(&jsonl_path).map(|m| m.len()).unwrap_or(0) == 0 {
        // Empty project — no JSONL data to import, just run bd init
        log_info!("[bd_migrate] No issues.jsonl data — empty project, attempting init-only migration");
        // Rename existing .db files to .db.backup so bd init doesn't refuse
        if let Ok(entries) = std::fs::read_dir(&beads_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".db") && !name.ends_with(".db.backup") {
                    let src = entry.path();
                    let dst = beads_dir.join(format!("{}.backup", name));
                    log_info!("[bd_migrate] Renaming {} -> {}", src.display(), dst.display());
                    std::fs::rename(&src, &dst).ok();
                }
                // Also remove .db-shm and .db-wal
                if name.ends_with(".db-shm") || name.ends_with(".db-wal") {
                    std::fs::remove_file(entry.path()).ok();
                }
            }
        }
        let init_output = new_command(&binary)
            .args(&["init", "--prefix", "project"])
            .current_dir(&working_dir)
            .env("PATH", get_extended_path())
            .env("BEADS_PATH", &working_dir)
            .output()
            .map_err(|e| format!("Failed to run bd init: {}", e))?;
        if init_output.status.success() {
            log_info!("[bd_migrate] Empty project initialized with Dolt backend");
            return Ok(MigrateResult {
                success: true,
                message: "Migration complete (empty project — initialized with Dolt backend)".to_string(),
            });
        }
        let init_stderr = String::from_utf8_lossy(&init_output.stderr);
        return Err(format!(
            "Migration failed (empty project, bd init also failed): {}. Original error: {}",
            init_stderr.trim(), stderr_migrate.trim()
        ));
    }

    // Detect prefix from JSONL — use the most common prefix
    let jsonl_content = std::fs::read_to_string(&jsonl_path)
        .map_err(|e| format!("Failed to read issues.jsonl: {}", e))?;
    let mut prefix_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for line in jsonl_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
                if let Some(last_dash) = id.rfind('-') {
                    let suffix = &id[last_dash + 1..];
                    if suffix.chars().all(|c| c.is_alphanumeric()) && !suffix.is_empty() {
                        *prefix_counts.entry(id[..last_dash].to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    let prefix = prefix_counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(p, _)| p.clone())
        .ok_or_else(|| "Could not detect issue prefix from issues.jsonl".to_string())?;

    if prefix_counts.len() > 1 {
        log_info!(
            "[bd_migrate] Multiple prefixes found: {:?}. Using most common: {}",
            prefix_counts, prefix
        );
    }
    log_info!("[bd_migrate] Detected prefix: {}", prefix);

    // Clean dolt dir again (bd migrate may have created a partial one)
    if dolt_dir.exists() {
        log_info!("[bd_migrate] Removing dolt/ directory before init");
        if let Err(e) = std::fs::remove_dir_all(&dolt_dir) {
            log_error!("[bd_migrate] Failed to remove dolt/: {}", e);
            return Err(format!("Failed to clean up dolt/ directory: {}", e));
        }
    }
    // Remove dolt-access.lock
    let dolt_lock2 = beads_dir.join("dolt-access.lock");
    if dolt_lock2.exists() {
        std::fs::remove_file(&dolt_lock2).ok();
    }
    // Backup main SQLite .db file (for comment restoration), then remove all SQLite files
    if let Ok(entries) = std::fs::read_dir(&beads_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".db") && !name.ends_with(".db.backup") {
                // Rename to .backup before deleting (preserves comments for Step 6)
                let backup_name = format!("{}.backup", name);
                let backup_path = beads_dir.join(&backup_name);
                if !backup_path.exists() {
                    log_info!("[bd_migrate] Backing up SQLite: {} -> {}", name, backup_name);
                    std::fs::rename(entry.path(), &backup_path).ok();
                } else {
                    log_info!("[bd_migrate] Removing SQLite file: {} (backup already exists)", name);
                    std::fs::remove_file(entry.path()).ok();
                }
            } else if name.ends_with(".db-shm") || name.ends_with(".db-wal") || name.ends_with(".db?mode=ro") {
                log_info!("[bd_migrate] Removing SQLite file: {}", name);
                std::fs::remove_file(entry.path()).ok();
            }
        }
    }

    // Reset metadata.json if it was set to dolt by a previous failed attempt
    let metadata_path = beads_dir.join("metadata.json");
    if metadata_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&metadata_path) {
            if content.contains("\"backend\":\"dolt\"") || content.contains("\"backend\": \"dolt\"") {
                log_info!("[bd_migrate] Resetting metadata.json backend from dolt to sqlite");
                std::fs::remove_file(&metadata_path).ok();
            }
        }
    }

    // Remove .local_version (stale after cleanup)
    let local_version = beads_dir.join(".local_version");
    if local_version.exists() {
        std::fs::remove_file(&local_version).ok();
    }

    // Step 1: bd init --prefix <prefix>
    let init_output = new_command(&binary)
        .args(&["init", "--prefix", &prefix])
        .current_dir(&working_dir)
        .env("PATH", get_extended_path())
        .env("BEADS_PATH", &working_dir)
        .output()
        .map_err(|e| format!("Failed to run bd init: {}", e))?;

    if !init_output.status.success() {
        let stderr = String::from_utf8_lossy(&init_output.stderr);
        return Err(format!("bd init failed: {}", stderr.trim()));
    }
    log_info!("[bd_migrate] bd init successful");

    // Step 2: Filter tombstone issues and sanitize fields for Dolt compatibility
    let temp_jsonl = beads_dir.join("_migrate_clean.jsonl");
    {
        let mut clean_lines = Vec::new();
        let mut skipped = 0u32;
        for line in jsonl_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(mut v) => {
                    if v.get("status").and_then(|s| s.as_str()) == Some("tombstone") {
                        skipped += 1;
                        continue;
                    }
                    // Re-prefix issues with a different prefix to match the target
                    if let Some(id) = v.get("id").and_then(|i| i.as_str()).map(String::from) {
                        if let Some(last_dash) = id.rfind('-') {
                            let issue_prefix = &id[..last_dash];
                            if issue_prefix != prefix {
                                let suffix = &id[last_dash..]; // includes the '-'
                                let new_id = format!("{}{}", prefix, suffix);
                                let old_prefix = issue_prefix.to_string();
                                log_info!("[bd_migrate] Re-prefixing {} -> {}", id, new_id);
                                let obj = v.as_object_mut().unwrap();
                                obj.insert("id".to_string(), serde_json::Value::String(new_id));
                                // Re-prefix dependency references
                                if let Some(deps) = obj.get_mut("dependencies").and_then(|d| d.as_array_mut()) {
                                    for dep in deps.iter_mut() {
                                        if let Some(dep_obj) = dep.as_object_mut() {
                                            for key in &["issue_id", "depends_on_id"] {
                                                if let Some(val) = dep_obj.get(*key).and_then(|v| v.as_str()).map(String::from) {
                                                    if val.starts_with(&old_prefix) {
                                                        let new_val = format!("{}{}", prefix, &val[old_prefix.len()..]);
                                                        dep_obj.insert(key.to_string(), serde_json::Value::String(new_val));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Truncate external_ref if it contains multiple lines (attachment paths)
                    // Dolt's external_ref column can't hold multi-line values with long paths
                    // Keep only the first line (the meaningful ref: redmine ID, URL, etc.)
                    let needs_truncate = v.get("external_ref")
                        .and_then(|e| e.as_str())
                        .map(|s| s.contains('\n') || s.len() > 100)
                        .unwrap_or(false);
                    if needs_truncate {
                        let ext_ref = v["external_ref"].as_str().unwrap();
                        let first_line = ext_ref.lines().next().unwrap_or("").to_string();
                        let issue_id = v.get("id").and_then(|i| i.as_str()).unwrap_or("?").to_string();
                        let orig_len = ext_ref.len();
                        v.as_object_mut().unwrap().insert(
                            "external_ref".to_string(),
                            serde_json::Value::String(first_line),
                        );
                        log_info!(
                            "[bd_migrate] Truncated external_ref for issue {} (was {} chars)",
                            issue_id, orig_len
                        );
                    }
                    clean_lines.push(serde_json::to_string(&v).unwrap_or_else(|_| trimmed.to_string()));
                }
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            }
        }
        log_info!(
            "[bd_migrate] Filtered JSONL: {} valid, {} skipped (tombstone/malformed)",
            clean_lines.len(),
            skipped
        );

        // Empty project — no issues to import, just init is enough
        if clean_lines.is_empty() {
            log_info!("[bd_migrate] No issues to import — empty project, init-only migration");
            return Ok(MigrateResult {
                success: true,
                message: "Migration complete (empty project — initialized with Dolt backend)".to_string(),
            });
        }

        std::fs::write(&temp_jsonl, clean_lines.join("\n") + "\n")
            .map_err(|e| format!("Failed to write cleaned JSONL: {}", e))?;
    }

    // Step 3: bd import -i <cleaned_jsonl>
    let import_output = new_command(&binary)
        .args(&["import", "-i", &temp_jsonl.to_string_lossy()])
        .current_dir(&working_dir)
        .env("PATH", get_extended_path())
        .env("BEADS_PATH", &working_dir)
        .output()
        .map_err(|e| format!("Failed to run bd import: {}", e))?;

    // Clean up temp file
    std::fs::remove_file(&temp_jsonl).ok();

    if !import_output.status.success() {
        let stderr = String::from_utf8_lossy(&import_output.stderr);
        log_error!("[bd_migrate] Import failed: {}", stderr.trim());
        // Clean up failed migration so the modal will reappear
        if dolt_dir.exists() {
            std::fs::remove_dir_all(&dolt_dir).ok();
        }
        if beads_dir.join("dolt-access.lock").exists() {
            std::fs::remove_file(beads_dir.join("dolt-access.lock")).ok();
        }
        return Err(format!("Import failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&import_output.stdout);
    log_info!("[bd_migrate] Import successful: {}", stdout.trim());

    // Step 4: Restore labels (bd import doesn't preserve them)
    // Re-read JSONL to find issues with labels and apply them via bd update
    let mut labels_restored = 0u32;
    for line in jsonl_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if v.get("status").and_then(|s| s.as_str()) == Some("tombstone") {
                continue;
            }
            let labels: Vec<String> = v
                .get("labels")
                .and_then(|l| l.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            if labels.is_empty() {
                continue;
            }

            let issue_id = match v.get("id").and_then(|i| i.as_str()) {
                Some(id) => id,
                None => continue,
            };

            // bd update <id> --set-labels label1 --set-labels label2
            let mut args = vec!["update".to_string(), issue_id.to_string()];
            for label in &labels {
                args.push("--set-labels".to_string());
                args.push(label.clone());
            }

            let label_output = new_command(&binary)
                .args(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .current_dir(&working_dir)
                .env("PATH", get_extended_path())
                .env("BEADS_PATH", &working_dir)
                .output();

            match label_output {
                Ok(o) if o.status.success() => {
                    labels_restored += 1;
                }
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    log_info!("[bd_migrate] Failed to restore labels for {}: {}", issue_id, stderr.trim());
                }
                Err(e) => {
                    log_info!("[bd_migrate] Failed to run bd update for {}: {}", issue_id, e);
                }
            }
        }
    }

    if labels_restored > 0 {
        log_info!("[bd_migrate] Restored labels for {} issues", labels_restored);
    }

    // Step 5: Restore dependencies/relations (bd import doesn't preserve them)
    let mut deps_restored = 0u32;
    for line in jsonl_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if v.get("status").and_then(|s| s.as_str()) == Some("tombstone") { continue; }
            let dependencies = match v.get("dependencies").and_then(|d| d.as_array()) {
                Some(deps) if !deps.is_empty() => deps,
                _ => continue,
            };

            for dep in dependencies {
                let dep_obj = match dep.as_object() {
                    Some(o) => o,
                    None => continue,
                };

                let issue_id = match dep_obj.get("issue_id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => continue,
                };
                let depends_on_id = match dep_obj.get("depends_on_id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => continue,
                };
                let dep_type = dep_obj.get("type").and_then(|v| v.as_str()).unwrap_or("blocks").to_string();

                // Re-prefix if needed
                let issue_id = reprefix_id(&issue_id, &prefix, &prefix_counts);
                let depends_on_id = reprefix_id(&depends_on_id, &prefix, &prefix_counts);

                // bd dep add <issue_id> <depends_on_id> --type <type>
                let dep_output = new_command(&binary)
                    .args(&["dep", "add", &issue_id, &depends_on_id, "--type", &dep_type])
                    .current_dir(&working_dir)
                    .env("PATH", get_extended_path())
                    .env("BEADS_PATH", &working_dir)
                    .output();

                match dep_output {
                    Ok(o) if o.status.success() => { deps_restored += 1; }
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        log_info!("[bd_migrate] Failed to restore dep {} -> {}: {}", issue_id, depends_on_id, stderr.trim());
                    }
                    Err(e) => {
                        log_info!("[bd_migrate] Failed to run bd dep add: {}", e);
                    }
                }
            }
        }
    }

    if deps_restored > 0 {
        log_info!("[bd_migrate] Restored {} dependencies/relations", deps_restored);
    }

    // Step 6: Restore comments from SQLite backup (if available)
    // bd import doesn't preserve comments, and JSONL only has empty bodies.
    // Look for a .db.backup file with a comments table.
    let mut comments_restored = 0u32;
    let sqlite_backup = {
        let mut found: Option<std::path::PathBuf> = None;
        if let Ok(entries) = std::fs::read_dir(&beads_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".db.backup") {
                    found = Some(entry.path());
                    break;
                }
            }
        }
        found
    };

    if let Some(backup_path) = sqlite_backup {
        log_info!("[bd_migrate] Found SQLite backup: {:?}, restoring comments", backup_path);
        // Use sqlite3 CLI to extract comments as JSON
        let sqlite_output = std::process::Command::new("sqlite3")
            .args(&[
                backup_path.to_string_lossy().as_ref(),
                "-json",
                "SELECT issue_id, author, text FROM comments WHERE text IS NOT NULL AND text != '' ORDER BY created_at ASC",
            ])
            .output();

        if let Ok(output) = sqlite_output {
            if output.status.success() {
                let json_str = String::from_utf8_lossy(&output.stdout);
                if let Ok(rows) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
                    for row in &rows {
                        let issue_id = match row.get("issue_id").and_then(|v| v.as_str()) {
                            Some(id) => id.to_string(),
                            None => continue,
                        };
                        let author = row.get("author").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let text = match row.get("text").and_then(|v| v.as_str()) {
                            Some(t) if !t.is_empty() => t,
                            _ => continue,
                        };

                        // Re-prefix if needed
                        let issue_id = reprefix_id(&issue_id, &prefix, &prefix_counts);

                        // Write comment to temp file to handle multiline text
                        let comment_file = beads_dir.join("_migrate_comment.txt");
                        if std::fs::write(&comment_file, text).is_err() {
                            continue;
                        }

                        let comment_output = new_command(&binary)
                            .args(&["comments", "add", &issue_id, "-f", &comment_file.to_string_lossy(), "--author", author])
                            .current_dir(&working_dir)
                            .env("PATH", get_extended_path())
                            .env("BEADS_PATH", &working_dir)
                            .output();

                        match comment_output {
                            Ok(o) if o.status.success() => { comments_restored += 1; }
                            Ok(o) => {
                                let stderr = String::from_utf8_lossy(&o.stderr);
                                log_info!("[bd_migrate] Failed to restore comment for {}: {}", issue_id, stderr.trim());
                            }
                            Err(e) => {
                                log_info!("[bd_migrate] Failed to run bd comments add: {}", e);
                            }
                        }
                        std::fs::remove_file(&comment_file).ok();
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log_info!("[bd_migrate] sqlite3 query failed: {}", stderr.trim());
            }
        }

        if comments_restored > 0 {
            log_info!("[bd_migrate] Restored {} comments from SQLite backup", comments_restored);
        }
    }

    Ok(MigrateResult {
        success: true,
        message: format!(
            "Migration to Dolt completed (via init+import). {} Labels: {}. Deps: {}. Comments: {}.",
            stdout.trim(),
            labels_restored,
            deps_restored,
            comments_restored,
        ),
    })
}



#[cfg(test)]
mod tests {
    use super::*;

    // ---- reprefix_id (#3) -------------------------------------------------------

    #[test]
    fn reprefix_id_changes_prefix_when_old_exists_in_counts() {
        let mut counts = std::collections::HashMap::new();
        counts.insert("old-prefix".to_string(), 5);

        let result = reprefix_id("old-prefix-abc", "new-prefix", &counts);
        assert_eq!(result, "new-prefix-abc");
    }

    #[test]
    fn reprefix_id_keeps_id_when_old_prefix_not_in_counts() {
        let mut counts = std::collections::HashMap::new();
        counts.insert("other-prefix".to_string(), 5);

        let result = reprefix_id("old-prefix-abc", "new-prefix", &counts);
        assert_eq!(result, "old-prefix-abc");
    }

    #[test]
    fn reprefix_id_keeps_id_when_already_target_prefix() {
        let mut counts = std::collections::HashMap::new();
        counts.insert("new-prefix".to_string(), 5);

        let result = reprefix_id("new-prefix-abc", "new-prefix", &counts);
        assert_eq!(result, "new-prefix-abc");
    }

    #[test]
    fn reprefix_id_keeps_id_without_dash() {
        let counts = std::collections::HashMap::new();

        let result = reprefix_id("nodash", "new-prefix", &counts);
        assert_eq!(result, "nodash");
    }

    #[test]
    fn reprefix_id_uses_last_dash() {
        let mut counts = std::collections::HashMap::new();
        counts.insert("proj-sub".to_string(), 1);

        let result = reprefix_id("proj-sub-abc", "new-prefix", &counts);
        assert_eq!(result, "new-prefix-abc");
    }


}
