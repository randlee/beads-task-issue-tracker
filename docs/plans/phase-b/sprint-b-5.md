---
id: b-5
title: btit-bd crate — BdCli (BeadsBackend, CliBackend, DoltOperations)
status: planned
branch: feature/sprint-b-5-btit-bd
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-5-btit-bd
target: integrate/phase-b
recommended_model: standard (thin implementation over btit-cli; Dolt detection moves with its tests)
dependency_relations:
  - prerequisite: b-4
    dependent: b-5
    relation: must_follow
    rationale: "BdCli wraps CliRunner, delegates to btit_cli::ops, tests with testing::RecordingInvoker, and fills the btit-bd skeleton b-4 registered; group A, forked from the b-4 head"
  - prerequisite: b-5
    dependent: b-7
    relation: must_follow
    rationale: "the app constructs BdCli; b-7 is the join layer of group A"
  - prerequisite: b-5
    dependent: b-10
    relation: must_follow
    rationale: "b-10 redesigns project_uses_dolt_for inside crates/btit-bd (content dependency; b-10 forks from the b-7 head)"
  - prerequisite: none
    parallel_pair: [b-5, b-6]
    relation: parallel_safe
    rationale: "crates/btit-bd/** vs crates/btit-br/**; root Cargo.toml, Cargo.lock, ci.yml and the recording invoker were written by b-4, so neither sprint touches a shared file"
  - prerequisite: none
    parallel_pair: [b-5, b-9]
    relation: parallel_safe
    rationale: "crates/btit-bd/** vs crates/btit-beads/** behind the frozen btit-beads API"
---

# Sprint b-5 — `btit-bd`: `BdCli`

## Recommended Agent / Model

Recommended model: standard (thin implementation over btit-cli; Dolt detection moves with its tests).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Fill the `crates/btit-bd` skeleton with `BdCli`, the backend for the Go `bd` CLI (and for an unrecognized client, which today shares every `_ =>` arm with bd), implementing `BeadsBackend`, `CliBackend` and `DoltOperations`.
- Move `project_uses_dolt_for` and its tests here; this is the only backend that can say yes.

## Hard Dependencies

- b-4 closure criteria met and QA-1 without Blocking finding (`CliRunner`, `CliInvoker`, `ops`, `testing::RecordingInvoker`, the `btit-bd` skeleton). This branch is forked from the `feature/sprint-b-4-btit-cli` head.

## Dependency Relations

Trigger definitions, per-branch QA and fix-layer rules: `plan-phase-b.md` "Dependency relations" and "Parallel groups: fork and re-merge".

