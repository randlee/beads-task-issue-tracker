---
id: a-4
title: btit adopts the sc-observability-log bridge
status: planned
branch: feature/sprint-a-4-btit-adoption
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-a-4-btit-adoption
target: integrate/phase-a
recommended_model: standard (bounded integration; UI renderer is pure logic)
dependency_relations:
  - prerequisite: a-1
    dependent: a-4
    relation: must_follow
    rationale: "consumes the frozen a-1 public API (init, BridgeOptions, LogGuard, InitError, DropCause, re-exports) and runtime graph; PR-completion trigger: a-1 PR merged before the a-4 branch is created"
  - prerequisite: none
    parallel_pair: [a-4, a-2]
    relation: parallel_safe
    rationale: "non-intersecting: a-4 owns src-tauri/, app/, tests/, CLAUDE.md, codebase-map, CHANGELOG and its own sprint doc; a-2 owns crates/ only; a-1 API frozen by tests/api_freeze.rs and runtime graph frozen by crates/runtime-deps.txt"
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
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Replace `tauri-plugin-log` in btit with `sc_observability_log::init`, without changing any of the 200 Rust log call-site lines or the 38 `logFrontend(` call sites.
- Logs become sc-observability JSONL.
- The in-app debug panel and the log commands (read, export, clear, path) keep working on the new file.

## Hard Dependencies

- a-1 PR merged to `integrate/phase-a`: `init`, `BridgeOptions`, `LogGuard`, `InitError`, `DropCause`, the `LevelFilter` re-export, the frozen API (`crates/sc-observability-log/tests/api_freeze.rs`) and the frozen runtime dependency graph (`crates/runtime-deps.txt`). The a-4 branch is created from `develop` after that merge, as a single PR on `develop` with no stack: GitHub stacks are strictly linear, so a-1 cannot have both a-2 and a-4 as children.
- a-2 and a-3 are **not** prerequisites (`parallel_safe`).

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA;
merge parent → child before every dev/fix round. PR-completion trigger: parent
PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- a-1 → a-4 — `must_follow` (a-4 follows a-1): consumes the frozen a-1 public API and runtime graph; PR-completion trigger: a-1 PR merged before the a-4 branch is created.
- a-4 ↔ a-2 — `parallel_safe`: a-4 owns `src-tauri/`, `app/`, `tests/`, `CLAUDE.md`, `.claude/codebase-map.md`, `CHANGELOG.md` and its own sprint doc; a-2 owns `crates/` only; the a-1 API and runtime graph are frozen.
- a-4 ↔ a-3 — `parallel_safe`: same ownership split as a-2.
- a-4 → a-5 — `must_follow` (a-5 follows a-4): the review covers the crates as adopted by btit; a-4 PR merges before a-5 development starts.

Stack: `none (single PR on integrate/phase-a, created after the a-1 PR merges)`.

## Exact Targets

Line numbers are at the phase baseline `develop@8554294`.

- `src-tauri/Cargo.toml`: remove `tauri-plugin-log` (line 39); add `sc-observability-log = { path = "../crates/sc-observability-log" }`; change `log = "0.4"` (line 37) to `log = { version = "0.4", features = ["kv"] }`
- `src-tauri/Cargo.lock`
- `src-tauri/src/lib.rs`: `run()` — logger setup (plugin block lines 42-54 inside `.setup` at 35-85), `CLI_BINARY` lock (line 63), and `.run(tauri::generate_context!()).expect(..)` (lines 153-154) becomes `.build(..)` + `.run(|app, event| ..)`
- `src-tauri/src/logging.rs`: `get_log_path`, `clear_logs`, `export_logs`, `read_logs`, `get_log_path_string`, `log_frontend` (lines 47-170); new `install_logging`, `on_run_event`
- `app/utils/log-format.ts` (new)
- `tests/utils/log-format.test.ts` (new)
- `app/components/layout/DebugPanel.vue`: render through `formatLogLines` before the existing colorizer (`colorizedLogs` line 82, `readLogs(300)` line 118)
- `CLAUDE.md`: `### Logging` → frontend mechanism line (line 55) and log file line (line 58)
- `.claude/codebase-map.md`: logs row (line 391)
- `CHANGELOG.md`: `[Unreleased]` entry
- `docs/plans/phase-a/sprint-a-4.md` (`status:` frontmatter only)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Dependencies.** `tauri-plugin-log` is removed from `src-tauri/Cargo.toml` and `Cargo.lock`. `sc-observability-log` is the only new direct dependency; `log` gains the `kv` feature explicitly (Deliverable 7 uses it; no reliance on feature unification through the path dependency).
2. **Logger initialization.** `logging::install_logging(app)` is called first in `setup`, as shown in the code samples:
   - service `beads-task-issue-tracker`
   - `log_root = app.path().app_log_dir()`
   - `LoggerConfig.level` `Debug` in debug builds and `Info` in release, the same as today; this is the only level setting (a-1 derives the `log` max level from it)
   - console sink enabled, replacing today's `Stdout` target
   - `parse_bracket_action = true`, `default_action = "log"`
   - an `InitError` fails `setup` through `?`, the same behavior as today's `plugin(..)?`. Tauri 2.10.2 runs `setup` inside `App::run` on `RuntimeRunEvent::Ready`, not inside `Builder::build`, and turns a setup `Err` into `panic!("Failed to setup app: {e}")` (`tauri-2.10.2/src/app.rs:1297-1300`). That panic is inside Tauri, outside btit's no-panic scope (Deliverable 10).
3. **Guard lifetime and exit.** The `LogGuard` lives in `static LOG_GUARD: Mutex<Option<Arc<LogGuard>>>` in `logging.rs`. `LogGuard::flush` takes `&self` and `LogGuard::shutdown` takes `self` (a-1 API), and the guard is not `Clone`, so the `Arc` lets `clear_logs` flush without holding the lock (Deliverable 5). No code path holds `LOG_GUARD` across a flush or a shutdown; every lock is a take or an `Arc` clone followed by an immediate release, recovering a poisoned mutex with `PoisonError::into_inner`. On `RunEvent::Exit`, `logging::on_run_event` takes the `Arc` out of `LOG_GUARD` and logs one `warn` record with the per-`DropCause` counts when `dropped_events().total() > 0`. It then calls `Arc::try_unwrap`:
   - `Ok(guard)` (the normal case): `guard.shutdown(LOG_IO_TIMEOUT)`, exactly once.
   - `Err(shared)` (a `clear_logs` flush holds the only other clone at this instant): `shared.flush(LOG_IO_TIMEOUT)` once, so the last records reach the file. Shutdown cannot consume a shared guard; the clone's `Drop` runs a-1's drop-shutdown if the process outlives the concurrent flush.

   Either branch makes exactly one bounded call, so exit is bounded by 1×`LOG_IO_TIMEOUT` (2 s) and never waits for the lock behind a `clear_logs` flush. There is no reliance on `Drop` in the normal case. A `ShutdownError` or `FlushError` is written to stderr with `let _ = writeln!(std::io::stderr(), ..)` because the logger is gone (`eprintln!` panics when stderr is closed).
4. **Log file path.** The active log file is `<app_log_dir>/logs/beads-task-issue-tracker.log.jsonl`. `install_logging` stores `guard.active_log_path()` in `static LOG_PATH: OnceLock<PathBuf>` before wrapping the guard in `Arc`; `get_log_path()` returns it (or an empty `PathBuf` before setup), and no path is built by hand. The per-OS `cfg` blocks in `logging.rs` and the `use std::env;` import are deleted.
5. **Log commands.** Names and frontend signatures are unchanged:
   - `read_logs(tail_lines)` returns the last N JSONL lines, without panicking indexing.
   - `export_logs` copies the active file under the same export-folder rules, keeping the `.log.jsonl` name.
   - `clear_logs` clones the `Arc<LogGuard>` under the `LOG_GUARD` lock and releases the lock before calling `flush(LOG_IO_TIMEOUT)` on the clone, so an exit that arrives during the flush takes the guard at once and stays bounded by 1×`LOG_IO_TIMEOUT` (Deliverable 3). It then truncates the active file (the sink opens it in append mode on every write, `../sc-observability/crates/sc-observability/src/sinks.rs:321-325`) and deletes the rotated `*.log.jsonl.N` files. A `FlushError` is reported in the command's existing `Err(String)` and the file is not truncated.
   - When `LOG_GUARD` is `None` (before `setup` installs the logger, or after exit took it), `clear_logs` skips the flush. It truncates `get_log_path()` and deletes the rotated files when that path exists, and otherwise returns `Ok(())`, as today's `if log_path.exists()`.
   - `get_log_path_string` returns the new path.
