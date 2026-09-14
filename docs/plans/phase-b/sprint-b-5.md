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
    rationale: "BdCli wraps CliRunner and delegates to btit_cli::ops; stack parent"
  - prerequisite: b-5
    dependent: b-6
    relation: must_follow
    rationale: "stack parent only (linearity); b-6 has no content dependency on b-5"
  - prerequisite: b-5
    dependent: b-7
    relation: must_follow
    rationale: "the app constructs BdCli"
  - prerequisite: none
    parallel_pair: [b-5, b-6]
    relation: parallel_safe
    rationale: "disjoint crates btit-bd / btit-br; each adds one member line to the root Cargo.toml, resolved in the b-6 rebase"
  - prerequisite: none
    parallel_pair: [b-5, b-8]
    relation: parallel_safe
    rationale: "b-5 owns crates/btit-bd/**; b-8 owns crates/btit-beads/** behind the frozen API"
---

# Sprint b-5 — `btit-bd`: `BdCli`

## Recommended Agent / Model

Recommended model: standard (thin implementation over btit-cli; Dolt detection moves with its tests).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Create `crates/btit-bd` with `BdCli`, the backend for the Go `bd` CLI (and for an unrecognized client, which today shares every `_ =>` arm with bd), implementing `BeadsBackend`, `CliBackend` and `DoltOperations`.
- Move `project_uses_dolt_for` and its tests here; this is the only backend that can say yes.

## Hard Dependencies

- b-4 pushed (`CliRunner`, `CliInvoker`, `ops`).

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA; merge parent → child before every dev/fix round. PR-completion trigger: parent PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- b-4 → b-5 — `must_follow`.
- b-5 → b-6 — `must_follow` (stack parent only).
- b-5 → b-7 — `must_follow`.
- b-5 ↔ b-6, b-5 ↔ b-8 — `parallel_safe` (see frontmatter).

Stack: `phase-b-core` · layer 5.

## Exact Targets

Line numbers are at `a18c724` (`crates/btit-app/src/` after b-1).

- `Cargo.toml` (root): member `crates/btit-bd`
- `crates/btit-bd/Cargo.toml`, `clippy.toml`, `src/lib.rs`, `src/backend.rs` (`BdCli`), `src/dolt.rs` (`project_uses_dolt_for`, `DoltOperations` arg builders), `tests/api_freeze.rs`
- Moved out of `crates/btit-app/src/cli.rs`: `project_uses_dolt_for` (482-515) and tests 1239-1302 (`project_uses_dolt_false_without_beads_dir`, `dolt_tmp`, `BD_1`, `project_uses_dolt_for_legacy_dolt_dir`, `project_uses_dolt_for_nested_layout_needs_metadata_and_dolt_dir`, `project_uses_dolt_for_sqlite_metadata_or_empty_dir_is_false`, `project_uses_dolt_for_br_and_legacy_bd_never_true`); the app's `project_uses_dolt(beads_dir)` wrapper (473-475) calls `btit_bd::dolt::project_uses_dolt_for(get_cli_client_info(), beads_dir)` until b-7
- `.github/workflows/ci.yml`: `rust-quality` gains `btit-bd`
- `docs/plans/phase-b/sprint-b-5.md` (`status:` frontmatter only)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Crate.** `crates/btit-bd`, `[lints] workspace = true`, `#![deny(missing_docs)]`, `publish = false`. Dependencies: `btit-types`, `btit-beads`, `btit-cli`, `serde_json`, `log`. Never `btit-br`.
2. **`BdCli`** as in the code sample: `BdCli::new(binary, locks)`. `BeadsBackend` methods delegate to `btit_cli::ops` with the bd choices: `close` → `suggest_next: false` (`issue_commands.rs:411-413`, the non-br branch), `delete` → `hard: capabilities().supports_delete_hard_flag` (`:471-473`), `relation_types` → `ops::relation_types(self.client())` (bd and unknown get the extra three, `:571-578`), `sync` → `ops::sync(.., capabilities().supports_daemon_flag)` (`migration.rs:224-226`), `project_uses_dolt` → `project_uses_dolt_for(self.info_tuple(), beads_dir)`, `dolt()` → `Some(self)`, `close_suggestions()` → `None`.
3. **`CliBackend`.** `binary()`, `probe()` (fresh), `run_raw()` via the runner, `release_source()` → `BD_RELEASE_SOURCE` (`https://api.github.com/repos/steveyegge/beads/releases/latest`, `https://github.com/steveyegge/beads/releases`, `updates.rs:287,291`), exported as a `pub const` for the app's `check_bd_cli_update` selection (b-7).
4. **`DoltOperations`.** Each method is `run_raw` with a fixed argument vector built by a pure `pub(crate) fn` in `dolt.rs`: `doctor_fix_args() = ["doctor", "--fix", "--yes"]` (`migration.rs:334`), `migrate_to_dolt_args() = ["migrate", "--to-dolt", "--yes"]` (`:624`), `init_args(prefix) = ["init", "--prefix", prefix]` (`:666,772`), `import_args(file) = ["import", "-i", file]` (`:880`). Unit tests assert these vectors.
5. **`project_uses_dolt_for` moved verbatim** (`cli.rs:482-515`) with its seven tests. `project_uses_dolt_false_without_beads_dir` (`cli.rs:1239-1248`) calls the wrapper today; here it calls `BdCli::new("bd", locks).project_uses_dolt(&dir)`, which still spawns `bd --version` (B10 is fixed in b-9, not here).
6. **Unit tests** for `relation_types()` (exact ten entries in order), `release_source()`, `dolt().is_some()`, `close_suggestions().is_none()`, and `capabilities()` for `(Bd, 0.49.6)`, `(Bd, 0.55.0)`, `(Bd, 1.0.4)`, `(Unknown, None)` through a `CliRunner` whose probe is pre-seeded (a `#[cfg(test)] BdCli::with_probe(probe)` constructor).
7. **API freeze and CI.** `crates/btit-bd/tests/api_freeze.rs` pins `BdCli::new`, `BD_RELEASE_SOURCE`, `project_uses_dolt_for` and `fn _obj(b: &BdCli) -> &dyn CliBackend { b }`; `rust-quality` covers `btit-bd`.

