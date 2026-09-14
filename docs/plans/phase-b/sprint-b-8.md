---
id: b-8
title: Legacy-path and Dolt rewire — migration, mtime, watcher, fs, updates on the slot; delete cli.rs
status: planned
branch: feature/sprint-b-8-legacy-dolt-rewire
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-8-legacy-dolt-rewire
target: integrate/phase-b
recommended_model: higher-effort (migration.rs is 1 177 lines with byte-exact error strings; DoltOperations and run_raw must reproduce every invocation)
dependency_relations:
  - prerequisite: b-7
    dependent: b-8
    relation: must_follow
    rationale: "rewires the remaining app modules onto the slot b-7 introduced and deletes the shims b-7 left; group B, forked from the b-7 head"
  - prerequisite: b-8
    dependent: b-11
    relation: must_follow
    rationale: "b-11 edits app modules this sprint rewires (polling.rs, updates.rs, attachments.rs, attachment_refs.rs); b-11 is the join layer of group B"
  - prerequisite: none
    parallel_pair: [b-8, b-10]
    relation: parallel_safe
    rationale: "crates/btit-app/** vs crates/btit-bd/** (neither touches Cargo.lock); project_uses_dolt_for's signature is pinned by crates/btit-bd/tests/api_freeze.rs, so b-8's callers are unaffected by b-10's body change"
---

# Sprint b-8 — Legacy-path and Dolt rewire

## Recommended Agent / Model

Recommended model: higher-effort (migration.rs is 1 177 lines with byte-exact error strings; `DoltOperations` and `run_raw` must reproduce every invocation).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Move the last app modules onto the backend slot: `migration.rs` (repair, migration status, Dolt migration) through `DoltOperations` (transport-neutral `DoltOpResult`) and, for the byte-identical restore/test invocations, `CliBackend::run_raw` reached through `BeadsBackend::cli()`; `polling.rs` `get_beads_mtime`, `watcher.rs`, `fs_commands.rs` through `BeadsBackend::project_uses_dolt`/`capabilities()`; `updates.rs` through the release-source constants.
- Delete `cli.rs` (the four b-7 shims) and close refactor item A4's doc comment.

## Hard Dependencies

- b-7 closure criteria met and QA-1 without Blocking finding. This branch is forked from the `feature/sprint-b-7-backend-slot` head.

## Dependency Relations

Trigger definitions, per-branch QA and fix-layer rules: `plan-phase-b.md` "Dependency relations" and "Parallel groups: fork and re-merge".

- b-7 → b-8 — `must_follow`.
- b-8 → b-11 — `must_follow` (b-11 is group B's join layer).
- b-8 ↔ b-10 — `parallel_safe`.

Stack: group B · layer 7 (b-8 | b-10, first to close). If this sprint closes first it is linked as layer 7; otherwise it is merged into the b-11 branch when it closes.

## Exact Targets

Line numbers are at `a18c724` (`crates/btit-app/src/` after b-1).

- `crates/btit-app/src/cli.rs`: deleted; `crates/btit-app/src/lib.rs`: `mod cli;` removed
- `crates/btit-app/src/migration.rs` (state left by b-7: the import line takes `new_command`/`get_extended_path` from `btit-cli` (b-4) and `crate::cli::{client_info, project_uses_dolt, supports_daemon_flag, uses_jsonl_files}` shims; `get_cli_client_info()` at 500-510 and 596-605 already reads the `client_info()` shim): `bd_repair_database` (311-430): Dolt path 330-354 → `be.dolt()` → `doctor_fix`; SQLite test call 398-408 → `cli_of(be, ..)?.run_raw` with the same args (`list --limit=1 [--no-daemon] --json`, built from `cli_of(be, ..)?.capabilities().supports_daemon_flag`); `uses_jsonl_files()` shim (371) → `cli_of(be, ..)` capabilities; `bd_check_needs_migration` (481-555): the `client_info()` shim (500-510) → `cli_of(be, ..)` `client()`/`version()`; `project_uses_dolt` (513) → `be.project_uses_dolt(&project)`; `bd_migrate_to_dolt` (569-1125): version guard 596-605 → `cli_of(be, ..)` `version()`; `migrate --to-dolt --yes` 623-629, `init --prefix` 665-671/771-777, `import -i` 879-885 → `DoltOperations`; restore steps 944-950, 1003-1009, 1081-1087 → `run_raw` (OQ-7 default); `sqlite3` call 1049 unchanged; `project_uses_dolt` (200, 270, 588) → slot; `ensure_refs_migrated_v3` (14) gets its doc comment (item A4)
- `crates/btit-app/src/polling.rs:95-160`: `get_beads_mtime` → `backend.project_uses_dolt(dir)` and `capabilities().uses_jsonl_files`
- `crates/btit-app/src/watcher.rs:86`, `fs_commands.rs:50,73`: `project_uses_dolt` → slot
- `crates/btit-app/src/updates.rs:1-3,271-319`: `check_bd_cli_update` selects `btit_br::BR_RELEASE_SOURCE` / `btit_bd::BD_RELEASE_SOURCE` by `detect_cli_client(&version_str)` (unchanged decision input); imports from `btit_beads::detect`
- `crates/btit-app/Cargo.toml`: `[dev-dependencies] btit-bd = { path = "../btit-bd", features = ["test-support"] }`, `btit-br = { …, features = ["test-support"] }`, `btit-cli = { …, features = ["test-support"] }` (test-only; normal dependency edges unchanged)
- `Cargo.lock` (root): not touched (dev-dependency features add no package; `btit-bd`/`btit-br` were added in b-7)
- `docs/plans/phase-b/sprint-b-8.md` (`status:` frontmatter and Implementation Notes)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **`bd_repair_database`.** Dolt path: `be.dolt()` → `Some(d)` → `d.doctor_fix(&project)` returning `DoltOpResult`; `success` → `RepairResult { success: true, message: format!("Database repaired via bd doctor. {}", r.message), backup_path: None }` (`migration.rs:341-348`; `message` is the trimmed stdout); `!success` → `Err(format!("Repair failed: {}", r.detail))` (`:350-352`; `detail` is the trimmed stderr); spawn error → `Err(format!("Failed to run bd doctor: {}", e))` (`:339`, literal `bd`); `None` (a Dolt project on a backend without Dolt, unreachable today because only `BdCli` detects Dolt) → `Err(BeadsError::Unsupported { operation: "Dolt repair", client }.to_string())`. SQLite path unchanged except the test call (`:398-408`) goes through `cli_of(be, "repair test")?.run_raw(..)` (the `no_daemon` flag from that `CliBackend`'s `capabilities()`).
2. **`bd_check_needs_migration` and `bd_migrate_to_dolt`.** Version gates read `cli_of(be, ..)` → `(c.client(), c.version())` (a non-CLI backend yields `Unsupported`, mapped to the same `reason`/`Err` texts as a missing version) with the same reason strings and the same `Err("Could not determine bd version")` (`:604`). `migrate --to-dolt --yes`, `init --prefix <p>`, `import -i <file>` go through `DoltOperations` (`DoltOpResult`: `message` = trimmed stdout for the success texts at `:633-636,903-904`, `detail` = trimmed stderr for `:642-643,680-682,780-781,891-900`) with today's spawn texts (`Failed to run bd migrate: {e}` :629, `Failed to run bd init: {e}` :671, `Failed to run bd import: {e}` :885) built from `BeadsError::Spawn`. Restore steps (`update <id> --set-labels …` per label `:937-942`, `dep add <id> <dep> --type <t>` `:1004`, `comments add <id> -f <file> --author <a>` `:1082`) go through `cli_of(be, ..)?.run_raw(..)` with byte-identical argument vectors (OQ-7 default). The `sqlite3` invocation stays a direct `std::process::Command` (it is not a beads CLI call).
3. **Legacy-path detection.** `get_beads_mtime_with(be, ..)`, `start_watching`, `fs_list` call `be.project_uses_dolt(&ProjectRef::local(Some(working_dir)))` (and `cli_of(be, ..).map(|c| c.capabilities().uses_jsonl_files).unwrap_or(false)` in `get_beads_mtime_with`) exactly where they called the wrappers; the two Tauri commands without a `_with` core (`start_watching`, `fs_list`) call `backend::current()` once at their top; `get_beads_mtime(beads_dir)` is a non-command wrapper (`polling.rs:95`, called by `bd_poll_data` and `bd_check_changed`) that calls `backend::current()` once and delegates to `get_beads_mtime_with(be, beads_dir)`, exactly as `sync_bd_database` (`migration.rs:188`) is a non-command wrapper over `be.sync`. The b-7 shims are deleted with `cli.rs`.
4. **`check_bd_cli_update`.** Keeps `get_bd_version().await` and `detect_cli_client(&version_str)`; the URL pair comes from `BR_RELEASE_SOURCE` for `Br` and `BD_RELEASE_SOURCE` otherwise (`updates.rs:285-292`).
5. **`cli.rs` deleted; A4 closed.** `mod cli;` removed from `lib.rs`; `ensure_refs_migrated_v3` carries the doc comment whose text was orphaned at `cli.rs:594-595` ("Auto-run refs migration v3 (filesystem-only attachments) if needed. Called synchronously before br sync to prevent UNIQUE constraint errors.").
6. **Tests.** The five `migration.rs` tests (`reprefix_id_*`, `:1133-1175`) and the `polling.rs` test are unchanged. Each rewired command gets a `<cmd>_with(be: &dyn BeadsBackend, ..)` core holding the body (`bd_repair_database_with`, `bd_check_needs_migration_with`, `bd_migrate_to_dolt_with`, `get_beads_mtime_with`), with the `#[tauri::command]` fn (or, for `get_beads_mtime`, the non-command wrapper) a one-line wrapper passing `backend::current().as_ref()`. Inside a `_with` core every CLI access goes through the parameter, including the binary name — `cli_of(be, ..)?.binary()` replaces every `get_cli_binary()` read (`migration.rs:332,403,622`, imported at `:5`), and the `use crate::config::get_cli_binary` import is removed from `migration.rs`: a local helper `fn cli_of<'a>(be: &'a dyn BeadsBackend, operation: &'static str) -> Result<&'a dyn CliBackend, BeadsError>` (`be.cli().ok_or(BeadsError::Unsupported { operation, client: CliClient::Unknown })`) replaces `backend::with_cli`, and no `_with` body names `backend::` at all — otherwise the injected `RecordingInvoker`-backed backend would be bypassed by the global slot. New unit tests in `crates/btit-app` build `BdCli::with_invoker(Box::new(RecordingInvoker::new(probe)))` through the `[dev-dependencies]` `test-support` features. They use only rule-invariant fixtures, so b-10's rewrite of `project_uses_dolt` (B3) changes none of their outcomes and b-10 edits no test in `btit-app`: a Dolt project is a `tempdir` with `.beads/.dolt/` (directory) and no `.beads/metadata.json`; a SQLite project has `.beads/beads.db` (plus `.beads/issues.jsonl` where the body reads it) and neither `.beads/metadata.json` nor `.beads/.dolt/`; the probe is `Bd 1.0.4` or `Bd 0.49.6` only (never `Br`, `Unknown` or a `None` version, whose answers b-10 changes). They assert the argument vectors the cores send: the SQLite repair test call (`["list", "--limit=1", "--json"]`, `:398-402`), the restore steps with their exact `a18c724` vectors — labels: `["update", id, "--set-labels", l1, "--set-labels", l2]` (one `--set-labels` per label, `migration.rs:937-942`); dependencies: `["dep", "add", id, dep, "--type", dep_type]` (`:1004`); comments: `["comments", "add", id, "-f", <comment_file>, "--author", author]` (`:1082`), and the `DoltOperations` calls' `DoltOpResult` mapping into `RepairResult`/`MigrateResult` texts.
7. **Manual verification** (recorded in the PR): repair on a Dolt project (doctor); `bd_check_needs_migration` on a Dolt project and on an empty project; `bd_migrate_to_dolt`'s empty-project init path (`:645-684`). The SQLite paths are unit-tested only (Deliverable 6): the installed `bd` is 1.0.4 (SQLite storage was removed in 0.51.0, `docs/crate-split-refactor-issues.md` B5) and the repository holds no SQLite `.beads` fixture, so no manual SQLite run is possible here.

## Required Work

- Error-text table for this sprint (byte-exact):

  | Site | Today (`migration.rs`) | After |
  |---|---|---|
  | doctor spawn | `Failed to run bd doctor: {e}` (339) | `BeadsError::Spawn { source, .. }` → same text |
  | doctor non-zero | `Repair failed: {stderr.trim()}` (352) | `DoltOpResult { success: false, detail, .. }` → `Repair failed: {detail}` (same text) |
  | migrate spawn | `Failed to run bd migrate: {e}` (629) | same |
  | init spawn | `Failed to run bd init: {e}` (671) | same |
  | import spawn | `Failed to run bd import: {e}` (885) | same |
  | version unknown | `Could not determine bd version` (604) | same |

- Changelog lines (collated by b-12): "Database repair and Dolt migration run through the bd backend's `DoltOperations`; legacy-path detection (mtime, watcher, directory listing) asks the selected backend. No user-visible change."

## Explicit Code Samples

```rust
// crates/btit-app/src/migration.rs — Dolt repair path after b-8 (migration.rs:330-354 today)
#[tauri::command]
pub(crate) async fn bd_repair_database(cwd: Option<String>) -> Result<RepairResult, String> {
    bd_repair_database_with(backend::current().as_ref(), cwd)
}
fn cli_of<'a>(be: &'a dyn BeadsBackend, operation: &'static str) -> Result<&'a dyn CliBackend, BeadsError> {
    be.cli().ok_or(BeadsError::Unsupported { operation, client: CliClient::Unknown })
}

pub(crate) fn bd_repair_database_with(be: &dyn BeadsBackend, cwd: Option<String>) -> Result<RepairResult, String> {
    // … working_dir / beads_dir as today (migration.rs:312-327) …
    let project = ProjectRef::local(Some(working_dir.clone()));
    if be.project_uses_dolt(&project) {
        log_info!("[bd_repair] Using Dolt-based repair strategy (bd >= 0.50.0): bd doctor --fix --yes");
        let Some(dolt) = be.dolt() else {
            let client = be.cli().map_or(CliClient::Unknown, |c| c.client());
            return Err(BeadsError::Unsupported { operation: "Dolt repair", client }.to_string());
        };
        let r: DoltOpResult = dolt.doctor_fix(&project).map_err(|e| match e {
            BeadsError::Spawn { source, .. } => format!("Failed to run bd doctor: {}", source),
            other => other.to_string(),
        })?;
        return if r.success {
            log_info!("[bd_repair] Dolt repair successful: {}", r.message);
            Ok(RepairResult { success: true, message: format!("Database repaired via bd doctor. {}", r.message), backup_path: None })
        } else {
            log_error!("[bd_repair] Dolt repair failed: {}", r.detail);
            Err(format!("Repair failed: {}", r.detail))
        };
    }
    // … SQLite path as today; the recreate test call: cli_of(be, "repair test").map_err(|e| e.to_string())?.run_raw(&project, &args) …
}
```

## This Sprint Does Not Close

- B-item fixes in `polling.rs`, `updates.rs`, `attachments.rs`, `attachment_refs.rs` (b-11) and in `btit-bd` (b-10).
- Workspace lints and formatting on the app crate (b-12).
- A manual run of the SQLite repair and SQLite→Dolt migration paths: no SQLite project can be produced with the installed `bd` 1.0.4 and none is checked in; these paths are covered by the `RecordingInvoker` tests of Deliverable 6 only.

## Acceptance Criteria

1. `crates/btit-app/src/cli.rs` does not exist; `! grep -rnE 'mod cli;|crate::cli::|execute_bd|btit_cli::run::run_json' crates/btit-app/src` (every beads CLI invocation goes through the backend; the non-beads spawns keep `btit_cli::command::new_command` for `gh auth token` (`updates.rs:75`) and `cmd /C start` (`attachments.rs:50`) so `CREATE_NO_WINDOW` is preserved, and `sqlite3`/`open`/`xdg-open` use `std::process::Command` directly, as today).
2. `git diff --name-only feature/sprint-b-7-backend-slot...HEAD | grep -vE '^(crates/btit-app/|docs/plans/phase-b/sprint-b-8.md$)'` prints nothing (group B non-intersection). Injection discipline: (a) `awk '/fn [a-z_]+_with\(/{c=1} c&&/(backend::|\bcurrent\(\)|with_cli\(|get_cli_binary\()/{print FILENAME":"FNR": "$0} c&&/^}/{c=0}' crates/btit-app/src/*.rs` prints nothing (no `_with` body reaches the global slot, directly or through `config::get_cli_binary()`); (b) `! grep -nE 'use crate::backend::\{|use crate::backend::(current|with_cli)|use crate::config::get_cli_binary' crates/btit-app/src/{migration,polling}.rs` (so a bare `current()`/`get_cli_binary()` cannot appear); (c) the only functions in `migration.rs` and `polling.rs` that call `backend::current()` are on the allow-list `bd_sync`, `bd_repair_database`, `bd_cleanup_stale_locks`, `bd_check_needs_migration`, `bd_migrate_to_dolt`, `bd_check_changed`, `bd_reset_mtime`, `bd_poll_data` (command wrappers) plus the two non-command wrappers `sync_bd_database` (`migration.rs:188`) and `get_beads_mtime` (`polling.rs:95`): `awk '/^(pub(\(crate\))? )?(async )?fn [a-z_]+/{name=$0; sub(/.*fn /,"",name); sub(/[(<].*/,"",name)} /backend::current\(\)/{print name}' crates/btit-app/src/{migration,polling}.rs | sort -u | diff - <(printf 'bd_check_changed\nbd_check_needs_migration\nbd_cleanup_stale_locks\nbd_migrate_to_dolt\nbd_poll_data\nbd_repair_database\nbd_reset_mtime\nbd_sync\nget_beads_mtime\nsync_bd_database\n' | sort)` is empty (names that do not use the slot simply do not appear; the diff allows only a subset).
3a. Test-only feature isolation: `cargo tree -e normal,features -p beads-issue-tracker | grep -c 'test-support'` is `0` (the `test-support` features are enabled only through `[dev-dependencies]`, never in the normal build).
3. `generate_handler!` still lists the same 65 names in order (gate from `sprint-b-7.md` Acceptance Criterion 2); the plan's command-signature gate (1) diffs empty (the `_with` cores are new free functions; the `#[tauri::command]` headers are unchanged) and the frontend invoke-subset gate (2) prints nothing.
4. Every row of the error-text table is covered by a unit test on the mapping closure, or by the `RecordingInvoker` tests of Deliverable 6.
5. `ensure_refs_migrated_v3` has a `///` doc comment; `cargo rustdoc -p beads-issue-tracker` emits no `missing_docs` warning for it (the app does not deny missing docs; the check is `grep -B3 'fn ensure_refs_migrated_v3' crates/btit-app/src/migration.rs | grep -c '^///'` ≥ 1).
6. `cargo test --workspace` passes; the test-preservation gate (with b-9's list) prints nothing.
7. Manual verification (Deliverable 7) recorded in the PR.
8. QA-1 complete; CI green; every command in Required Validation passes.

## Required Validation

- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --manifest-path crates/btit-app/Cargo.toml --all-targets` (warnings allowed until b-12; no errors)
- `git diff --name-only feature/sprint-b-7-backend-slot...HEAD | grep -vE '^(crates/btit-app/|docs/plans/phase-b/sprint-b-8.md$)'` prints nothing
- `cargo test -p beads-issue-tracker` (the `test-support` dev-dependency features are enabled for the app's own tests)
- command-signature gate (1) and frontend invoke-subset gate (2) from `plan-phase-b.md` "Command contract gates"
- `pnpm test`
- `npx vue-tsc --noEmit`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate
- `pnpm tauri:dev` (manual, Deliverable 7)
