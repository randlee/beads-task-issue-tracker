use crate::backend;
use btit_beads::backend::BeadsBackend;
use btit_beads::issues::transform_issue;
use crate::migration::{cli_of, sync_bd_database};
use btit_types::{BdRawIssue, Issue, ListQuery, ProjectRef};
use std::fs;
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::sync::LazyLock;
use std::sync::Mutex;

// Filesystem mtime tracking for change detection (per-project)
pub(crate) static LAST_KNOWN_MTIME: LazyLock<Mutex<HashMap<String, std::time::SystemTime>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ============================================================================
// Batched Poll Data
// ============================================================================

/// All data needed for a single poll cycle, fetched in one IPC call.
#[derive(Debug, Serialize)]
pub struct PollData {
    #[serde(rename = "openIssues")]
    pub open_issues: Vec<Issue>,
    #[serde(rename = "closedIssues")]
    pub closed_issues: Vec<Issue>,
    #[serde(rename = "readyIssues")]
    pub ready_issues: Vec<Issue>,
}

/// Batched poll: sync once, then fetch all issues + ready in 2 commands (was 3).
/// Replaces 3 separate IPC calls (bd_list + bd_list(closed) + bd_ready) with one.
#[tauri::command]
pub(crate) async fn bd_poll_data(cwd: Option<String>) -> Result<PollData, String> {
    log_info!("[bd_poll_data] Batched poll starting");

    let cwd_ref = cwd.as_deref();

    // Single sync for the entire poll cycle
    sync_bd_database(cwd_ref);

    // Fetch issues: single --all call for bd >= 0.55, fallback to 2 calls for older versions
    let project = ProjectRef::local(cwd.clone());
    let all = ListQuery { include_all: Some(true), ..ListQuery::default() };
    let backend = backend::current();
    let raw_all = backend.list(&project, &all).map_err(|e| e.to_string())?;
    let (raw_open, raw_closed): (Vec<_>, Vec<_>) = raw_all.into_iter()
        .partition(|issue: &BdRawIssue| issue.status != "closed");

    // Fetch ready issues
    let raw_ready = backend.ready(&project).map_err(|e| e.to_string())?;

    log_info!("[bd_poll_data] Batched poll done: {} open, {} closed, {} ready",
        raw_open.len(), raw_closed.len(), raw_ready.len());

    // Update mtime AFTER our commands ran, so the next bd_check_changed
    // only detects EXTERNAL changes (not our own poll's side effects)
    {
        let working_dir = cwd_ref
            .map(String::from)
            .or_else(|| env::var("BEADS_PATH").ok())
            .unwrap_or_else(|| {
            env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });
        let beads_dir = std::path::Path::new(&working_dir).join(".beads");

        if let Some(mtime) = get_beads_mtime(&beads_dir) {
            let mut map = LAST_KNOWN_MTIME.lock().unwrap();
            map.insert(working_dir, mtime);
        }
    }

    Ok(PollData {
        open_issues: raw_open.into_iter().map(transform_issue).collect(),
        closed_issues: raw_closed.into_iter().map(transform_issue).collect(),
        ready_issues: raw_ready.into_iter().map(transform_issue).collect(),
    })
}

/// Get the latest mtime across all beads database files.
/// - Dolt backend (bd >= 0.50.0): checks .beads/ dir, .beads/.dolt/ (legacy) or
///   .beads/dolt/<name>/.dolt/ (bd 0.52+ nested layout), and manifest files
/// - SQLite backend: checks beads.db, beads.db-wal, and optionally issues.jsonl
pub(crate) fn get_beads_mtime(beads_dir: &std::path::Path) -> Option<std::time::SystemTime> {
    get_beads_mtime_with(backend::current().as_ref(), beads_dir)
}

