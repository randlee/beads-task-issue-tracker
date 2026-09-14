---
id: b-7
title: App rewire — Tauri commands on Arc<dyn CliBackend>; delete moved code
status: planned
branch: feature/sprint-b-7-app-rewire
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-7-app-rewire
target: integrate/phase-b
recommended_model: higher-effort (touches every app module; 65 command contracts must stay identical)
dependency_relations:
  - prerequisite: b-5
    dependent: b-7
    relation: must_follow
    rationale: "constructs BdCli; uses DoltOperations in migration.rs"
  - prerequisite: b-6
    dependent: b-7
    relation: must_follow
    rationale: "constructs BrCli; stack parent"
  - prerequisite: b-7
    dependent: b-9
    relation: must_follow
    rationale: "b-9 edits app modules and btit-bd/btit-cli state that b-7 finalizes"
  - prerequisite: none
    parallel_pair: [b-7, b-8]
    relation: parallel_safe
    rationale: "b-7 owns crates/btit-app/** and the root Cargo.lock; b-8 owns crates/btit-beads/** behind the frozen API"
---

# Sprint b-7 — App rewire: Tauri commands on `Arc<dyn CliBackend>`

## Recommended Agent / Model

Recommended model: higher-effort (touches every app module; 65 command contracts must stay identical).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Replace the app's global CLI state (`CLI_BINARY`, `CLI_CLIENT_INFO`, `PROJECT_LOCKS`, the `supports_*`/`uses_*` wrappers, `execute_bd`, `project_uses_dolt`, `AppInvoker`) with one backend slot holding `Arc<dyn CliBackend>`, built by probing the configured binary and selecting `BrCli` or `BdCli`.
- Route every Tauri command through the traits; delete `cli.rs` and the transitional adapter. Command names, argument names, return shapes and error strings are unchanged.

## Hard Dependencies

- b-5 and b-6 pushed (`BdCli`, `BrCli`, `BD_RELEASE_SOURCE`, `BR_RELEASE_SOURCE`).

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA; merge parent → child before every dev/fix round. PR-completion trigger: parent PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- b-5, b-6 → b-7 — `must_follow`.
- b-7 → b-9 — `must_follow`.
- b-7 ↔ b-8 — `parallel_safe`.

Stack: `phase-b-core` · layer 7.

## Exact Targets

Line numbers are at `a18c724` (`crates/btit-app/src/` after b-1; b-4/b-5 already shrank `cli.rs`).