## Required Work

- `info_tuple()` is a private helper turning the runner's `client_info()` into `Option<(CliClient, u32, u32, u32)>`, the signature `project_uses_dolt_for` keeps so its tests move unchanged.
- The app keeps calling the moved `project_uses_dolt_for` through its wrapper until b-7; nothing else in the app changes in this sprint.
- Changelog lines (collated by b-10): "New crate `btit-bd`: the `bd` backend (`BeadsBackend`, `CliBackend`, `DoltOperations`)."

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

- App construction of `BdCli` and removal of the app wrapper (b-7).
- B3 (Dolt detection rules), B10 (wrapper-calling test), B13 (test rename) in this crate (b-9).

## Acceptance Criteria

1. `cargo tree -e normal -p btit-bd --depth 1` lists exactly `btit-beads`, `btit-cli`, `btit-types`, `log`, `serde_json`; `! grep -rnE '^\s*(pub(\(crate\))? )?static ' crates/btit-bd/src` (instances only; two `BdCli` for two projects may coexist).
2. `BdCli` implements `BeadsBackend`, `CliBackend`, `DoltOperations` (pinned by `tests/api_freeze.rs`); it does not implement `CloseSuggestions`.
3. `project_uses_dolt_for` and its seven tests exist in `crates/btit-bd` and no longer in `crates/btit-app/src/cli.rs`; the app wrapper calls the moved function.
4. The argument-vector tests (Deliverable 4) and the behaviour tests (Deliverable 6) pass.
5. `cargo test --workspace` passes; test-preservation gate prints nothing.
6. `cargo clippy -p btit-bd --all-targets -- -D warnings`, `cargo rustdoc -p btit-bd -- -D missing-docs` pass; no `allow(clippy::…)` for the deny set in `src`.
7. `git diff --exit-code feature/sprint-b-4-btit-cli...HEAD -- crates/btit-beads crates/btit-types crates/btit-cli` is empty.
8. CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-types -p btit-beads -p btit-cli -p btit-bd`
- `cargo clippy -p btit-bd --all-targets -- -D warnings`
- `cargo rustdoc -p btit-bd -- -D missing-docs`
- `cargo test --workspace`
- `cargo check --workspace --all-targets`
- `cargo tree -e normal -p btit-bd --depth 1 --prefix none --format '{p}' | sed -E 's/ v.*//' | sort | diff - <(printf 'btit-bd\nbtit-beads\nbtit-cli\nbtit-types\nlog\nserde_json\n')`
- `git diff --exit-code feature/sprint-b-4-btit-cli...HEAD -- crates/btit-beads crates/btit-types crates/btit-cli`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate
