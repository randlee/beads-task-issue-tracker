---
id: b-4
title: btit-cli crate — shared CLI transport and issue-operation bodies
status: planned
branch: feature/sprint-b-4-btit-cli
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-4-btit-cli
target: integrate/phase-b
recommended_model: higher-effort (largest code motion; process, lock and PATH semantics must stay identical)
dependency_relations:
  - prerequisite: b-3
    dependent: b-4
    relation: must_follow
    rationale: "CliRunner returns BeadsError, uses parse/gates/detect and the log_*! macros from btit-beads; stack parent"
  - prerequisite: b-4
    dependent: b-5
    relation: must_follow
    rationale: "BdCli wraps CliRunner and the shared ops"
  - prerequisite: b-4
    dependent: b-6
    relation: must_follow
    rationale: "BrCli wraps CliRunner and the shared ops (content dependency; b-6 is stacked on b-5 for linearity)"
  - prerequisite: none
    parallel_pair: [b-4, b-8]
    relation: parallel_safe
    rationale: "b-4 owns crates/btit-cli/** and the app's cli.rs/issue_commands.rs adoption; b-8 owns crates/btit-beads/** behind the frozen API"
---

# Sprint b-4 — `btit-cli`: shared CLI transport and issue-operation bodies

## Recommended Agent / Model

Recommended model: higher-effort (largest code motion; process, lock and PATH semantics must stay identical).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Create `crates/btit-cli`, the transport both CLIs share: extended `PATH`, `new_command`, the per-project lock, the JSON/raw invocation functions, the `--version` probe and auto-detection, and the issue-operation bodies that `issue_commands.rs`, `polling.rs`, `attachments.rs` and `migration.rs` execute through `execute_bd` today.
- Provide `CliRunner`, the per-binary instance that `BdCli` (b-5) and `BrCli` (b-6) wrap.
- The app adopts the crate immediately through a transitional `AppInvoker` adapter over its existing statics, so no invocation logic exists twice. `execute_bd` becomes a thin wrapper.

## Hard Dependencies

- b-3 pushed (`BeadsError`, `capabilities_for`, `parse_issues_tolerant`, `detect::*`, `log_*!` macros).

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA; merge parent → child before every dev/fix round. PR-completion trigger: parent PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- b-3 → b-4 — `must_follow`.
- b-4 → b-5, b-4 → b-6 — `must_follow` (content).
- b-4 ↔ b-8 — `parallel_safe`: b-4 does not edit `crates/btit-beads/**`; b-8 edits nothing outside it.

Stack: `phase-b-core` · layer 4.

## Exact Targets

Line numbers are at `a18c724` (`crates/btit-app/src/` after b-1).