/// Body of [`get_beads_mtime`] over an injected backend; the project is `beads_dir`'s parent.
pub(crate) fn get_beads_mtime_with(
    be: &dyn BeadsBackend,
    beads_dir: &std::path::Path,
) -> Option<std::time::SystemTime> {
    let project = ProjectRef::local(beads_dir.parent().map(|p| p.to_string_lossy().into_owned()));
    let uses_dolt = be.project_uses_dolt(&project);
    let uses_jsonl = !uses_dolt
        && cli_of(be, "mtime").map(|c| c.capabilities().uses_jsonl_files).unwrap_or(false);
    get_beads_mtime_for(uses_dolt, uses_jsonl, beads_dir)
}

/// Pure core of [`get_beads_mtime_with`]: computes the latest mtime for a beads
/// project given whether it uses Dolt and whether SQLite reads should also
/// check `issues.jsonl`. Performs no backend or CLI calls.
pub(crate) fn get_beads_mtime_for(
    uses_dolt: bool,
    uses_jsonl: bool,
    beads_dir: &std::path::Path,
) -> Option<std::time::SystemTime> {
    if uses_dolt {
        // Dolt backend: check directory mtimes and manifest files
        let mut times: Vec<std::time::SystemTime> = Vec::new();

        // .beads/ dir mtime
        if let Ok(m) = fs::metadata(beads_dir) {
            if let Ok(t) = m.modified() { times.push(t); }
        }

        // Collect all .dolt/ directories to check:
        // - Legacy layout: .beads/.dolt/
        // - Nested layout (bd 0.52+): .beads/dolt/<name>/.dolt/
        let mut dolt_dirs: Vec<std::path::PathBuf> = Vec::new();

        let legacy_dolt = beads_dir.join(".dolt");
        if legacy_dolt.is_dir() {
            dolt_dirs.push(legacy_dolt);
        }

        let nested_dolt = beads_dir.join("dolt");
        if nested_dolt.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&nested_dolt) {
                for entry in entries.flatten() {
                    let sub_dolt = entry.path().join(".dolt");
                    if sub_dolt.is_dir() {
                        dolt_dirs.push(sub_dolt);
                    }
                }
            }
        }

        // Check mtime of each .dolt/ dir and its manifest files
        for dolt_dir in &dolt_dirs {
            if let Ok(m) = fs::metadata(dolt_dir) {
                if let Ok(t) = m.modified() { times.push(t); }
            }
            for name in &["manifest", "noms/manifest"] {
                let p = dolt_dir.join(name);
                if let Ok(m) = fs::metadata(&p) {
                    if let Ok(t) = m.modified() { times.push(t); }
                }
            }
        }

        // Also check issues.jsonl (Dolt exports to it for git sync)
        let jsonl_path = beads_dir.join("issues.jsonl");
        if let Ok(m) = fs::metadata(&jsonl_path) {
            if let Ok(t) = m.modified() { times.push(t); }
        }

        times.into_iter().max()
    } else {
        // SQLite backend: check db, WAL, and optionally JSONL
        let mut paths = vec![
            beads_dir.join("beads.db"),
            beads_dir.join("beads.db-wal"),
        ];
        if uses_jsonl {
            paths.push(beads_dir.join("issues.jsonl"));
        }
        paths.iter()
            .filter_map(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
            .max()
    }
}

/// Check if the beads database has changed since last check (via filesystem mtime).
/// Returns true if changes detected or if this is the first check.
/// This is extremely cheap — just a few stat() calls, no bd process spawns.
#[tauri::command]
pub(crate) async fn bd_check_changed(cwd: Option<String>) -> Result<bool, String> {
    let working_dir = cwd
        .or_else(|| env::var("BEADS_PATH").ok())
        .unwrap_or_else(|| {
            env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });

    let beads_dir = std::path::Path::new(&working_dir).join(".beads");
    let current_mtime = get_beads_mtime(&beads_dir);

    let mut map = LAST_KNOWN_MTIME.lock().unwrap();
    let previous = map.get(&working_dir).copied();

    match (current_mtime, previous) {
        (Some(current), Some(prev)) => {
            if current != prev {
                log_info!("[bd_check_changed] mtime changed — data may have been modified");
                map.insert(working_dir, current);
                Ok(true)
            } else {
                log_debug!("[bd_check_changed] mtime unchanged — no changes");
                Ok(false)
            }
        }
        (Some(current), None) => {
            // First check — store mtime, report changed so initial load happens
            map.insert(working_dir, current);
            Ok(true)
        }
        (None, _) => {
            // No database file found
            log_warn!("[bd_check_changed] No beads database found in {}", working_dir);
            Ok(true) // Report changed to let caller handle missing db
        }
    }
}

