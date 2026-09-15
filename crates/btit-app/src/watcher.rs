use crate::backend;
use btit_types::ProjectRef;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::Emitter;

// ============================================================================
// File Watcher (debounced native fs watcher via notify crate)
// ============================================================================

pub(crate) struct WatcherState {
    debouncer: Option<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>>,
    watched_path: Option<String>,
}

impl Default for WatcherState {
    fn default() -> Self {
        Self {
            debouncer: None,
            watched_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BeadsChangedPayload {
    path: String,
}

// ============================================================================
// File Watcher Commands
// ============================================================================

#[tauri::command]
pub(crate) fn start_watching(
    path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<WatcherState>>,
) -> Result<(), String> {
    let be = backend::current();
    let mut watcher_state = state.lock().map_err(|e| format!("Lock error: {}", e))?;

    // Stop existing watcher if any
    if watcher_state.debouncer.is_some() {
        log::info!("[watcher] Stopping previous watcher for: {:?}", watcher_state.watched_path);
        watcher_state.debouncer = None;
        watcher_state.watched_path = None;
    }

    let beads_dir = PathBuf::from(&path).join(".beads");
    if !beads_dir.exists() {
        return Err(format!(".beads directory not found at: {}", beads_dir.display()));
    }

    let project_path = path.clone();
    let app_handle = app.clone();

    let mut debouncer = new_debouncer(
        Duration::from_millis(1000),
        move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
            match res {
                Ok(events) => {
                    // Filter: only emit if we have actual data-change events
                    let has_data_events = events.iter().any(|e| {
                        matches!(e.kind, DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous)
                    });
                    if has_data_events {
                        log::info!("[watcher] Change detected in .beads/ ({} events)", events.len());
                        let _ = app_handle.emit(
                            "beads-changed",
                            BeadsChangedPayload { path: project_path.clone() },
                        );
                    }
                }
                Err(e) => {
                    log::error!("[watcher] Error: {:?}", e);
                }
            }
        },
    ).map_err(|e| format!("Failed to create watcher: {}", e))?;

    // Watch .beads/ directory
    // Dolt backend: recursive (changes happen in .dolt/ subdirectories)
    // SQLite backend: non-recursive (all target files are at root level)
    let watch_mode = if be.project_uses_dolt(&ProjectRef::local(Some(path.clone()))) {
        notify::RecursiveMode::Recursive
    } else {
        notify::RecursiveMode::NonRecursive
    };
    debouncer.watcher().watch(
        beads_dir.as_path(),
        watch_mode,
    ).map_err(|e| format!("Failed to watch .beads/: {}", e))?;

    log::info!("[watcher] Started watching: {}", beads_dir.display());
    watcher_state.debouncer = Some(debouncer);
    watcher_state.watched_path = Some(path);

    Ok(())
}

#[tauri::command]
pub(crate) fn stop_watching(
    state: tauri::State<'_, Mutex<WatcherState>>,
) -> Result<(), String> {
    let mut watcher_state = state.lock().map_err(|e| format!("Lock error: {}", e))?;

    if watcher_state.debouncer.is_some() {
        log::info!("[watcher] Stopped watching: {:?}", watcher_state.watched_path);
        watcher_state.debouncer = None;
        watcher_state.watched_path = None;
    }

    Ok(())
}

#[derive(Debug, Serialize)]
pub(crate) struct WatcherStatusInfo {
    active: bool,
    #[serde(rename = "watchedPath")]
    watched_path: Option<String>,
}

#[tauri::command]
pub(crate) fn get_watcher_status(
    state: tauri::State<'_, Mutex<WatcherState>>,
) -> Result<WatcherStatusInfo, String> {
    let watcher_state = state.lock().map_err(|e| format!("Lock error: {}", e))?;

    Ok(WatcherStatusInfo {
        active: watcher_state.debouncer.is_some(),
        watched_path: watcher_state.watched_path.clone(),
    })
}

