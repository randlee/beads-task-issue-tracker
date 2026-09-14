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
    rationale: "crates/btit-app/** (+ its Cargo.lock refresh) vs crates/btit-bd/**; project_uses_dolt_for's signature is pinned by crates/btit-bd/tests/api_freeze.rs, so b-8's callers are unaffected by b-10's body change"
---

# Sprint b-8 — Legacy-path and Dolt rewire

## Recommended Agent / Model

Recommended model: higher-effort (migration.rs is 1 177 lines with byte-exact error strings; `DoltOperations` and `run_raw` must reproduce every invocation).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Move the last app modules onto the backend slot: `migration.rs` (repair, migration status, Dolt migration) through `DoltOperations` and `CliBackend::run_raw`; `polling.rs` `get_beads_mtime`, `watcher.rs`, `fs_commands.rs` through `BeadsBackend::project_uses_dolt`/`capabilities()`; `updates.rs` through the release-source constants.
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
- `crates/btit-app/src/migration.rs`: `bd_repair_database` (311-430): Dolt path 330-354 → `backend.dolt()` → `doctor_fix`; SQLite test call 398-408 → `run_raw` with the same args (`list --limit=1 [--no-daemon] --json`, built from `capabilities().supports_daemon_flag`); `uses_jsonl_files()` (371) → `capabilities()`; `bd_check_needs_migration` (481-555): `get_cli_client_info()` match (500-510) → `backend.client()/version()`; `project_uses_dolt` (513) → slot; `bd_migrate_to_dolt` (569-1125): version guard 596-605 → `backend.version()`; `migrate --to-dolt --yes` 623-629, `init --prefix` 665-671/771-777, `import -i` 879-885 → `DoltOperations`; restore steps 944-950, 1003-1009, 1081-1087 → `run_raw` (OQ-7 default); `sqlite3` call 1049 unchanged; `project_uses_dolt` (200, 270, 588) → slot; `ensure_refs_migrated_v3` (14) gets its doc comment (item A4)
- `crates/btit-app/src/polling.rs:95-160`: `get_beads_mtime` → `backend.project_uses_dolt(dir)` and `capabilities().uses_jsonl_files`
- `crates/btit-app/src/watcher.rs:86`, `fs_commands.rs:50,73`: `project_uses_dolt` → slot
- `crates/btit-app/src/updates.rs:1-3,271-319`: `check_bd_cli_update` selects `btit_br::BR_RELEASE_SOURCE` / `btit_bd::BD_RELEASE_SOURCE` by `detect_cli_client(&version_str)` (unchanged decision input); imports from `btit_beads::detect`
- `Cargo.lock` (root): refresh only if the app's dependency edges change (none expected)
- `docs/plans/phase-b/sprint-b-8.md` (`status:` frontmatter and Implementation Notes)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **`bd_repair_database`.** Dolt path: `backend.dolt()` → `Some(d)` → `d.doctor_fix(&project)`; success → `RepairResult { success: true, message: format!("Database repaired via bd doctor. {}", stdout.trim()), backup_path: None }` (`migration.rs:341-348`); failure → `Err(format!("Repair failed: {}", stderr.trim()))` (`:350-352`); spawn error → `Err(format!("Failed to run bd doctor: {}", e))` (`:339`, literal `bd`); `None` (a Dolt project on a backend without Dolt, unreachable today because only `BdCli` detects Dolt) → `Err(BeadsError::Unsupported { operation: "Dolt repair", client }.to_string())`. SQLite path unchanged except the test call (`:398-408`) goes through `run_raw`.
2. **`bd_check_needs_migration` and `bd_migrate_to_dolt`.** Version gates read `backend.client()`/`backend.version()` with the same reason strings and the same `Err("Could not determine bd version")` (`:604`). `migrate --to-dolt --yes`, `init --prefix <p>`, `import -i <file>` go through `DoltOperations` with today's success/failure/spawn texts (`Failed to run bd migrate: {e}` :629, `Failed to run bd init: {e}` :671, `Failed to run bd import: {e}` :885) built from `BeadsError`/`CliOutput` fields. Restore steps (`label add`, `dep add`, `comments add`) go through `run_raw` with byte-identical argument vectors (OQ-7 default). The `sqlite3` invocation stays a direct `std::process::Command` (it is not a beads CLI call).
3. **Legacy-path detection.** `get_beads_mtime`, `start_watching`, `fs_list` call `backend::current().project_uses_dolt(..)` (and `capabilities().uses_jsonl_files` in `get_beads_mtime`) exactly where they called the wrappers.
4. **`check_bd_cli_update`.** Keeps `get_bd_version().await` and `detect_cli_client(&version_str)`; the URL pair comes from `BR_RELEASE_SOURCE` for `Br` and `BD_RELEASE_SOURCE` otherwise (`updates.rs:285-292`).
5. **`cli.rs` deleted; A4 closed.** `mod cli;` removed from `lib.rs`; `ensure_refs_migrated_v3` carries the doc comment whose text was orphaned at `cli.rs:594-595` ("Auto-run refs migration v3 (filesystem-only attachments) if needed. Called synchronously before br sync to prevent UNIQUE constraint errors.").
6. **Tests.** The five `migration.rs` tests (`reprefix_id_*`, `:1133-1175`) and the `polling.rs` test are unchanged. New unit tests through `RecordingInvoker`-backed `BdCli::with_probe` cover the argument vectors `bd_repair_database`'s SQLite test call and `bd_migrate_to_dolt`'s restore steps send (`["label","add",id,label]`, `["dep","add",a,b]`, `["comments","add",id,text]`, `:944-950,1003-1009,1081-1087`), via a `#[cfg(test)]` seam that lets the command functions take a backend argument.
7. **Manual verification** (recorded in the PR): repair on a Dolt project (doctor) and on a SQLite project (backup + recreate); `bd_check_needs_migration` on a Dolt project, a legacy SQLite project and an empty project; `bd_migrate_to_dolt` on a small SQLite project if one is available, otherwise the empty-project init path.