6. **Frontend log command.** `log_frontend(level, message)` keeps its signature and logs with `target: "frontend"`. A leading `[tag]` in the message becomes `action` through the bridge. Frontend records are categorized by `target` (replacing today's literal `[frontend]` message prefix), so the debug panel's existing `\[frontend\]` highlight keeps matching the rendered `[<target>]` segment.
7. **Unknown frontend level (explicit decision).** `"error"`, `"warn"` and `"info"` map to their levels. Any other string still logs at `Info`, as today, but the record carries `frontend_level = <raw>` as a `log` kv field, so the fallback is visible in JSONL (compiled against `log` 0.4.34 while planning). The TypeScript wrapper already restricts the level to those three (`app/utils/bd-api.ts:680`).
8. **Log line formatting.** `app/utils/log-format.ts` exports `formatLogLine` and `formatLogLines` as shown in the code samples. Unit tests cover valid, partial, non-JSON and empty lines.
9. **Debug panel.** `DebugPanel.vue` shows `formatLogLines(await readLogs(300))`. Its existing level and tag colorization regexes still apply, because rendered lines contain `[LEVEL]`, `[target]` and `[action]`.
10. **No panics in touched code.** `src-tauri/src/logging.rs` starts with `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, clippy::todo, clippy::unimplemented, clippy::indexing_slicing)]`, and `run()` in `src-tauri/src/lib.rs` contains no `.unwrap()`, `.expect(`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`, `println!` or `eprintln!` (the print macros panic on a closed stream): the `CLI_BINARY` lock recovers poisoning with `PoisonError::into_inner`, and a `build` failure is written to stderr with `let _ = writeln!(std::io::stderr(), ..)` and ends the process with `std::process::exit(1)`. The build-failure branch does not call `log::error!`: `Builder::build` runs before `setup` (Tauri 2.10.2 runs `setup` inside `App::run` on `Ready`), so no logger is installed yet and the call would be a no-op. The rest of `src-tauri` is out of scope for this rule in phase-a.
11. **Docs.** `CLAUDE.md` (line 55: `log_frontend` logs with `target: "frontend"`; line 58: the new file location and JSONL format), `.claude/codebase-map.md` and `CHANGELOG.md` document the new log location and format. The old `beads.log` is left on disk and neither read nor deleted.

## Required Work

- Keep the `log_info!`, `log_warn!`, `log_error!` and `log_debug!` macros, `LOGGING_ENABLED`, `VERBOSE_LOGGING` and all 9 logging commands.
- Keep the `=== Beads Task-Issue Tracker starting ===` line and every `[startup]` log. They must run after `install_logging`.
- Rendered line format: `[<timestamp>][<LEVEL>][<target>] [<action>] <message>`. When `action` equals the configured default action `log`, omit the `[<action>] ` segment.
- **Error inventory** (authoritative for a-4, by enum and variant):

  | Enum::Variant | Where | btit behavior |
  |---|---|---|
  | `sc_observability_log::InitError::*` (all variants) | `install_logging` in `setup` | propagated with `?`; the app does not start (unchanged from the plugin today). Tauri 2.10.2 turns the setup `Err` into `panic!("Failed to setup app: {e}")` inside `App::run` (`tauri-2.10.2/src/app.rs:1297-1300`); that panic is in Tauri, outside btit's no-panic scope |
  | `ServiceName::new` / `ActionName::new` `ValueValidationError` | `install_logging` | propagated with `?` (constant literals; cannot fail in practice) |
  | `sc_observability_log::FlushError::*` | `clear_logs` | `Err(format!("Failed to flush logs: {e}"))` from the command, file not truncated |
| `sc_observability_log::FlushError::*` | `on_run_event`, `Err(shared)` branch | `writeln!` to stderr of the variant (result ignored); exit continues |
  | `sc_observability_log::ShutdownError::*` | `on_run_event` | `writeln!` to stderr of the variant (result ignored); exit continues |
  | `DropCause::*` counts | `on_run_event` | one `warn` record listing non-zero causes before shutdown |
  | `tauri::Error` from `.build(..)` | `run()` | `writeln!` to stderr (result ignored), `std::process::exit(1)`; no `log::error!`, because no logger is installed before `setup` runs |

- Manual verification:
  1. Run `pnpm tauri:build` and launch the built app.
  2. Confirm that the JSONL file exists and contains startup records with `target` `app_lib` / `app_lib.cli`.
  3. Confirm that the debug panel shows colorized rendered lines.
  4. Confirm that `clear_logs` empties the panel and new records then appear.
  5. Quit the app and confirm the last record logged before quit is in the file.

  Record the file path and a 3-record excerpt in the PR description.

## Explicit Code Samples

```rust
// src-tauri/src/logging.rs (additions and changes)
#![deny(
    clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable,
    clippy::todo, clippy::unimplemented, clippy::indexing_slicing
)]
use std::io::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::Duration;
use sc_observability_log::{init, ActionName, BridgeOptions, DropCause, LevelFilter, LogGuard, LoggerConfig, ServiceName};
use tauri::Manager;