- `crates/btit-app/src/backend.rs` (new): slot, factory, `check_bd_compatibility`
- `crates/btit-app/src/cli.rs`: deleted (remaining content: statics, `get_cli_client_info` 326-365, wrappers 380-466, `project_uses_dolt` 473-475, `reset_bd_version_cache` 518-521, `execute_bd` wrapper, `AppInvoker`, `check_bd_compatibility` 621-663)
- `crates/btit-app/src/lib.rs`: `mod cli;` → `mod backend;`; setup block 43-66 (`config::load_config`, `CLI_BINARY` write, startup probe) → `backend::install(&config.cli_binary)` + the same startup log lines through the slot; `generate_handler!` entry `cli::check_bd_compatibility` → `backend::check_bd_compatibility` (name of the command unchanged)
- `crates/btit-app/src/config.rs`: `CLI_BINARY` (8) removed; `get_cli_binary` (58-60) → `backend::current().binary().to_string()`; `set_cli_binary_path` (93-110): `reset_bd_version_cache()` → `backend::replace(&binary)`; `get_bd_version` (63-81) and `validate_cli_binary_internal` (118-151) use `btit_cli::run::probe_version_output`; `AppConfig`'s `#[serde(default = "crate::cli::default_cli_binary")]` (12) → `"btit_cli::probe::default_cli_binary"` path via a local fn
- `crates/btit-app/src/issue_commands.rs`: `AppInvoker` → `backend::current()`; `bd_close` → `backend.close` (the br flag now lives in `BrCli`); `bd_available_relation_types` → `backend.relation_types()`; `bd_delete` → `backend.delete` + unchanged attachment cleanup (480-504)
- `crates/btit-app/src/polling.rs`: `project_uses_dolt`/`uses_jsonl_files` (96, 153) → slot; `bd_poll_data` (43-60) → `backend.list`/`backend.ready`
- `crates/btit-app/src/migration.rs`: `sync_bd_database` (188-251), `bd_sync` (259-301) → `backend.sync` (mapping table in `sprint-b-4.md`); `bd_repair_database` (311-430): Dolt path → `backend.dolt()`, SQLite test call 398-408 → `run_raw`; `bd_check_needs_migration` (500-510) → `backend.client()/version()`; `bd_migrate_to_dolt` (569-1125): `migrate --to-dolt` 623-629, `init` 665-671/771-777, `import` 879-885 → `DoltOperations`; restore steps 944-950, 1003-1009, 1081-1087 → `run_raw` (OQ-7 default); `sqlite3` call 1049 unchanged; `ensure_refs_migrated_v3` (14) gets its doc comment (item A4)
- `crates/btit-app/src/watcher.rs:86`, `fs_commands.rs:50,73`: `project_uses_dolt` → slot
- `crates/btit-app/src/attachments.rs:189`: `purge_orphan_attachments` → `backend.list(include_all: true)`
- `crates/btit-app/src/updates.rs:271-319`: `check_bd_cli_update` selects `BR_RELEASE_SOURCE`/`BD_RELEASE_SOURCE` by `detect_cli_client(&version_str)` (unchanged decision input)
- `crates/btit-app/Cargo.toml`: dependencies `btit-bd`, `btit-br`, `btit-cli`, `btit-beads`, `btit-types`
- `docs/plans/phase-b/sprint-b-7.md` (`status:` frontmatter only)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Backend slot** (`backend.rs`, code sample): `static SLOT: RwLock<Option<Arc<dyn CliBackend>>>`, `static PROJECT_LOCKS: LazyLock<Arc<ProjectLocks>>` (one lock map for the process, so replacing the backend never allows two concurrent `bd` processes on one project), `install(binary)`, `replace(binary)`, `current() -> Arc<dyn CliBackend>`. `current()` before `install` builds a backend for `"bd"` (today's `CLI_BINARY` default, `config.rs:8`). Every lock access recovers poisoning with `PoisonError::into_inner`. `current()` is the only place a command obtains a backend (gate: `grep -rn 'BdCli::new\|BrCli::new' crates/btit-app/src` matches only `backend.rs`), so per-project selection (OQ-8) later becomes `for_project(&ProjectRef)` in this one module. The slot is the last process-global piece of CLI state; the library crates have none (plan "No process-global client state").
2. **Factory** `build_backend(binary) -> Arc<dyn CliBackend>`: `probe_cli_binary(binary)`; `Some(p) if p.client == Br` → `BrCli`; otherwise (`Bd`, `Unknown`, or no answer) → `BdCli`. Unit-tested with a fake probe through a `build_backend_with(binary, probe: Option<CliProbe>)` seam.
3. **Startup.** `setup` calls `backend::install(&config.cli_binary)` where today it writes `CLI_BINARY` (`lib.rs:46-48`), then logs the same `[startup]` lines using `backend.probe()`, `cli_client_name`, `cli_compatibility_warnings` and `extended_path_entries` (`lib.rs:52-66`).
4. **`check_bd_compatibility`** (moved to `backend.rs`, same command name and `CompatibilityInfo` shape): fresh `probe_cli_binary`; when `(probe.client, probe.version)` differs from `(backend.client(), backend.version())` the slot is rebuilt (today's cache refresh, `cli.rs:643-646`); the five capability fields come from `capabilities_for(client, tuple)` on the fresh probe, exactly as today's wrappers read the just-refreshed cache; `legacy`, `min_supported_major`, `searched_paths`, `warnings` unchanged.
5. **Commands rewired.** Every `#[tauri::command]` listed in Exact Targets calls the trait; each keeps its name, parameters, return type, log lines and `Err(String)` texts (`map_err(|e| e.to_string())` on `BeadsError`, plus the command-specific formats in `migration.rs` — `Failed to run bd doctor: {e}` (339), `Failed to run bd migrate: {e}` (629), `Failed to run bd init: {e}` (671), `Failed to run bd import: {e}` (885), `Repair failed: {stderr}` (352), `Sync failed: {stderr}` (293) — built from `BeadsError`/`CliOutput` fields). `bd_repair_database` on a Dolt project whose backend has `dolt() == None` returns `Err(BeadsError::Unsupported { operation: "Dolt repair", client }.to_string())` (unreachable today: only bd projects are detected as Dolt).
6. **Deletions.** `cli.rs`, `AppInvoker`, `CLI_BINARY`, `CLI_CLIENT_INFO`, `reset_bd_version_cache`, `execute_bd`, the wrappers and `project_uses_dolt` are gone from the app; `generate_handler!` has the same 65 names in the same order.
7. **Deviation record** (in this doc's Implementation Notes and the PR): (a) client kind is fixed per backend instance; re-detection happens in `set_cli_binary_path` and `check_bd_compatibility` (plan "Backend selection state"); (b) `purge_orphan_attachments` on bd < 0.55 uses the two-call list fallback; (c) poisoned mutexes are recovered instead of panicking; (d) `parse_issues_tolerant` context labels. Nothing else.
8. **Test preservation.** The plan's gate prints nothing: every baseline test exists somewhere in the workspace. The two `config.rs` tests (`config.rs:161-175`) stay; `cli.rs` has no tests left to move.
9. **Manual verification** (recorded in the PR): with `bd` 1.x configured — list/poll, show, create, update, close, delete (attachment folder removed), search, labels, dependencies, sync, repair on a Dolt project (doctor), `check_bd_compatibility` JSON; then switch Settings to a `br` binary if one is available, or to a non-existent path, and confirm `set_cli_binary_path` validation errors and `check_bd_compatibility.found = false` with `searchedPaths` populated.

## Required Work

- `ProjectRef::local(cwd)` is built at every command from its `cwd`/`options.cwd`; `resolve_working_dir` inside `btit-cli` keeps the `BEADS_PATH`/current-dir fallback. `migration.rs`, `polling.rs` and `attachment_refs.rs` keep their own `working_dir` computations for their filesystem work (they are not CLI invocations).
- `bd_check_needs_migration` (`migration.rs:500-510`): `match (backend.client(), backend.version()) { (CliClient::Bd, Some(v)) if v.major > 0 || v.minor >= 50 => .., _ => return Ok(..) }`, same reason strings.
- `bd_migrate_to_dolt` version guard (`migration.rs:596-605`): `backend.version()`; `None` → `Err("Could not determine bd version")` as today.
- `AppConfig` default: `fn default_binary() -> String { btit_cli::probe::default_cli_binary() }` referenced by `#[serde(default = "default_binary")]`.
- Changelog lines (collated by b-10): "The Tauri app selects a backend (`BdCli` or `BrCli`) per configured binary and drives every beads command through the `BeadsBackend` traits; command names, arguments and results are unchanged."

## Explicit Code Samples

```rust
// crates/btit-app/src/backend.rs
use std::sync::{Arc, LazyLock, PoisonError, RwLock};
use btit_beads::backend::CliBackend;
use btit_cli::{locks::ProjectLocks, probe::probe_cli_binary};
use btit_types::{CliClient, CliProbe};

static PROJECT_LOCKS: LazyLock<Arc<ProjectLocks>> = LazyLock::new(|| Arc::new(ProjectLocks::default()));
static SLOT: RwLock<Option<Arc<dyn CliBackend>>> = RwLock::new(None);

fn build_backend_with(binary: &str, probe: Option<CliProbe>) -> Arc<dyn CliBackend> {
    match probe {
        Some(p) if p.client == CliClient::Br => Arc::new(btit_br::BrCli::new(binary, PROJECT_LOCKS.clone())),
        _ => Arc::new(btit_bd::BdCli::new(binary, PROJECT_LOCKS.clone())),   // Bd, Unknown, or no answer (today's `_ =>` arms)
    }
}
pub(crate) fn build_backend(binary: &str) -> Arc<dyn CliBackend> { build_backend_with(binary, probe_cli_binary(binary)) }

pub(crate) fn install(binary: &str) { *SLOT.write().unwrap_or_else(PoisonError::into_inner) = Some(build_backend(binary)); }
pub(crate) fn replace(binary: &str) { install(binary) }   // set_cli_binary_path: today resets the version cache (config.rs:101)
pub(crate) fn current() -> Arc<dyn CliBackend> {
    if let Some(b) = SLOT.read().unwrap_or_else(PoisonError::into_inner).as_ref() { return b.clone(); }
    install("bd");                                           // config.rs:8 default before setup
    current()
}

#[tauri::command]
pub(crate) async fn check_bd_compatibility() -> btit_types::CompatibilityInfo { /* Deliverable 4 */ }
```

```rust
// crates/btit-app/src/issue_commands.rs (shape of every rewired command)
#[tauri::command]
pub(crate) async fn bd_show(id: String, options: CwdOptions) -> Result<Option<Issue>, String> {
    log_info!("[bd_show] Called for issue: {} with cwd: {:?}", id, options.cwd);
    sync_bd_database(options.cwd.as_deref());
    let backend = backend::current();
    let raw = backend.show(&ProjectRef::local(options.cwd), &id).map_err(|e| e.to_string())?;
    log_info!("[bd_show] Issue {} found: {}", id, raw.is_some());
    Ok(raw.map(transform_issue))
}
```

## This Sprint Does Not Close

- B-item behaviour fixes (b-8, b-9); B13 probe-failure caching (b-9).
- Workspace lints, formatting and clippy cleanliness for `btit-app` (b-10).
- Splitting attachments/migration/polling/watcher/updates/probe into crates (not in phase-b).

## Acceptance Criteria

1. `crates/btit-app/src/cli.rs` does not exist; `! grep -rnE 'CLI_CLIENT_INFO|CLI_BINARY|execute_bd|AppInvoker|reset_bd_version_cache|get_cli_client_info|supports_(daemon|list_all|delete_hard)_flag\(\)|uses_(jsonl_files|dolt_backend)\(\)' crates/btit-app/src`.
2. `sed -n '/generate_handler!\[/,/\]/p' crates/btit-app/src/lib.rs | grep -oE '[a-z_]+::[a-z_]+' | sed 's/.*:://'` equals the same extraction at `a18c724` (65 names, same order); `diff <(…baseline…) <(…HEAD…)` is empty.
3. `grep -c 'tauri::command' crates/btit-app/src/*.rs | awk -F: '{s+=$2} END {print s}'` is `65`.
4. `cargo tree -e normal -p beads-issue-tracker --depth 1` includes `btit-bd`, `btit-br`, `btit-cli`, `btit-beads`, `btit-types`; `cargo tree -e normal --workspace -i tauri` lists only `beads-issue-tracker` as a dependent; likewise for `sc-observability-log`.
5. `cargo test --workspace` passes; the test-preservation gate prints nothing.
6. The factory tests (Deliverable 2) and `check_bd_compatibility` JSON tests (moved to `btit-types` in b-2) pass; a new test asserts `check_bd_compatibility`'s capability fields equal `capabilities_for(probe)` for a seeded probe.
7. Manual verification (Deliverable 9) recorded in the PR; the deviation record (Deliverable 7) is in this doc's Implementation Notes.
8. `git diff --exit-code feature/sprint-b-6-btit-br...HEAD -- crates/btit-types crates/btit-beads crates/btit-cli crates/btit-bd crates/btit-br` is empty.
9. CI green; every command in Required Validation passes.

## Required Validation

- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --manifest-path crates/btit-app/Cargo.toml --all-targets` (warnings allowed until b-10; no errors)
- `cargo fmt --check -p btit-types -p btit-beads -p btit-cli -p btit-bd -p btit-br`
- `git diff --exit-code feature/sprint-b-6-btit-br...HEAD -- crates/btit-types crates/btit-beads crates/btit-cli crates/btit-bd crates/btit-br`
- `pnpm test`
- `npx vue-tsc --noEmit`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate; the `generate_handler!` diff from Acceptance Criterion 2
- `pnpm tauri:dev` (manual, Deliverable 9)
