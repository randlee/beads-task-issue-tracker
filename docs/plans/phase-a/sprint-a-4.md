---
id: a-4
title: btit adopts the sc-observability-log bridge
status: planned
branch: feature/sprint-a-4-btit-adoption
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-a-4-btit-adoption
target: develop
recommended_model: standard (bounded integration; UI renderer is pure logic)
dependency_relations:
  - prerequisite: a-1
    dependent: a-4
    relation: must_follow
    rationale: "consumes the a-1 init/BridgeOptions/LogGuard/LevelFilter API and frozen runtime graph; PR-completion trigger: a-1 PR merged before the a-4 branch is created"
  - prerequisite: none
    parallel_pair: [a-4, a-2]
    relation: parallel_safe
    rationale: "non-intersecting: a-4 owns src-tauri/, app/, tests/, CLAUDE.md, codebase-map, CHANGELOG; a-2 owns crates/ sources only; runtime graph frozen by a-1"
  - prerequisite: none
    parallel_pair: [a-4, a-3]
    relation: parallel_safe
    rationale: "non-intersecting: same ownership split as a-2"
  - prerequisite: a-4
    dependent: a-5
    relation: must_follow
    rationale: "the review covers the crates as adopted by btit; a-4 PR merges before a-5 development starts"
---

# Sprint a-4 — btit adopts the sc-observability-log bridge

## Recommended Agent / Model

Recommended model: standard (bounded integration; UI renderer is pure logic).
Recommended agent: not set — the btit developer pool is pending the `arch-ctm` decision on PR #38.
Planning advice; team-lead assigns from the active pool.

## Goal

- Replace `tauri-plugin-log` in btit with `sc_observability_log::init`, without changing any of the 200 Rust log call sites or the 38 `logFrontend` call sites.
- Logs become sc-observability JSONL.
- The in-app debug panel and the log commands (read, export, clear, path) keep working on the new file.

## Hard Dependencies

- a-1 PR merged to `develop`: `init`, `BridgeOptions`, `LogGuard`, the `LevelFilter` re-export, and the frozen runtime dependency graph (`crates/runtime-deps.txt`). The a-4 branch is created from `develop` after that merge, as a single PR on `develop` with no stack: GitHub stacks are strictly linear, so a-1 cannot have both a-2 and a-4 as children.
- a-2 and a-3 are **not** prerequisites (`parallel_safe`).

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA;
merge parent → child before every dev/fix round. PR-completion trigger: parent
PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- a-1 → a-4 — `must_follow` (a-4 follows a-1): consumes the a-1 init/BridgeOptions/LogGuard/LevelFilter API and frozen runtime graph; PR-completion trigger: a-1 PR merged before the a-4 branch is created
- a-4 ↔ a-2 — `parallel_safe`: non-intersecting: a-4 owns src-tauri/, app/, tests/, CLAUDE.md, codebase-map, CHANGELOG; a-2 owns crates/ sources only; runtime graph frozen by a-1
- a-4 ↔ a-3 — `parallel_safe`: non-intersecting: same ownership split as a-2
- a-4 → a-5 — `must_follow` (a-5 follows a-4): the review covers the crates as adopted by btit; a-4 PR merges before a-5 development starts

Stack: `none (single PR on develop, created after the a-1 PR merges)`.

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
   - `clear_logs` flushes the guard, truncates the active file (the sink opens it in append mode, `../sc-observability/crates/sc-observability/src/sinks.rs:321-325`, `JsonlFileSink::write` reopens with `.append(true)` on every write), and deletes the rotated `*.log.jsonl.N` files.
   - `get_log_path_string` returns the new path.
6. **Frontend log command.** `log_frontend(level, message)` keeps its signature and logs with `target: "frontend"`. A leading `[tag]` in the message becomes `action` through the bridge. Frontend records are deliberately categorized by `target` (replacing today's literal `[frontend]` message prefix), which is why the debug panel's existing `\[frontend\]` highlight keeps matching the rendered `[<target>]` segment.
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
9. Refactor-review item A1 is closed by design: JSONL records carry the Rust `log` target as `LogEvent.target` (for example `app_lib.cli`, taken from `module_path!()` via `log::Record::target()`), and the debug panel renders that field. No log line depends on the literal `[app_lib]` prefix. Evidence is in the PR description.
10. The a-4 diff touches no file under `crates/`.

## Required Validation

- `pnpm test`
- `npx vue-tsc --noEmit`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --all-targets`
- `pnpm tauri:build` (manual verification per Required Work)