/// Reset the cached mtime for a specific project (or all projects).
/// Called from the frontend when switching projects to force a fresh poll.
#[tauri::command]
pub(crate) async fn bd_reset_mtime(cwd: Option<String>) -> Result<(), String> {
    let mut map = LAST_KNOWN_MTIME.lock().unwrap();
    if let Some(path) = cwd {
        log_info!("[bd_reset_mtime] Resetting mtime for: {}", path);
        map.remove(&path);
    } else {
        log_info!("[bd_reset_mtime] Resetting all cached mtimes");
        map.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_beads_mtime_for_returns_none_for_empty_dir() {
        // A path that does not exist on disk: no `.beads` directory, no
        // database or manifest files, so both backend kinds must return
        // `None` without touching the filesystem in a way that could succeed.
        let temp_dir = std::env::temp_dir().join(format!("beads_test_mtime_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));

        assert_eq!(get_beads_mtime_for(false, false, &temp_dir), None);
        assert_eq!(get_beads_mtime_for(true, false, &temp_dir), None);
    }

    // ---- get_beads_mtime_with over a RecordingInvoker-backed BdCli (b-8) ----------

    use crate::test_backend::{bd_invoker, recording_bd, FakeBackend, TempProject, BD_0_49_6, BD_1_0_4};
    use std::time::{Duration, SystemTime};

    /// Sets `path`'s mtime to `t` and returns the mtime the filesystem stored.
    fn set_mtime(path: &std::path::Path, t: SystemTime) -> Result<SystemTime, String> {
        let file = fs::File::options().write(true).open(path).map_err(|e| e.to_string())?;
        file.set_modified(t).map_err(|e| e.to_string())?;
        fs::metadata(path).and_then(|m| m.modified()).map_err(|e| e.to_string())
    }

    #[test]
    fn mtime_sqlite_counts_jsonl_only_when_the_backend_uses_jsonl() -> Result<(), String> {
        let project = TempProject::sqlite("mtime_sqlite")?;
        project.write(".beads/issues.jsonl", "{}\n")?;
        let base = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let db_time = set_mtime(&project.beads_dir().join("beads.db"), base)?;
        let jsonl_time = set_mtime(
            &project.beads_dir().join("issues.jsonl"),
            base + Duration::from_secs(60),
        )?;

        let (bd049, inv049) = recording_bd(bd_invoker(BD_0_49_6));
        assert_eq!(get_beads_mtime_with(&bd049, &project.beads_dir()), Some(jsonl_time));
        let (bd1, inv1) = recording_bd(bd_invoker(BD_1_0_4));
        assert_eq!(get_beads_mtime_with(&bd1, &project.beads_dir()), Some(db_time));
        let no_cli = FakeBackend { uses_dolt: false, cli: None };
        assert_eq!(get_beads_mtime_with(&no_cli, &project.beads_dir()), Some(db_time));

        assert!(inv049.calls().is_empty() && inv1.calls().is_empty());
        assert_eq!(inv049.probe_calls() + inv1.probe_calls(), 0);
        Ok(())
    }

    #[test]
    fn mtime_dolt_project_reads_the_dolt_layout() -> Result<(), String> {
        let project = TempProject::dolt("mtime_dolt")?;
        let (bd1, inv) = recording_bd(bd_invoker(BD_1_0_4));
        // No beads.db exists, so a SQLite reading would be None.
        assert!(get_beads_mtime_with(&bd1, &project.beads_dir()).is_some());
        let sqlite_view = FakeBackend { uses_dolt: false, cli: None };
        assert_eq!(get_beads_mtime_with(&sqlite_view, &project.beads_dir()), None);
        assert!(inv.calls().is_empty());
        Ok(())
    }
}