- b-4 → b-5 — `must_follow`.
- b-5 → b-7 — `must_follow` (b-7 is group A's join layer).
- b-5 → b-10 — `must_follow` (content).
- b-5 ↔ b-6, b-5 ↔ b-9 — `parallel_safe` (see frontmatter).

Stack: group A · layer 5 (b-5 | b-6 | b-9, first to close). If this sprint closes first it is linked as layer 5; otherwise it is merged into the b-7 branch when it closes.

## Exact Targets

Line numbers are at `a18c724` (`crates/btit-app/src/` after b-1).

- `crates/btit-bd/src/lib.rs` (module declarations and re-exports), `src/backend.rs` (`BdCli`), `src/dolt.rs` (`project_uses_dolt_for`, `DoltOperations` arg builders), `tests/api_freeze.rs`
- Moved out of `crates/btit-app/src/cli.rs` **as a copy plus test move**: `project_uses_dolt_for` (482-515) is copied here; its tests 1239-1302 (`project_uses_dolt_false_without_beads_dir`, `dolt_tmp`, `BD_1`, `project_uses_dolt_for_legacy_dolt_dir`, `project_uses_dolt_for_nested_layout_needs_metadata_and_dolt_dir`, `project_uses_dolt_for_sqlite_metadata_or_empty_dir_is_false`, `project_uses_dolt_for_br_and_legacy_bd_never_true`) move here. The app's copy of the function stays in `cli.rs` untouched until b-7 deletes it, so this sprint edits nothing under `crates/btit-app/` (group A non-intersection).
- `docs/plans/phase-b/sprint-b-5.md` (`status:` frontmatter only)

No edit to the root `Cargo.toml`, `Cargo.lock`, `crates/btit-bd/Cargo.toml`, `crates/btit-bd/clippy.toml` or `.github/workflows/ci.yml` (all written by b-4).

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **`BdCli`** as in the code sample: `BdCli::new(binary, locks)`. `BeadsBackend` methods delegate to `btit_cli::ops` with the bd choices: `close` → `suggest_next: false` (`issue_commands.rs:411-413`, the non-br branch), `delete` → `hard: capabilities().supports_delete_hard_flag` (`:471-473`), `relation_types` → `ops::relation_types(self.client())` (bd and unknown get the extra three, `:571-578`), `sync` → `ops::sync(.., capabilities().supports_daemon_flag)` (`migration.rs:224-226`), `project_uses_dolt` → `project_uses_dolt_for(self.info_tuple(), beads_dir)`, `dolt()` → `Some(self)`, `close_suggestions()` → `None`.
2. **`CliBackend`.** `binary()`, `probe()` (fresh), `run_raw()` via the runner, `release_source()` → `BD_RELEASE_SOURCE` (`https://api.github.com/repos/steveyegge/beads/releases/latest`, `https://github.com/steveyegge/beads/releases`, `updates.rs:287,291`), exported as a `pub const` for the app's `check_bd_cli_update` selection (b-8).
3. **`DoltOperations`.** Each method is `run_raw` with a fixed argument vector built by a pure `pub(crate) fn` in `dolt.rs`: `doctor_fix_args() = ["doctor", "--fix", "--yes"]` (`migration.rs:334`), `migrate_to_dolt_args() = ["migrate", "--to-dolt", "--yes"]` (`:624`), `init_args(prefix) = ["init", "--prefix", prefix]` (`:666,772`), `import_args(file) = ["import", "-i", file]` (`:880`). Unit tests assert these vectors.
4. **`project_uses_dolt_for` copied verbatim** (`cli.rs:482-515`) with its seven tests moved. `project_uses_dolt_false_without_beads_dir` (`cli.rs:1239-1248`) calls the wrapper today; here it calls `BdCli::new("bd", locks).project_uses_dolt(&dir)`, which still spawns `bd --version` (B10 is fixed in b-10, not here).
5. **Unit tests** through `BdCli::with_probe(probe)` (`#[cfg(test)]`, built on `CliRunner::with_probe`) and `RecordingInvoker`: `relation_types()` (exact ten entries in order), `release_source()`, `dolt().is_some()`, `close_suggestions().is_none()`, `capabilities()` for `(Bd, 0.49.6)`, `(Bd, 0.55.0)`, `(Bd, 1.0.4)`, `(Unknown, None)`, and `close` issuing `close <id>` without `--suggest-next`.
6. **API freeze.** `crates/btit-bd/tests/api_freeze.rs` pins `BdCli::new`, `BD_RELEASE_SOURCE`, `project_uses_dolt_for` and `fn _obj(b: &BdCli) -> &dyn CliBackend { b }`.

## Required Work

- `info_tuple()` is a private helper turning the runner's `client_info()` into `Option<(CliClient, u32, u32, u32)>`, the signature `project_uses_dolt_for` keeps so its tests move unchanged.
- The `ops` calls that need a `&dyn CliInvoker` receive `&self.runner`.
- Changelog lines (collated by b-12): "New crate `btit-bd`: the `bd` backend (`BeadsBackend`, `CliBackend`, `DoltOperations`)."

## Explicit Code Samples

```rust
// crates/btit-bd/src/backend.rs
use std::{path::Path, sync::Arc};
use btit_beads::{backend::{BeadsBackend, CliBackend, CloseSuggestions, DoltOperations}, error::BeadsError};
use btit_cli::{locks::ProjectLocks, ops, runner::{CliInvoker, CliRunner}};
use btit_types::*;

/// GitHub location of bd releases (updates.rs:287,291).
pub const BD_RELEASE_SOURCE: ReleaseSource = ReleaseSource {
    api_url: "https://api.github.com/repos/steveyegge/beads/releases/latest",
    releases_url: "https://github.com/steveyegge/beads/releases",
};

/// The Go `bd` CLI (also used for an unrecognized client, which shares bd's defaults today).
#[derive(Debug)]
pub struct BdCli { runner: CliRunner }

impl BdCli {
    pub fn new(binary: impl Into<String>, locks: Arc<ProjectLocks>) -> Self { Self { runner: CliRunner::new(binary, locks) } }
    fn info_tuple(&self) -> Option<(CliClient, u32, u32, u32)> {
        self.runner.client_info().and_then(|p| p.version.map(|v| (p.client, v.major, v.minor, v.patch)))
    }
}

impl BeadsBackend for BdCli {
    fn client(&self) -> CliClient { self.runner.client() }
    fn version(&self) -> Option<CliVersion> { self.runner.version() }
    fn capabilities(&self) -> BackendCapabilities { self.runner.capabilities() }
    fn project_uses_dolt(&self, beads_dir: &Path) -> bool { crate::dolt::project_uses_dolt_for(self.info_tuple(), beads_dir) }
    fn list(&self, p: &ProjectRef, q: &ListQuery) -> Result<Vec<BdRawIssue>, BeadsError> { ops::list(&self.runner, p, q) }
    // ready, status, show, create, update, search, label_add, label_remove, comment_add, dep_add, dep_remove: direct delegation
    fn close(&self, p: &ProjectRef, id: &str) -> Result<serde_json::Value, BeadsError> { ops::close(&self.runner, p, id, false) }
    fn delete(&self, p: &ProjectRef, id: &str) -> Result<(), BeadsError> { ops::delete(&self.runner, p, id, self.capabilities().supports_delete_hard_flag) }
    fn relation_types(&self) -> Vec<RelationType> { ops::relation_types(self.client()) }
    fn sync(&self, p: &ProjectRef) -> Result<(), BeadsError> { ops::sync(&self.runner, p, self.capabilities().supports_daemon_flag) }
    fn dolt(&self) -> Option<&dyn DoltOperations> { Some(self) }
}

impl CliBackend for BdCli {
    fn binary(&self) -> &str { self.runner.binary_ref() }
    fn probe(&self) -> Option<CliProbe> { self.runner.probe() }
    fn run_raw(&self, p: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> { self.runner.run_raw(p, args) }
    fn release_source(&self) -> ReleaseSource { BD_RELEASE_SOURCE }
}

impl DoltOperations for BdCli {
    fn doctor_fix(&self, p: &ProjectRef) -> Result<CliOutput, BeadsError> { self.runner.run_raw(p, &crate::dolt::doctor_fix_args()) }
    fn migrate_to_dolt(&self, p: &ProjectRef) -> Result<CliOutput, BeadsError> { self.runner.run_raw(p, &crate::dolt::migrate_to_dolt_args()) }
    fn init(&self, p: &ProjectRef, prefix: &str) -> Result<CliOutput, BeadsError> { self.runner.run_raw(p, &crate::dolt::init_args(prefix)) }
    fn import_jsonl(&self, p: &ProjectRef, file: &Path) -> Result<CliOutput, BeadsError> { /* to_string_lossy, then run_raw(import_args(..)) */ }
}
```

## This Sprint Does Not Close

- App construction of `BdCli` and deletion of the app's `project_uses_dolt_for` copy (b-7).
- B3 (Dolt detection rules), B10 (wrapper-calling test), B13 (test rename) in this crate (b-10).

## Acceptance Criteria

1. `git diff --name-only feature/sprint-b-4-btit-cli...HEAD | grep -vE '^(crates/btit-bd/|docs/plans/phase-b/sprint-b-5.md$)'` prints nothing (group A non-intersection).
2. `cargo tree -e normal -p btit-bd --depth 1` lists exactly `btit-beads`, `btit-cli`, `btit-types`, `log`, `serde_json`; `! grep -rnE '^\s*(pub(\(crate\))? )?static ' crates/btit-bd/src` (instances only; two `BdCli` for two projects may coexist).
3. `BdCli` implements `BeadsBackend`, `CliBackend`, `DoltOperations` (pinned by `tests/api_freeze.rs`); it does not implement `CloseSuggestions`.
4. `project_uses_dolt_for` and its seven tests exist in `crates/btit-bd`; the seven tests no longer exist in `crates/btit-app/src/cli.rs` (the function body there is unchanged).
5. The argument-vector tests (Deliverable 3) and the behaviour tests (Deliverable 5) pass.
6. `cargo test --workspace` passes; test-preservation gate prints nothing.
7. `cargo clippy -p btit-bd --all-targets -- -D warnings`, `cargo rustdoc -p btit-bd -- -D missing-docs` pass; no `allow(clippy::…)` for the deny set in `src`.
8. QA-1 for this branch complete (per-branch rule); CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-bd`
- `cargo clippy -p btit-bd --all-targets -- -D warnings`
- `cargo rustdoc -p btit-bd -- -D missing-docs`
- `cargo test --workspace`
- `cargo check --workspace --all-targets`
- `cargo tree -e normal -p btit-bd --depth 1 --prefix none --format '{p}' | sed -E 's/ v.*//' | sort | diff - <(printf 'btit-bd\nbtit-beads\nbtit-cli\nbtit-types\nlog\nserde_json\n')`
- `git diff --name-only feature/sprint-b-4-btit-cli...HEAD | grep -vE '^(crates/btit-bd/|docs/plans/phase-b/sprint-b-5.md$)'` prints nothing
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate
