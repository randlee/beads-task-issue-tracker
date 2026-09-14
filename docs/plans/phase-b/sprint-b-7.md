---
id: b-7
title: Backend slot and beads-command rewire — Arc<dyn BeadsBackend> in the app
status: planned
branch: feature/sprint-b-7-backend-slot
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-7-backend-slot
target: integrate/phase-b
recommended_model: higher-effort (introduces the slot every command depends on; 65 command contracts must stay identical)
dependency_relations:
  - prerequisite: b-5
    dependent: b-7
    relation: must_follow
    rationale: "constructs BdCli; join layer of group A"
  - prerequisite: b-6
    dependent: b-7
    relation: must_follow
    rationale: "constructs BrCli; join layer of group A"
  - prerequisite: b-9
    dependent: b-7
    relation: must_follow
    rationale: "join layer of group A: b-9 is merged into this branch; no content dependency"
  - prerequisite: b-7
    dependent: b-8
    relation: must_follow
    rationale: "b-8 rewires the remaining app modules onto the slot and deletes the shims b-7 leaves; group B forks from the b-7 head"
  - prerequisite: b-7
    dependent: b-10
    relation: must_follow
    rationale: "fork point only: group B forks from the b-7 head (b-10's content dependency is b-5)"
---

# Sprint b-7 — Backend slot and beads-command rewire

## Recommended Agent / Model

Recommended model: higher-effort (introduces the slot every command depends on; 65 command contracts must stay identical).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Introduce the app's single backend slot holding `Arc<dyn BeadsBackend>` (transport-neutral; CLI facts are reached through `backend.cli()`), built by probing the configured binary and selecting `BrCli` or `BdCli`, and move `check_bd_compatibility` next to it.
- Route the beads commands that already run through `ops` (`issue_commands.rs`, `bd_poll_data`, `purge_orphan_attachments`, `sync_bd_database`/`bd_sync`) and the CLI configuration commands (`config.rs`) through the slot; delete `AppInvoker`, `execute_bd` and every CLI static.
- Leave `cli.rs` as a handful of one-line shims delegating to the slot for the modules b-8 rewires (`migration.rs` repair/migrate, `polling.rs` mtime, `watcher.rs`, `fs_commands.rs`, `updates.rs`), so this sprint and b-8 have a clean boundary.

## Hard Dependencies

- Group A: b-5, b-6 and b-9 pushed (merge-forward); all three merged into this branch before closure (PR-completion). This branch is created from the group A first closer's head (layer 5).

## Dependency Relations

Trigger definitions, per-branch QA and fix-layer rules: `plan-phase-b.md` "Dependency relations" and "Parallel groups: fork and re-merge".

- b-5, b-6, b-9 → b-7 — `must_follow` (join layer of group A: `git merge --no-ff origin/<member>` before every dev round for each pushed member; closure requires all three merged in).
- b-7 → b-8, b-7 → b-10 — `must_follow` (group B forks from this branch's head after closure and QA-1 without Blocking finding).

Stack: `phase-b-core` · layer 6.

## Exact Targets

Line numbers are at `a18c724` (`crates/btit-app/src/` after b-1; b-4 already moved the op bodies out and added `AppInvoker`).

- `crates/btit-app/src/backend.rs` (new): slot, factory, `check_bd_compatibility`
- `crates/btit-app/src/cli.rs`: deleted content: statics `CLI_CLIENT_INFO` (19-20) and `PROJECT_LOCKS` (b-4), `get_cli_client_info` (326-365), the five wrappers (380-466), the app copy of `project_uses_dolt_for` (482-515), `reset_bd_version_cache` (518-521), `execute_bd` wrapper and `AppInvoker` (b-4), `check_bd_compatibility` (621-663, moves to `backend.rs`). Remaining content: shims `project_uses_dolt(beads_dir)`, `supports_daemon_flag()`, `uses_jsonl_files()`, `client_info() -> (CliClient, Option<CliVersion>)`, each one line over `backend::current()`, for `migration.rs:200,224,270,279,330,371,399,500,513,588,596`, `polling.rs:96,153`, `watcher.rs:86`, `fs_commands.rs:50,73`
- `crates/btit-app/src/lib.rs`: `mod backend;`; setup block 43-66 (`config::load_config`, `CLI_BINARY` write, startup probe) → `backend::install(&config.cli_binary)` + the same startup log lines through the slot; `generate_handler!` entry `cli::check_bd_compatibility` → `backend::check_bd_compatibility` (command name unchanged)
- `crates/btit-app/src/config.rs`: `CLI_BINARY` (8) removed; `get_cli_binary` (58-60) → `backend::cli_or_unsupported("cli binary")?.binary().to_string()` (a `backend.rs` helper: `current().cli()` mapped to `BeadsError::Unsupported`); `set_cli_binary_path` (93-110): `reset_bd_version_cache()` → `backend::replace(&binary)`; `get_bd_version` (63-81) and `validate_cli_binary_internal` (118-151) use `btit_cli::run::probe_version_output`; `AppConfig`'s `#[serde(default = "crate::cli::default_cli_binary")]` (12) → a local `default_binary()` over `btit_cli::probe::default_cli_binary`
- `crates/btit-app/src/issue_commands.rs`: `AppInvoker` → `backend::current()`; `bd_close` → `backend.close` (the br flag now lives in `BrCli`); `bd_available_relation_types` → `backend.relation_types()`; `bd_delete` → `backend.delete` + unchanged attachment cleanup (480-504)
- `crates/btit-app/src/polling.rs:34-89`: `bd_poll_data` → `backend.list`/`backend.ready`
- `crates/btit-app/src/attachments.rs:189`: `purge_orphan_attachments` → `backend.list(include_all: true)`
- `crates/btit-app/src/migration.rs:188-301`: `sync_bd_database`, `bd_sync` → `backend.sync` (mapping table in `sprint-b-4.md`)
- `crates/btit-app/Cargo.toml`: dependencies `btit-bd`, `btit-br`
- `docs/plans/phase-b/sprint-b-7.md` (`status:` frontmatter and Implementation Notes)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Backend slot** (`backend.rs`, code sample): `static SLOT: RwLock<Option<Arc<dyn BeadsBackend>>>`, `static PROJECT_LOCKS: LazyLock<Arc<ProjectLocks>>` (one lock map for the process, so replacing the backend never allows two concurrent `bd` processes on one project), `install(binary)`, `replace(binary)`, `current() -> Arc<dyn BeadsBackend>`, and `with_cli<T>(operation: &'static str, f: impl FnOnce(&dyn CliBackend) -> T) -> Result<T, BeadsError>` which calls `current().cli()` and maps `None` to `BeadsError::Unsupported { operation, client: CliClient::Unknown }` (unreachable with `BdCli`/`BrCli`; the seam a non-CLI transport would hit). `current()` before `install` builds a backend for `"bd"` (today's `CLI_BINARY` default, `config.rs:8`). Every lock access recovers poisoning with `PoisonError::into_inner`. `current()` is the only place a command obtains a backend (gate: `grep -rn 'BdCli::new\|BrCli::new' crates/btit-app/src` matches only `backend.rs`), so per-project selection (OQ-8) later becomes `for_project(&ProjectRef)` in this one module. The slot is the last process-global piece of CLI state; the library crates have none (plan "No process-global client state").
2. **Factory** `build_backend(binary) -> Arc<dyn BeadsBackend>`: `probe_cli_binary(binary)`; `Some(p) if p.client == Br` → `BrCli`; otherwise (`Bd`, `Unknown`, or no answer) → `BdCli`. Unit-tested with a fake probe through a `build_backend_with(binary, probe: Option<CliProbe>)` seam.
3. **Startup.** `setup` calls `backend::install(&config.cli_binary)` where today it writes `CLI_BINARY` (`lib.rs:46-48`), then logs the same `[startup]` lines using `with_cli(.., |c| c.probe())`, `cli_client_name`, `cli_compatibility_warnings` and `extended_path_entries` (`lib.rs:52-66`).
4. **`check_bd_compatibility`** (moved to `backend.rs`, same command name and `CompatibilityInfo` shape): fresh `probe_cli_binary`; when `(probe.client, probe.version)` differs from the slot's `cli().map(|c| (c.client(), c.version()))` the slot is rebuilt (today's cache refresh, `cli.rs:643-646`); the five capability fields come from `capabilities_for(client, tuple)` on the fresh probe, exactly as today's wrappers read the just-refreshed cache; `legacy`, `min_supported_major`, `searched_paths`, `warnings` unchanged.
5. **Commands rewired.** Every `#[tauri::command]` in `issue_commands.rs`, plus `bd_poll_data`, `purge_orphan_attachments`, `bd_sync`, `get_bd_version`, `get_cli_binary_path`, `set_cli_binary_path`, `validate_cli_binary`, keeps its name, parameters, return type, log lines and `Err(String)` texts (`map_err(|e| e.to_string())` on `BeadsError`; the `sync` mapping table of `sprint-b-4.md` unchanged). Transport-neutral operations go through `backend::current()` directly; `get_cli_binary`/`get_cli_binary_path` go through `with_cli` (`binary()`).
6. **Shims for b-8.** `cli.rs` keeps exactly four `pub(crate)` one-liners (`project_uses_dolt(beads_dir)`, `supports_daemon_flag`, `uses_jsonl_files`, `client_info`) delegating to the slot, so `migration.rs`, `polling.rs` (`get_beads_mtime`), `watcher.rs` and `fs_commands.rs` compile unchanged; `updates.rs:1-3` keeps its `use crate::cli::{detect_cli_client, parse_bd_version}` re-exports. Nothing else remains in `cli.rs`. The `project_uses_dolt(beads_dir: &Path)` shim keeps today's path-based signature and turns it into `ProjectRef::local(Some(<beads_dir's parent>))` (every caller passes `<working_dir>/.beads`); b-8 replaces the callers with `ProjectRef`s directly.
7. **Deletions.** `AppInvoker`, `execute_bd`, `CLI_BINARY`, `CLI_CLIENT_INFO`, `PROJECT_LOCKS` (moved), `reset_bd_version_cache`, `get_cli_client_info`, the app copy of `project_uses_dolt_for`, and the unused wrappers `supports_list_all_flag`, `supports_delete_hard_flag`, `uses_dolt_backend` are gone from the app; `generate_handler!` has the same 65 names in the same order; the plan's command-signature gate (1) and frontend invoke-subset gate (2) pass.
8. **Deviation record** (Implementation Notes and the PR): (a) client kind is fixed per backend instance; re-detection happens in `set_cli_binary_path` and `check_bd_compatibility` (plan "Backend selection state"); (b) poisoned mutexes are recovered instead of panicking; (c) `parse_issues_tolerant` context labels; (d) the `log` target of the moved CLI code changes with its module path (`app_lib::cli` → `btit_cli::run` etc., `module_path!()` via `log::Record::target()`; the JSONL `target` field and the debug panel do not depend on the old names, `sprint-a-4.md` AC 9). (The `purge_orphan_attachments` legacy-fallback delta was recorded by b-4.) Nothing else.
9. **Group A join.** b-5, b-6 and b-9 are merged into this branch; evidence: `git branch --contains origin/feature/sprint-b-5-btit-bd`, `… b-6-btit-br`, `… b-9-beads-domain-fixes` each list this branch (the stack parent is contained trivially, the late finishers through their join merges); the test-preservation gate is run with b-9's replacement list applied and prints nothing. This join layer is rebased only with `git rebase --rebase-merges feature/<layer-5>` (plan "gh-stack and worktree workflow"), so the join merges survive.
10. **Manual verification** (recorded in the PR): with `bd` 1.x configured — list/poll, show, create, update, close, delete (attachment folder removed), search, labels, dependencies, sync, `check_bd_compatibility` JSON; then switch Settings to a `br` binary if one is available, or to a non-existent path, and confirm `set_cli_binary_path` validation errors and `check_bd_compatibility.found = false` with `searchedPaths` populated.

## Required Work

- `ProjectRef::local(cwd)` is built at every command from its `cwd`/`options.cwd`; `resolve_working_dir` inside `btit-cli` keeps the `BEADS_PATH`/current-dir fallback. `migration.rs`, `polling.rs` and `attachment_refs.rs` keep their own `working_dir` computations for their filesystem work.
- `AppConfig` default: `fn default_binary() -> String { btit_cli::probe::default_cli_binary() }` referenced by `#[serde(default = "default_binary")]`.
- Changelog lines (collated by b-12): "The Tauri app selects a backend (`BdCli` or `BrCli`) per configured binary and drives every beads command through the `BeadsBackend` traits; command names, arguments and results are unchanged."

## Explicit Code Samples

```rust
// crates/btit-app/src/backend.rs
use std::sync::{Arc, LazyLock, PoisonError, RwLock};
use btit_beads::{backend::{BeadsBackend, CliBackend}, error::BeadsError};
use btit_cli::{locks::ProjectLocks, probe::probe_cli_binary};
use btit_types::{CliClient, CliProbe};

static PROJECT_LOCKS: LazyLock<Arc<ProjectLocks>> = LazyLock::new(|| Arc::new(ProjectLocks::default()));
static SLOT: RwLock<Option<Arc<dyn BeadsBackend>>> = RwLock::new(None);

fn build_backend_with(binary: &str, probe: Option<CliProbe>) -> Arc<dyn BeadsBackend> {
    match probe {
        Some(p) if p.client == CliClient::Br => Arc::new(btit_br::BrCli::new(binary, PROJECT_LOCKS.clone())),
        _ => Arc::new(btit_bd::BdCli::new(binary, PROJECT_LOCKS.clone())),   // Bd, Unknown, or no answer (today's `_ =>` arms)
    }
}
pub(crate) fn build_backend(binary: &str) -> Arc<dyn BeadsBackend> { build_backend_with(binary, probe_cli_binary(binary)) }

pub(crate) fn install(binary: &str) { *SLOT.write().unwrap_or_else(PoisonError::into_inner) = Some(build_backend(binary)); }
pub(crate) fn replace(binary: &str) { install(binary) }   // set_cli_binary_path: today resets the version cache (config.rs:101)
pub(crate) fn current() -> Arc<dyn BeadsBackend> {
    if let Some(b) = SLOT.read().unwrap_or_else(PoisonError::into_inner).as_ref() { return b.clone(); }
    install("bd");                                           // config.rs:8 default before setup
    current()
}
/// CLI-only access (config.rs, updates.rs, migration.rs raw paths, check_bd_compatibility). `None` only for a non-CLI transport.
pub(crate) fn with_cli<T>(operation: &'static str, f: impl FnOnce(&dyn CliBackend) -> T) -> Result<T, BeadsError> {
    let backend = current();
    backend.cli().map(f).ok_or(BeadsError::Unsupported { operation, client: CliClient::Unknown })
}

#[tauri::command]
pub(crate) async fn check_bd_compatibility() -> btit_types::CompatibilityInfo { /* Deliverable 4 */ }
```

```rust
// crates/btit-app/src/cli.rs after this sprint (whole file; deleted by b-8)
//! Transitional shims for the modules b-8 rewires. Each is one call into the backend slot.
use crate::backend;
pub(crate) fn project_uses_dolt(beads_dir: &std::path::Path) -> bool {
    // callers pass `<working_dir>/.beads`; the trait takes the project
    let cwd = beads_dir.parent().map(|p| p.to_string_lossy().into_owned());
    backend::current().project_uses_dolt(&btit_types::ProjectRef::local(cwd))
}
fn caps() -> btit_types::BackendCapabilities { backend::with_cli("capabilities", |c| c.capabilities()).unwrap_or_default() }
pub(crate) fn supports_daemon_flag() -> bool { caps().supports_daemon_flag }
pub(crate) fn uses_jsonl_files() -> bool { caps().uses_jsonl_files }
pub(crate) fn client_info() -> (btit_types::CliClient, Option<btit_types::CliVersion>) {
    backend::with_cli("client info", |c| (c.client(), c.version())).unwrap_or((btit_types::CliClient::Unknown, None))
}
pub(crate) use btit_beads::detect::{detect_cli_client, parse_bd_version};
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

- `migration.rs` repair/check/migrate, `get_beads_mtime`, `watcher.rs`, `fs_commands.rs`, `updates.rs` on the slot, and the deletion of `cli.rs` (b-8).
- B-item behaviour fixes (b-9, b-10, b-11); B13 probe-failure caching (b-11).
- Workspace lints, formatting and clippy cleanliness for `btit-app` (b-12).

## Acceptance Criteria

1. `! grep -rnE 'CLI_CLIENT_INFO|CLI_BINARY|execute_bd|AppInvoker|reset_bd_version_cache|get_cli_client_info|supports_(list_all|delete_hard)_flag\(\)|uses_dolt_backend\(\)|fn project_uses_dolt_for' crates/btit-app/src`; `grep -c 'pub(crate) fn' crates/btit-app/src/cli.rs` is `4`; `grep -rn 'dyn CliBackend' crates/btit-app/src` matches only `backend.rs` (the slot type is `Arc<dyn BeadsBackend>`; CLI access goes through `with_cli`).
2. `sed -n '/generate_handler!\[/,/\]/p' crates/btit-app/src/lib.rs | grep -oE '[a-z_]+::[a-z_]+' | sed 's/.*:://'` equals the same extraction at `a18c724` (65 names, same order); the plan's command-signature gate (1) diffs empty and the frontend invoke-subset gate (2) prints nothing.
3. `grep -c 'tauri::command' crates/btit-app/src/*.rs | awk -F: '{s+=$2} END {print s}'` is `65`.
4. `cargo tree -e normal -p beads-issue-tracker --depth 1` includes `btit-bd`, `btit-br`, `btit-cli`, `btit-beads`, `btit-types`; the only workspace crate depending on `tauri` or `sc-observability-log` is the app: `for d in tauri sc-observability-log; do cargo tree -e normal --workspace -i "$d" --depth 1 --prefix none --format '{p}' | sed -E 's/ v.*//' | grep -E '^(btit-|beads-issue-tracker)' | sort -u | diff - <(echo beads-issue-tracker); done` is empty (inverted tree, one level up from the dependency).
5. b-5, b-6 and b-9 are merged in (`git branch --contains origin/feature/sprint-b-9-beads-domain-fixes` lists this branch, likewise the other two); `cargo test --workspace` passes; the test-preservation gate with b-9's replacement list prints nothing.
6. The factory tests (Deliverable 2) pass; a new test asserts `check_bd_compatibility`'s capability fields equal `capabilities_for(probe)` for a seeded probe.
7. Manual verification (Deliverable 10) recorded in the PR; the deviation record (Deliverable 8) is in this doc's Implementation Notes.
8. Join-layer file discipline, two mechanical checks: (a) `git diff --exit-code <layer-5 head>...HEAD -- crates/btit-types crates/btit-cli` is empty (no group A member and no b-7 commit touches those crates; `crates/btit-beads`, `crates/btit-bd`, `crates/btit-br` legitimately change through the merged members and are covered by the members' own gates); (b) b-7's own commits touch only the app: `git log --first-parent --no-merges --format=%H <layer-5 head>..HEAD | xargs -I{} git show --name-only --format= {} | sort -u | grep -vE '^(crates/btit-app/|Cargo.lock$|docs/plans/phase-b/sprint-b-7.md$)'` prints nothing (`--first-parent` excludes the merged members' commits; `--no-merges` excludes the join merges themselves).
9. QA-1 complete; CI green; every command in Required Validation passes.

## Required Validation

- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --manifest-path crates/btit-app/Cargo.toml --all-targets` (warnings allowed until b-12; no errors)
- `cargo fmt --check -p btit-types -p btit-beads -p btit-cli -p btit-bd -p btit-br`
- `git diff --exit-code <layer-5 head>...HEAD -- crates/btit-types crates/btit-cli`
- `git log --first-parent --no-merges --format=%H <layer-5 head>..HEAD | xargs -I{} git show --name-only --format= {} | sort -u | grep -vE '^(crates/btit-app/|Cargo.lock$|docs/plans/phase-b/sprint-b-7.md$)'` prints nothing
- command-signature gate (1) and frontend invoke-subset gate (2) from `plan-phase-b.md` "Command contract gates"
- the inverted `cargo tree` gate from Acceptance Criterion 4
- `pnpm test`
- `npx vue-tsc --noEmit`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate (b-9 replacement list applied); the `generate_handler!` diff from Acceptance Criterion 2
- `pnpm tauri:dev` (manual, Deliverable 10)