## Required Work

- Error-text table for this sprint (byte-exact):

  | Site | Today (`migration.rs`) | After |
  |---|---|---|
  | doctor spawn | `Failed to run bd doctor: {e}` (339) | `BeadsError::Spawn { source, .. }` → same text |
  | doctor non-zero | `Repair failed: {stderr.trim()}` (352) | `CliOutput { success: false, stderr }` → same text |
  | migrate spawn | `Failed to run bd migrate: {e}` (629) | same |
  | init spawn | `Failed to run bd init: {e}` (671) | same |
  | import spawn | `Failed to run bd import: {e}` (885) | same |
  | version unknown | `Could not determine bd version` (604) | same |

- Changelog lines (collated by b-12): "Database repair and Dolt migration run through the bd backend's `DoltOperations`; legacy-path detection (mtime, watcher, directory listing) asks the selected backend. No user-visible change."

## Explicit Code Samples

```rust
// crates/btit-app/src/migration.rs — Dolt repair path after b-8 (migration.rs:330-354 today)
let backend = backend::current();
let project = ProjectRef::local(Some(working_dir.clone()));
if backend.project_uses_dolt(&beads_dir) {
    log_info!("[bd_repair] Using Dolt-based repair strategy (bd >= 0.50.0): bd doctor --fix --yes");
    let Some(dolt) = backend.dolt() else {
        return Err(BeadsError::Unsupported { operation: "Dolt repair", client: backend.client() }.to_string());
    };
    let out = dolt.doctor_fix(&project).map_err(|e| match e {
        BeadsError::Spawn { source, .. } => format!("Failed to run bd doctor: {}", source),
        other => other.to_string(),
    })?;
    return if out.success {
        log_info!("[bd_repair] Dolt repair successful: {}", out.stdout.trim());
        Ok(RepairResult { success: true, message: format!("Database repaired via bd doctor. {}", out.stdout.trim()), backup_path: None })
    } else {
        log_error!("[bd_repair] Dolt repair failed: {}", out.stderr.trim());
        Err(format!("Repair failed: {}", out.stderr.trim()))
    };
}
```

## This Sprint Does Not Close

- B-item fixes in `polling.rs`, `updates.rs`, `attachments.rs`, `attachment_refs.rs` (b-11) and in `btit-bd` (b-10).
- Workspace lints and formatting on the app crate (b-12).

## Acceptance Criteria

1. `crates/btit-app/src/cli.rs` does not exist; `! grep -rn 'mod cli\|crate::cli' crates/btit-app/src`; `! grep -rnE 'new_command|get_extended_path' crates/btit-app/src` (every spawn of a beads CLI goes through the backend; the `sqlite3` and `open`/`xdg-open`/`cmd` spawns use `std::process::Command` directly, as today).
2. `git diff --name-only feature/sprint-b-7-backend-slot...HEAD | grep -vE '^(crates/btit-app/|Cargo.lock$|docs/plans/phase-b/sprint-b-8.md$)'` prints nothing (group B non-intersection).
3. `generate_handler!` still lists the same 65 names in order (gate from `sprint-b-7.md` Acceptance Criterion 2).
4. Every row of the error-text table is covered by a unit test on the mapping closure, or by the `RecordingInvoker` tests of Deliverable 6.
5. `ensure_refs_migrated_v3` has a `///` doc comment; `cargo rustdoc -p beads-issue-tracker` emits no `missing_docs` warning for it (the app does not deny missing docs; the check is `grep -B3 'fn ensure_refs_migrated_v3' crates/btit-app/src/migration.rs | grep -c '^///'` ≥ 1).
6. `cargo test --workspace` passes; the test-preservation gate (with b-9's list) prints nothing.
7. Manual verification (Deliverable 7) recorded in the PR.
8. QA-1 complete; CI green; every command in Required Validation passes.

## Required Validation

- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --manifest-path crates/btit-app/Cargo.toml --all-targets` (warnings allowed until b-12; no errors)
- `git diff --name-only feature/sprint-b-7-backend-slot...HEAD | grep -vE '^(crates/btit-app/|Cargo.lock$|docs/plans/phase-b/sprint-b-8.md$)'` prints nothing
- `pnpm test`
- `npx vue-tsc --noEmit`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate
- `pnpm tauri:dev` (manual, Deliverable 7)