- `Cargo.toml` (root): member `crates/btit-cli`
- `crates/btit-cli/Cargo.toml`, `clippy.toml`, `src/lib.rs`, `src/path.rs`, `src/command.rs`, `src/locks.rs`, `src/run.rs`, `src/probe.rs`, `src/runner.rs`, `src/ops.rs`, `tests/api_freeze.rs`
- Moved out of `crates/btit-app/src/cli.rs`: 14-15 (`BD_PROJECT_LOCKS` → `ProjectLocks`), 22-59 (`get_extended_path`), 63-72 (`new_command`), 185-196 (`probe_cli_binary`), 200-207 (`extended_path_entries`), 255-281 (`default_cli_binary`), 523-592 (`execute_bd` body → `run_json`); tests 754-758, 804-817, 846-874, 1116-1161, 1222-1238
- Moved out of `crates/btit-app/src/config.rs`: the `--version` spawn at 65-69 and 129-133 → `probe_version_output`
- Moved out of `crates/btit-app/src/issue_commands.rs`: the bodies of `bd_list` (18-72), `bd_count` fetch (81-90), `bd_ready` (139-141), `bd_status` (149-152), `bd_show` (162-208), `bd_create` (214-276), `bd_update` (285-402), `bd_close` (409-426), `bd_search` (433-449), `bd_label_add`/`bd_label_remove` (455-456, 463-464), `bd_delete` CLI part (470-475), `bd_comments_add` (511-513), `bd_dep_add`/`bd_dep_remove`/`bd_dep_add_relation`/`bd_dep_remove_relation` (520-551), `bd_available_relation_types` table (556-578) → `ops.rs`
- Moved out of `crates/btit-app/src/migration.rs`: the sync invocation 222-232 → `ops::sync`
- `crates/btit-app/src/cli.rs`: `execute_bd` becomes a wrapper; new `pub(crate) struct AppInvoker;` implementing `btit_cli::CliInvoker` over `config::CLI_BINARY`, `CLI_CLIENT_INFO` and a `static PROJECT_LOCKS: LazyLock<Arc<ProjectLocks>>`
- `crates/btit-app/src/issue_commands.rs`, `polling.rs:43-60`, `attachments.rs:189-191`, `migration.rs:222-250`: call `btit_cli::ops::*` through `AppInvoker`
- `.github/workflows/ci.yml`: `rust-quality` gains `btit-cli`
- `docs/plans/phase-b/sprint-b-4.md` (`status:` frontmatter only)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Crate.** `crates/btit-cli`, `[lints] workspace = true`, `#![deny(missing_docs)]`, `publish = false`. Dependencies: `btit-types`, `btit-beads`, `serde_json`, `log`. No Tauri, no `sc-observability-log`.
2. **`path.rs`.** `get_extended_path` and `extended_path_entries` moved verbatim (`cli.rs:22-59, 200-207`), including the per-OS extra directories and separators. `command.rs`: `new_command` verbatim (`CREATE_NO_WINDOW` on Windows, `cli.rs:63-72`).
3. **`locks.rs`.** `pub struct ProjectLocks` wrapping today's `Mutex<HashMap<String, Arc<Mutex<()>>>>` (`cli.rs:14-15`) with `fn guard(&self, working_dir: &str) -> Arc<Mutex<()>>`. Poisoning is recovered with `PoisonError::into_inner` where today's code calls `.unwrap()` (`cli.rs:548,553`); this is the one behaviour delta (no panic on a poisoned lock) and is listed in the PR.
4. **`run.rs`.** `resolve_working_dir(project: &ProjectRef, client: CliClient) -> Result<String, BeadsError>` (cwd → `BEADS_PATH` → `current_dir()` → `"."`, `cli.rs:524-531`; the `#[non_exhaustive]` wildcard arm returns `BeadsError::Unsupported { operation: "non-local project reference", client }`). `run_json(binary, no_daemon: bool, locks: &ProjectLocks, working_dir: &str, command: &str, args: &[String]) -> Result<String, BeadsError>` is `execute_bd`'s body (`cli.rs:533-591`): subcommand split, `--no-daemon` when `no_daemon`, `--json`, the `[bd] …` log lines, the project lock, `PATH`/`BEADS_PATH` env, `SchemaMigration` on `no such column: spec_id`, `CommandFailed` with stderr, the `VERBOSE_LOGGING` preview. `run_raw(binary, working_dir, args: &[&str]) -> Result<CliOutput, BeadsError>`: `new_command(binary).args(args).current_dir(working_dir).env("PATH", ..).env("BEADS_PATH", working_dir).output()`, mapping the spawn error to `Spawn { operation: args.first() }` and any exit status to `Ok(CliOutput)` (the caller decides, as `migration.rs` does today). `probe_version_output(binary) -> Result<CliOutput, BeadsError>`: `--version` from `std::env::temp_dir()` with the extended `PATH` (`config.rs:65-69,129-133`, `cli.rs:186-190`).
5. **`probe.rs`.** `probe_cli_binary(binary) -> Option<CliProbe>` (`cli.rs:185-196`, via `probe_version_output` + `parse_cli_probe`) and `default_cli_binary() -> String` (`cli.rs:255-281`, ungated `log::info!/warn!` kept).
6. **`runner.rs`.** The `CliInvoker` trait and `CliRunner` exactly as in the code samples. `CliRunner::client_info()` reproduces `get_cli_client_info` (`cli.rs:326-365`): probe lazily, cache only a parsed success, log the same lines. `CliRunner::capabilities()` = `capabilities_for(client, version)`.
7. **`ops.rs`.** One function per operation, generic over `&dyn CliInvoker`, with today's bodies: `list` (with the `--all` two-call fallback when `!supports_list_all_flag`, `issue_commands.rs:22-39`), `ready`, `status`, `show` (not-found via `e.to_string().to_lowercase()` containing `no issue found`/`not found`, empty stdout, array-or-object, strict deserialize → `ParseFailed { target: Issue, id: Some(id) }`), `create`, `update` (empty stdout → `show` fallback with lenient `.ok()`), `close(inv, project, id, suggest_next: bool)`, `search`, `label_add`, `label_remove`, `delete(inv, project, id, hard: bool)`, `comment_add`, `dep_add(.., relation_type: Option<&str>)`, `dep_remove`, `relation_types(client) -> Vec<RelationType>` (`issue_commands.rs:556-578`), `sync(inv, project, no_daemon) -> Result<(), BeadsError>` (`migration.rs:222-232`, non-zero exit → `CommandFailed`). Log lines keep their text; the `context` label of `parse_issues_tolerant` calls may become the op name (plan "Behaviour preserved").
8. **App adoption, no duplicated logic.** `execute_bd` = `run_json(&get_cli_binary(), supports_daemon_flag(), &PROJECT_LOCKS, &wd, command, args).map_err(|e| e.to_string())` with `wd` from `resolve_working_dir`. `AppInvoker` implements `CliInvoker` over the statics (transitional; deleted by b-7). Every `#[tauri::command]` in `issue_commands.rs` keeps its signature and calls the `ops` function, keeping its own log lines, `transform_issue` mapping and `map_err(|e| e.to_string())`; `bd_delete` keeps the attachment-folder cleanup; `bd_available_relation_types` maps `RelationType` to the same `{"value","label"}` JSON. `polling.rs` `bd_poll_data` uses `ops::list` with `include_all: true` and partitions on `status != "closed"` (today's `--all` path; for the two-call fallback the merged list contains the same issues, `polling.rs:43-56`). `attachments.rs` `purge_orphan_attachments` uses `ops::list(include_all: true)` (today an unconditional `--all` call, `attachments.rs:189`; on bd < 0.55 this now takes the fallback — legacy-only delta listed in the PR). `migration.rs` `sync_bd_database`/`bd_sync` call `ops::sync` and map the result to today's log/return text (table in Required Work).
9. **Tests moved.** The cli.rs tests in Exact Targets move to `btit-cli`. New unit tests cover the pure pieces this sprint introduces: `resolve_working_dir` precedence (cwd, `BEADS_PATH`, current dir), the arg vectors built by each `ops` function through a `#[cfg(test)]` recording `CliInvoker` (asserting, for example, `list` with `include_all` on a `supports_list_all_flag = false` invoker issues `list --limit=0` then `list --limit=0 --status=closed`, `issue_commands.rs:25-33`), `show`'s not-found and shape handling, `update`'s empty-output fallback, `close` with and without `--suggest-next`, `delete` with and without `--hard`, `relation_types` for `Br` vs others.
10. **API freeze and CI.** `crates/btit-cli/tests/api_freeze.rs` pins `CliInvoker`, `CliRunner` and every `ops` signature; `rust-quality` covers `btit-cli`.

## Required Work

- **`sync` result mapping in the app** (byte-exact with `migration.rs:234-250, 290-300`):

  | `ops::sync` result | `sync_bd_database` | `bd_sync` |
  |---|---|---|
  | `Ok(())` | `log_info!("[sync] Sync completed successfully")`; cooldown updated | same log; cooldown updated; `Ok(())` |
  | `Err(CommandFailed { stderr, .. })` | `log_warn!("[sync] {} sync failed: {}", binary, stderr)` | `log_error!("[bd_sync] Sync failed: {}", stderr.trim())`; `Err(format!("Sync failed: {}", stderr.trim()))` |
  | `Err(Spawn { source, .. })` | `log_error!("[sync] Failed to run {} sync: {}", binary, source)` | `Err(format!("Failed to run {} sync: {}", binary, source))` |

- `ops` functions take `project: &ProjectRef`; the app constructs `ProjectRef::local(options.cwd)`.
- The recording invoker used by tests lives under `#[cfg(test)]` in `runner.rs` and is not public.
- Changelog lines (collated by b-10): "New crate `btit-cli`: the process transport shared by bd and br (extended PATH, per-project lock, `--json` invocation) and the shared issue-operation bodies."

## Explicit Code Samples

```rust
// crates/btit-cli/src/runner.rs
use std::sync::{Arc, Mutex, PoisonError};
use btit_beads::{error::BeadsError, gates::capabilities_for};
use btit_types::{BackendCapabilities, CliClient, CliOutput, CliProbe, CliVersion, ProjectRef};
use crate::locks::ProjectLocks;

/// What `ops` needs from a CLI: identity, cached version info and the two invocation forms.
/// Implemented by `CliRunner` and, until b-7, by the app's transitional `AppInvoker`.
pub trait CliInvoker: Send + Sync {
    fn binary(&self) -> String;
    /// Cached `(client, version)`; probes lazily like `get_cli_client_info` (cli.rs:326-365).
    fn client_info(&self) -> Option<CliProbe>;
    fn capabilities(&self) -> BackendCapabilities {
        match self.client_info() {
            Some(p) => capabilities_for(p.client, p.version),
            None => BackendCapabilities::default(),
        }
    }
    fn client(&self) -> CliClient { self.client_info().map_or(CliClient::Unknown, |p| p.client) }
    fn version(&self) -> Option<CliVersion> { self.client_info().and_then(|p| p.version) }
    /// `<binary> <command…> <args…> [--no-daemon] --json` under the project lock (execute_bd, cli.rs:523-592).
    fn run_json(&self, project: &ProjectRef, command: &str, args: &[String]) -> Result<String, BeadsError>;
    /// `<binary> <args…>` with PATH/BEADS_PATH, no `--json`, no lock.
    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError>;
}

/// One configured binary. Owned by `BdCli`/`BrCli`; the app holds one at a time.
#[derive(Debug)]
pub struct CliRunner {
    binary: String,
    locks: Arc<ProjectLocks>,
    probe: Mutex<Option<CliProbe>>,
}

impl CliRunner {
    pub fn new(binary: impl Into<String>, locks: Arc<ProjectLocks>) -> Self { /* probe: None */ }
    /// Fresh `--version` (probe_cli_binary); does not touch the cache.
    pub fn probe(&self) -> Option<CliProbe> { crate::probe::probe_cli_binary(&self.binary) }
    fn cached(&self) -> std::sync::MutexGuard<'_, Option<CliProbe>> {
        self.probe.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl CliInvoker for CliRunner {
    fn binary(&self) -> String { self.binary.clone() }
    fn client_info(&self) -> Option<CliProbe> {
        let mut cached = self.cached();
        if let Some(p) = cached.as_ref() { return Some(p.clone()); }
        let output = crate::run::probe_version_output(&self.binary).ok()?;      // spawn failure → None
        if !output.success { log_warn!("[cli_detect] Failed to get version from {}", self.binary); return None; }
        let probe = btit_beads::detect::parse_cli_probe(&output.stdout);
        match probe.version {
            Some(v) => {                                                          // cache parsed successes only (cli.rs:351-360); b-9 (B13) caches failures too
                log_info!("[cli_detect] Detected {} client v{}", btit_beads::detect::cli_client_name(probe.client), v);
                *cached = Some(probe.clone());
                Some(probe)
            }
            None => { log_warn!("[cli_detect] Could not parse version from: {}", probe.raw); None }
        }
    }
    fn run_json(&self, project: &ProjectRef, command: &str, args: &[String]) -> Result<String, BeadsError> {
        let wd = crate::run::resolve_working_dir(project, self.client())?;
        crate::run::run_json(&self.binary, self.capabilities().supports_daemon_flag, &self.locks, &wd, command, args)
    }
    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> {
        let wd = crate::run::resolve_working_dir(project, self.client())?;
        crate::run::run_raw(&self.binary, &wd, args)
    }
}
```

```rust
// crates/btit-cli/src/ops.rs (signatures; bodies are the a18c724 command bodies)
pub fn list(inv: &dyn CliInvoker, project: &ProjectRef, query: &ListQuery) -> Result<Vec<BdRawIssue>, BeadsError>;
pub fn ready(inv: &dyn CliInvoker, project: &ProjectRef) -> Result<Vec<BdRawIssue>, BeadsError>;
pub fn status(inv: &dyn CliInvoker, project: &ProjectRef) -> Result<serde_json::Value, BeadsError>;
pub fn show(inv: &dyn CliInvoker, project: &ProjectRef, id: &str) -> Result<Option<BdRawIssue>, BeadsError>;
pub fn create(inv: &dyn CliInvoker, project: &ProjectRef, payload: &CreatePayload) -> Result<BdRawIssue, BeadsError>;
pub fn update(inv: &dyn CliInvoker, project: &ProjectRef, id: &str, updates: &UpdatePayload) -> Result<Option<BdRawIssue>, BeadsError>;
pub fn close(inv: &dyn CliInvoker, project: &ProjectRef, id: &str, suggest_next: bool) -> Result<serde_json::Value, BeadsError>;
pub fn search(inv: &dyn CliInvoker, project: &ProjectRef, query: &str) -> Result<Vec<BdRawIssue>, BeadsError>;
pub fn label_add(inv: &dyn CliInvoker, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError>;
pub fn label_remove(inv: &dyn CliInvoker, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError>;
pub fn delete(inv: &dyn CliInvoker, project: &ProjectRef, id: &str, hard: bool) -> Result<(), BeadsError>;
pub fn comment_add(inv: &dyn CliInvoker, project: &ProjectRef, id: &str, content: &str) -> Result<(), BeadsError>;
pub fn dep_add(inv: &dyn CliInvoker, project: &ProjectRef, issue_id: &str, depends_on_id: &str, relation_type: Option<&str>) -> Result<(), BeadsError>;
pub fn dep_remove(inv: &dyn CliInvoker, project: &ProjectRef, issue_id: &str, depends_on_id: &str) -> Result<(), BeadsError>;
pub fn relation_types(client: CliClient) -> Vec<RelationType>;   // Br → common 7; Bd/Unknown → common + tracks, until, validates
pub fn sync(inv: &dyn CliInvoker, project: &ProjectRef, no_daemon: bool) -> Result<(), BeadsError>;
```

```rust
// crates/btit-app/src/cli.rs (transitional, deleted in b-7)
pub(crate) static PROJECT_LOCKS: LazyLock<Arc<ProjectLocks>> = LazyLock::new(|| Arc::new(ProjectLocks::default()));

pub(crate) struct AppInvoker;
impl CliInvoker for AppInvoker {
    fn binary(&self) -> String { config::get_cli_binary() }
    fn client_info(&self) -> Option<CliProbe> { get_cli_client_info().map(|(c, a, b, d)| CliProbe { client: c, version: Some((a, b, d).into()), raw: String::new() }) }
    fn run_json(&self, project: &ProjectRef, command: &str, args: &[String]) -> Result<String, BeadsError> { /* resolve_working_dir + run_json with supports_daemon_flag() and &PROJECT_LOCKS */ }
    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> { /* resolve_working_dir + run_raw */ }
}
```

## This Sprint Does Not Close

- `impl BeadsBackend`/`CliBackend` (b-5, b-6); the app's backend slot and deletion of the statics (b-7).
- `migration.rs` Dolt operations and raw invocations other than `sync` (b-7 via `DoltOperations`/`run_raw`).
- Caching of failed probes (B13, b-9).

## Acceptance Criteria

1. `cargo tree -e normal -p btit-cli --depth 1` lists exactly `btit-beads`, `btit-types`, `log`, `serde_json`; `! grep -rn 'tauri\|sc_observability' crates/btit-cli/src`.
2. `crates/btit-app/src/cli.rs` contains none of the moved functions from Exact Targets; `execute_bd` is a ≤ 10-line wrapper over `btit_cli::run_json`; no `#[tauri::command]` body in `issue_commands.rs` builds CLI args itself (`! grep -nE '"--(status|type|priority|assignee|all|limit|force|hard|suggest-next|title|description|set-labels|external-ref|estimate|design|acceptance|notes|metadata|spec-id|parent)' crates/btit-app/src/issue_commands.rs`).
3. `git diff -M origin/integrate/phase-b...HEAD` shows the op bodies as moves; deltas inside them are limited to `execute_bd(..)?` → `inv.run_json(..)?`, `Err(String)` → `BeadsError` constructors from the b-3 error table, and `context` labels; listed in the PR description.
4. The recording-invoker tests from Deliverable 9 pass and cover every `ops` function.
5. `cargo test --workspace` passes; the test-preservation gate prints nothing.
6. `pnpm tauri:dev` manual check: list, show, create, update, close, search, label add/remove, delete, dependency add/remove, sync and the debug panel behave as before; the `[bd] <binary> <args> | cwd: <dir>` log lines are unchanged in shape. Evidence (three log lines) in the PR description.
7. `cargo clippy -p btit-cli --all-targets -- -D warnings`, `cargo rustdoc -p btit-cli -- -D missing-docs` pass; `! grep -rnE 'allow\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)' crates/btit-cli/src`.
8. `crates/btit-beads/tests/api_freeze.rs` and `crates/btit-types/tests/api_freeze.rs` are byte-identical to the b-3 branch (`git diff --exit-code feature/sprint-b-3-btit-beads...HEAD -- crates/btit-beads crates/btit-types`).
9. CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-types -p btit-beads -p btit-cli`
- `cargo clippy -p btit-cli --all-targets -- -D warnings`
- `cargo rustdoc -p btit-cli -- -D missing-docs`
- `cargo test --workspace`
- `cargo check --workspace --all-targets`
- `cargo tree -e normal -p btit-cli --depth 1 --prefix none --format '{p}' | sed -E 's/ v.*//' | sort | diff - <(printf 'btit-beads\nbtit-cli\nbtit-types\nlog\nserde_json\n')`
- `git diff --exit-code feature/sprint-b-3-btit-beads...HEAD -- crates/btit-beads crates/btit-types`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate
- `pnpm tauri:dev` (manual, Acceptance Criterion 6)