static LOG_GUARD: Mutex<Option<Arc<LogGuard>>> = Mutex::new(None);
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
const LOG_IO_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn install_logging(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let log_root = app.path().app_log_dir()?;
    let mut config = LoggerConfig::default_for(ServiceName::new("beads-task-issue-tracker")?, log_root);
    config.level = if cfg!(debug_assertions) { LevelFilter::Debug } else { LevelFilter::Info };
    config.enable_console_sink = true;
    let guard = init(config, BridgeOptions { default_action: ActionName::new("log")?, parse_bracket_action: true })?;
    if let Some(path) = guard.active_log_path() {
        let _ = LOG_PATH.set(path.to_path_buf());
    }
    *LOG_GUARD.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(guard));
    Ok(())
}

pub(crate) fn on_run_event(_app: &tauri::AppHandle, event: &tauri::RunEvent) {
    if !matches!(event, tauri::RunEvent::Exit) {
        return;
    }
    // Take the Arc and release the lock in the same statement: exit never waits
    // behind a clear_logs flush.
    let taken = LOG_GUARD.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(shared) = taken {
        let dropped = shared.dropped_events();
        if dropped.total() > 0 {
            let summary: Vec<String> = DropCause::ALL
                .iter()
                .filter(|c| dropped.get(**c) > 0)
                .map(|c| format!("{c:?}={}", dropped.get(*c)))
                .collect();
            log::warn!("[logging] dropped events: {}", summary.join(", "));
        }
        match Arc::try_unwrap(shared) {
            Ok(guard) => {
                if let Err(e) = guard.shutdown(LOG_IO_TIMEOUT) {
                    let _ = writeln!(std::io::stderr(), "beads-task-issue-tracker: log shutdown failed: {e:?}");
                }
            }
            // A clear_logs flush holds the other clone: flush once instead (still 1x LOG_IO_TIMEOUT).
            Err(shared) => {
                if let Err(e) = shared.flush(LOG_IO_TIMEOUT) {
                    let _ = writeln!(std::io::stderr(), "beads-task-issue-tracker: log flush at exit failed: {e:?}");
                }
            }
        }
    }
}

#[tauri::command]
pub(crate) async fn clear_logs() -> Result<(), String> {
    // Clone the Arc and release the lock before flushing.
    let shared = LOG_GUARD.lock().unwrap_or_else(PoisonError::into_inner).clone();
    if let Some(guard) = shared {
        guard.flush(LOG_IO_TIMEOUT).map_err(|e| format!("Failed to flush logs: {e}"))?;
    } // None: logger not installed (before setup) or already taken at exit — no flush.
    let log_path = get_log_path();
    if log_path.as_os_str().is_empty() || !log_path.exists() {
        return Ok(());
    }
    fs::write(&log_path, "").map_err(|e| format!("Failed to clear logs: {e}"))?;
    remove_rotated_logs(&log_path)?;
    log_info!("[debug] Logs cleared");
    Ok(())
}

/// Deletes `<active file name>.<N>` siblings, the names sc-observability rotates to
/// (`../sc-observability/crates/sc-observability/src/lib.rs:639`).
fn remove_rotated_logs(active: &Path) -> Result<(), String> {
    let (Some(dir), Some(name)) = (active.parent(), active.file_name().and_then(|n| n.to_str())) else {
        return Ok(());
    };
    let prefix = format!("{name}.");
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to list log dir: {e}"))?;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(candidate) = file_name.to_str() else { continue };
        let is_rotated = candidate
            .strip_prefix(prefix.as_str())
            .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()));
        if is_rotated {
            fs::remove_file(entry.path()).map_err(|e| format!("Failed to remove rotated log {candidate}: {e}"))?;
        }
    }
    Ok(())
}

pub(crate) fn get_log_path() -> PathBuf {
    LOG_PATH.get().cloned().unwrap_or_default()
}

#[tauri::command]
pub(crate) async fn log_frontend(level: String, message: String) {
    let (lvl, unknown) = match level.as_str() {
        "error" => (log::Level::Error, None),
        "warn" => (log::Level::Warn, None),
        "info" => (log::Level::Info, None),
        other => (log::Level::Info, Some(other)),
    };
    match unknown {
        None => log::log!(target: "frontend", lvl, "{}", message),
        Some(raw) => log::log!(target: "frontend", lvl, frontend_level = raw; "{}", message),
    }
}

// read_logs tail without panicking indexing:
// let lines: Vec<&str> = content.lines().collect();
// let start = lines.len().saturating_sub(n);
// Ok(lines.get(start..).unwrap_or_default().join("\n"))
```

```rust
// src-tauri/src/lib.rs (run)
let builder = tauri::Builder::default()
    // ...
    .setup(|app| {
        logging::install_logging(app)?;
        log::info!("=== Beads Task-Issue Tracker starting ===");
        // ... existing startup logging unchanged, except:
        *config::CLI_BINARY.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = config.cli_binary.clone();
        // ...
        Ok(())
    })
    .invoke_handler(tauri::generate_handler![/* unchanged */]);
// `build` does not run `setup` (Tauri 2.10.2 runs it inside `App::run` on Ready),
// so no logger exists in the Err branch: stderr only, no `log::error!`.
match builder.build(tauri::generate_context!()) {
    Ok(app) => app.run(|handle, event| logging::on_run_event(handle, &event)),
    Err(e) => {
        let _ = std::io::Write::write_fmt(&mut std::io::stderr(), format_args!("beads-task-issue-tracker: failed to build tauri application: {e}\n"));
        std::process::exit(1);
    }
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
- The no-panic rule for the rest of `src-tauri` (later rollout).
- Behavior issues B1–B13 in `docs/crate-split-refactor-issues.md`.

## Acceptance Criteria

1. `tauri-plugin-log` appears nowhere in `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` or `src-tauri/src`.
2. No Rust `log_*!`/`log::*!` call site and no `logFrontend(` call site changes. `git diff integrate/phase-a...HEAD` shows no changed lines matching those patterns, other than in `logging.rs` (`log_frontend`, `on_run_event`, `clear_logs`).
3. The built app writes `<app_log_dir>/logs/beads-task-issue-tracker.log.jsonl`. Startup records carry the expected `target`, and bracket-tagged records carry the tag as `action`. Evidence is in the PR description.
4. `read_logs`, `export_logs`, `clear_logs` and `get_log_path_string` work on the JSONL file. `clear_logs` truncates the active file and removes rotated files, and logging continues afterwards. Evidence is in the PR description.
5. `formatLogLine`/`formatLogLines` tests pass, and the debug panel shows rendered, colorized lines.
6. The per-OS `get_log_path` `cfg` blocks are gone, `logging.rs` has no platform-gated imports (closes refactor-review item A2 for `logging.rs` only), and `get_log_path` uses `LogGuard::active_log_path` rather than a hand-built path.
7. On `RunEvent::Exit` the guard is taken out of `LOG_GUARD` and exactly one bounded call is made: `shutdown(LOG_IO_TIMEOUT)` when `Arc::try_unwrap` succeeds, otherwise `flush(LOG_IO_TIMEOUT)`. No code path holds `LOG_GUARD` across `flush` or `shutdown` (`clear_logs` clones the `Arc` and releases the lock first), so exit is bounded by 1×`LOG_IO_TIMEOUT`. The last record logged before quit is present in the file. Evidence is in the PR description.
8. No panics in touched code: `logging.rs` carries the `#![deny(..)]` attribute from the code sample and `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets` reports no error; `grep -nE '\.unwrap\(\)|\.expect\(|panic!|unreachable!|todo!|unimplemented!|eprintln!|println!' src-tauri/src/lib.rs src-tauri/src/logging.rs` prints nothing.
9. Refactor-review item A1 is closed by design: JSONL records carry the Rust `log` target as `LogEvent.target` (for example `app_lib.cli`, taken from `module_path!()` via `log::Record::target()`), and the debug panel renders that field. No log line depends on the literal `[app_lib]` prefix. Evidence is in the PR description.
10. `CLAUDE.md` lines 55 and 58 describe the new mechanism and file.
11. The a-4 diff touches no file under `crates/`.
12. Every command in Required Validation passes.

## Required Validation

- `pnpm test`
- `npx vue-tsc --noEmit`
- `cargo test --locked --manifest-path src-tauri/Cargo.toml`
- `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets`
- `! grep -nE '\.unwrap\(\)|\.expect\(|panic!|unreachable!|todo!|unimplemented!|eprintln!|println!' src-tauri/src/lib.rs src-tauri/src/logging.rs`
- `python3 scripts/check_version_sync.py`
- `git diff --exit-code integrate/phase-a...HEAD -- crates/`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --all-targets`
- `pnpm tauri:build` (manual verification per Required Work)
