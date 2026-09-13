---
id: a-4
title: btit adopts the sc-observability-log bridge
status: planned
branch: feature/sprint-a-4-btit-adoption
target: develop
recommended_model: standard (bounded integration; UI renderer is pure logic)
dependency_relations:
  - prerequisite: a-3
    dependent: a-4
    relation: must_follow
    rationale: a-4 adds path dependencies on the complete crate pair to src-tauri/Cargo.toml and pins them in src-tauri/Cargo.lock; the crate API must be final. Merge a-3 forward before every dev/fix round; a-3 PR merges first.
---

# Sprint a-4 — btit adopts the sc-observability-log bridge

## Goal

- Replace `tauri-plugin-log` in btit with `sc_observability_log::init`, without changing any of the 200 Rust log call sites or the 38 `logFrontend` call sites.
- Logs become sc-observability JSONL.
- The in-app debug panel and the log commands (read, export, clear, path) keep working on the new file.

## Hard Dependencies

- a-3 merged: complete `sc-observability-log` API, including the a-1 `init`, `BridgeOptions`, `LogGuard` and the `LevelFilter` re-export.

## Exact Targets

- `src-tauri/Cargo.toml`: remove `tauri-plugin-log`; add `sc-observability-log = { path = "../crates/sc-observability-log" }`
- `src-tauri/Cargo.lock`
- `src-tauri/src/lib.rs`: logger setup (currently lines 43-53), guard lifetime, `.build()` + `.run(|app, event| ..)`
- `src-tauri/src/logging.rs`: `get_log_path`, `clear_logs`, `export_logs`, `read_logs`, `get_log_path_string`, `log_frontend` (currently lines 47-170)
- `app/utils/log-format.ts` (new)
- `tests/utils/log-format.test.ts` (new)
- `app/components/layout/DebugPanel.vue`: render through `formatLogLines` before the existing colorizer (currently lines 82-118)
- `CLAUDE.md`: `### Logging` → log file line (currently line 58)
- `.claude/codebase-map.md`: logs row (currently line 391)
- `CHANGELOG.md`: `[Unreleased]` entry

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Dependencies.** `tauri-plugin-log` is removed from `src-tauri/Cargo.toml` and `Cargo.lock`. `sc-observability-log` is the only new direct dependency.
2. **Logger initialization.** Logging is initialized in `setup` as shown in the code samples:
   - service `beads-task-issue-tracker`
   - `log_root = app.path().app_log_dir()`
   - level `Debug` in debug builds and `Info` in release, the same as today
   - console sink enabled, replacing today's `Stdout` target
   - `parse_bracket_action = true`, `default_action = "log"`
3. **Guard lifetime.** The `LogGuard` lives in managed state and is flushed and dropped on `RunEvent::Exit`.
4. **Log file path.** The active log file is `<app_log_dir>/logs/beads-task-issue-tracker.log.jsonl`. `get_log_path()` returns that path from a `OnceLock<PathBuf>` set during setup, and the per-OS `cfg` blocks in `logging.rs` are deleted.
5. **Log commands.** Names and signatures are unchanged:
   - `read_logs(tail_lines)` returns the last N JSONL lines.
   - `export_logs` copies the active file under the same export-folder rules, keeping the `.log.jsonl` name.
   - `clear_logs` flushes the guard, truncates the active file (the sink opens it in append mode, `sc-observability/src/sinks.rs:123-126`), and deletes the rotated `*.log.jsonl.N` files.
   - `get_log_path_string` returns the new path.
6. **Frontend log command.** `log_frontend(level, message)` keeps its signature and logs with `target: "frontend"`. A leading `[tag]` in the message becomes `action` through the bridge.
7. **Log line formatting.** `app/utils/log-format.ts` exports `formatLogLine` and `formatLogLines` as shown in the code samples. Unit tests cover valid, partial, non-JSON and empty lines.
8. **Debug panel.** `DebugPanel.vue` shows `formatLogLines(await readLogs(300))`. Its existing level and tag colorization regexes still apply, because rendered lines contain `[LEVEL]`, `[target]` and `[action]`.
9. **Docs.** `CLAUDE.md`, `.claude/codebase-map.md` and `CHANGELOG.md` document the new log location and JSONL format. The old `beads.log` is left on disk and neither read nor deleted.

## Required Work

- Keep the `log_info!`, `log_warn!`, `log_error!` and `log_debug!` macros, `LOGGING_ENABLED`, `VERBOSE_LOGGING` and all 9 logging commands.
- Keep the `=== Beads Task-Issue Tracker starting ===` line and every `[startup]` log. They must run after `init`.
- Rendered line format: `[<timestamp>][<LEVEL>][<target>] [<action>] <message>`. When `action` equals the configured default action `log`, omit the `[<action>] ` segment.
- Manual verification:
  1. Run `pnpm tauri:build` and launch the built app.
  2. Confirm that the JSONL file exists and contains startup records with `target` `app_lib` / `app_lib.cli`.
  3. Confirm that the debug panel shows colorized rendered lines.
  4. Confirm that `clear_logs` empties the panel and new records then appear.

  Record the file path and a 3-record excerpt in the PR description.

## Explicit Code Samples

```rust
// src-tauri/src/lib.rs (setup)
use sc_observability_log::{init, ActionName, BridgeOptions, LevelFilter, LogGuard, LoggerConfig, ServiceName};

struct LogGuardState(std::sync::Mutex<Option<LogGuard>>);

.setup(|app| {
    let log_root = app.path().app_log_dir()?;
    let service = ServiceName::new("beads-task-issue-tracker")?;
    let mut config = LoggerConfig::default_for(service, log_root.clone());
    config.level = if cfg!(debug_assertions) { LevelFilter::Debug } else { LevelFilter::Info };
    config.enable_console_sink = true;
    let guard = init(config, BridgeOptions {
        default_action: ActionName::new("log")?,
        parse_bracket_action: true,
        max_level: if cfg!(debug_assertions) { log::LevelFilter::Debug } else { log::LevelFilter::Info },
    })?;
    logging::set_log_path(log_root.join("logs").join("beads-task-issue-tracker.log.jsonl"));
    app.manage(LogGuardState(std::sync::Mutex::new(Some(guard))));
    log::info!("=== Beads Task-Issue Tracker starting ===");
    // ... existing startup logging unchanged
    Ok(())
})
// ...
.build(tauri::generate_context!())
.expect("error while building tauri application")
.run(|app, event| {
    if let tauri::RunEvent::Exit = event {
        if let Some(guard) = app.state::<LogGuardState>().0.lock().unwrap().take() {
            let _ = guard.flush();
        } // guard dropped here: shutdown
    }
});

// src-tauri/src/logging.rs
pub(crate) fn set_log_path(path: std::path::PathBuf);   // OnceLock; called once in setup
pub(crate) fn get_log_path() -> std::path::PathBuf;     // active JSONL path

#[tauri::command]
pub(crate) async fn log_frontend(level: String, message: String) {
    let level = match level.as_str() { "error" => log::Level::Error, "warn" => log::Level::Warn, _ => log::Level::Info };
    log::log!(target: "frontend", level, "{}", message);
}
```

```ts
// app/utils/log-format.ts
export interface LogRecordView { timestamp: string; level: string; target: string; action: string; message: string }
export function formatLogLine(line: string, defaultAction?: string): string  // non-JSON lines returned unchanged
export function formatLogLines(jsonl: string, defaultAction?: string): string
```

## This Sprint Does Not Close

- Converting btit call sites to structured fields, a-2 macros or `#[instrument]` (not in phase-a).
- Migrating, deleting or importing the old `beads.log`.
- Switching from the path dependency to the crates.io release (phase-b, after sc-observability publishes).
- Behavior issues B1–B13 in `docs/crate-split-refactor-issues.md`.

## Acceptance Criteria

1. `tauri-plugin-log` appears nowhere in `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` or `src-tauri/src`.
2. No Rust `log_*!`/`log::*!` call site and no `logFrontend(` call site changes. `git diff develop...HEAD` shows no changed lines matching those patterns, other than in `logging.rs::log_frontend` and `lib.rs` setup.
3. The built app writes `<app_log_dir>/logs/beads-task-issue-tracker.log.jsonl`. Startup records carry the expected `target`, and bracket-tagged records carry the tag as `action`. Evidence is in the PR description.
4. `read_logs`, `export_logs`, `clear_logs` and `get_log_path_string` work on the JSONL file. `clear_logs` truncates the active file and removes rotated files, and logging continues afterwards. Evidence is in the PR description.
5. `formatLogLine`/`formatLogLines` tests pass, and the debug panel shows rendered, colorized lines.
6. The per-OS `get_log_path` `cfg` blocks are gone, and `logging.rs` has no platform-gated imports (closes refactor-review item A2 for `logging.rs`).
7. The `LogGuard` is flushed on `RunEvent::Exit`. The last record logged before quit is present in the file.
8. All btit gates pass, including the Windows cross-check.

## Required Validation

- `pnpm test`
- `npx vue-tsc --noEmit`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --all-targets`
- `pnpm tauri:build` (manual verification per Required Work)
