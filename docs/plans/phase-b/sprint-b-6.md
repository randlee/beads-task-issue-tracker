---
id: b-6
title: btit-br crate — BrCli (BeadsBackend, CliBackend, CloseSuggestions)
status: planned
branch: feature/sprint-b-6-btit-br
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-6-btit-br
target: integrate/phase-b
recommended_model: standard (thin implementation over btit-cli, mirror of b-5)
dependency_relations:
  - prerequisite: b-4
    dependent: b-6
    relation: must_follow
    rationale: "BrCli wraps CliRunner, delegates to btit_cli::ops, tests with testing::RecordingInvoker, and fills the btit-br skeleton b-4 registered; group A, forked from the b-4 head"
  - prerequisite: b-6
    dependent: b-7
    relation: must_follow
    rationale: "the app constructs BrCli; b-7 is the join layer of group A"
  - prerequisite: none
    parallel_pair: [b-6, b-5]
    relation: parallel_safe
    rationale: "crates/btit-br/** vs crates/btit-bd/**; shared manifests, lockfile, workflow and the recording invoker were written by b-4"
  - prerequisite: none
    parallel_pair: [b-6, b-9]
    relation: parallel_safe
    rationale: "crates/btit-br/** vs crates/btit-beads/** behind the frozen btit-beads API"
---

# Sprint b-6 — `btit-br`: `BrCli`

## Recommended Agent / Model

Recommended model: standard (thin implementation over btit-cli, mirror of b-5).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Fill the `crates/btit-br` skeleton with `BrCli`, the backend for the Rust `br` CLI (beads_rust), implementing `BeadsBackend`, `CliBackend` and `CloseSuggestions`, with today's br-specific choices: never Dolt, `--suggest-next` on close, the common relation types only, the beads_rust release repository.

## Hard Dependencies

- b-4 closure criteria met and QA-1 without Blocking finding. This branch is forked from the `feature/sprint-b-4-btit-cli` head; b-5's content is not needed.

## Dependency Relations

Trigger definitions, per-branch QA and fix-layer rules: `plan-phase-b.md` "Dependency relations" and "Parallel groups: fork and re-merge".

- b-4 → b-6 — `must_follow`.
- b-6 → b-7 — `must_follow` (b-7 is group A's join layer).
- b-6 ↔ b-5, b-6 ↔ b-9 — `parallel_safe`.

Stack: group A · layer 5 (b-5 | b-6 | b-9, first to close). If this sprint closes first it is linked as layer 5; otherwise it is merged into the b-7 branch when it closes.

## Exact Targets

Line numbers are at `a18c724`.

- `crates/btit-br/src/lib.rs` (module declarations and re-exports), `src/backend.rs`, `tests/api_freeze.rs`
- Source of the br-specific arms it implements: `cli.rs:374,394,416,437,455,487` (`Br` arms of the gates and Dolt detection), `issue_commands.rs:410-413` (`--suggest-next`), `:571-572` (common relation types), `updates.rs:286,290` (release URLs)
- `docs/plans/phase-b/sprint-b-6.md` (`status:` frontmatter only)

No edit to the root `Cargo.toml`, `Cargo.lock`, `crates/btit-br/Cargo.toml`, `crates/btit-br/clippy.toml`, `.github/workflows/ci.yml` or anything under `crates/btit-app/`.

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **`BrCli`** as in the code sample. `BeadsBackend`: `project_uses_dolt` → `false` (the `Br` arm, `cli.rs:487`); `close` → `self.close_suggesting_next(..)` (today br always passes `--suggest-next`, `issue_commands.rs:410-413`); `delete` → `hard: capabilities().supports_delete_hard_flag` (always `false` for br, `cli.rs:437`; kept data-driven); `relation_types` → `ops::relation_types(CliClient::Br)` (the common seven, `:556-564,571-572`); `sync` → `ops::sync(.., capabilities().supports_daemon_flag)` (`false`, `cli.rs:374`); `dolt()` → `None`; `close_suggestions()` → `Some(self)`.
2. **`CliBackend`.** `release_source()` → `BR_RELEASE_SOURCE` (`https://api.github.com/repos/Dicklesworthstone/beads_rust/releases/latest`, `https://github.com/Dicklesworthstone/beads_rust/releases`, `updates.rs:286,290`), exported `pub const`.
3. **`CloseSuggestions`.** `close_suggesting_next` → `ops::close(&self.runner, p, id, true)`.
4. **Unit tests** through `BrCli::with_probe` (`#[cfg(test)]`, built on `CliRunner::with_probe`) and `RecordingInvoker`: `project_uses_dolt` is `false` even when `<dir>/.dolt` exists; `relation_types()` is exactly the seven common entries in order; `release_source()`; `dolt().is_none()`; `close_suggestions().is_some()`; `capabilities()` for `(Br, 0.1.33)` equals `{ supports_daemon_flag: false, uses_jsonl_files: true, uses_dolt_backend: false, supports_list_all_flag: true, supports_delete_hard_flag: false }` (`cli.rs:374,394,416,437,455`; the `supports_list_all_flag` value is B7/OQ-4 — if b-9 flips it, b-11 updates this expectation); `close` issues `close <id> --suggest-next`.
5. **API freeze.** `crates/btit-br/tests/api_freeze.rs` pins `BrCli::new`, `BR_RELEASE_SOURCE` and `fn _obj(b: &BrCli) -> &dyn CliBackend { b }`.

## Required Work

- Changelog lines (collated by b-12): "New crate `btit-br`: the `br` (beads_rust) backend (`BeadsBackend`, `CliBackend`, `CloseSuggestions`)."

## Explicit Code Samples

```rust
// crates/btit-br/src/backend.rs
pub const BR_RELEASE_SOURCE: ReleaseSource = ReleaseSource {
    api_url: "https://api.github.com/repos/Dicklesworthstone/beads_rust/releases/latest",
    releases_url: "https://github.com/Dicklesworthstone/beads_rust/releases",
};

/// The Rust `br` CLI (beads_rust): SQLite+JSONL only, no daemon, `--suggest-next` on close.
#[derive(Debug)]
pub struct BrCli { runner: CliRunner }

impl BrCli {
    pub fn new(binary: impl Into<String>, locks: Arc<ProjectLocks>) -> Self { Self { runner: CliRunner::new(binary, locks) } }
}

impl BeadsBackend for BrCli {
    fn client(&self) -> CliClient { self.runner.client() }
    fn version(&self) -> Option<CliVersion> { self.runner.version() }
    fn capabilities(&self) -> BackendCapabilities { self.runner.capabilities() }
    fn project_uses_dolt(&self, _beads_dir: &Path) -> bool { false }                       // cli.rs:487
    fn close(&self, p: &ProjectRef, id: &str) -> Result<serde_json::Value, BeadsError> { self.close_suggesting_next(p, id) }
    fn delete(&self, p: &ProjectRef, id: &str) -> Result<(), BeadsError> { ops::delete(&self.runner, p, id, self.capabilities().supports_delete_hard_flag) }
    fn relation_types(&self) -> Vec<RelationType> { ops::relation_types(CliClient::Br) }
    fn sync(&self, p: &ProjectRef) -> Result<(), BeadsError> { ops::sync(&self.runner, p, self.capabilities().supports_daemon_flag) }
    fn close_suggestions(&self) -> Option<&dyn CloseSuggestions> { Some(self) }
    // list, ready, status, show, create, update, search, label_add, label_remove, comment_add, dep_add, dep_remove: direct delegation
}

impl CliBackend for BrCli {
    fn binary(&self) -> &str { self.runner.binary_ref() }
    fn probe(&self) -> Option<CliProbe> { self.runner.probe() }
    fn run_raw(&self, p: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> { self.runner.run_raw(p, args) }
    fn release_source(&self) -> ReleaseSource { BR_RELEASE_SOURCE }
}

impl CloseSuggestions for BrCli {
    fn close_suggesting_next(&self, p: &ProjectRef, id: &str) -> Result<serde_json::Value, BeadsError> { ops::close(&self.runner, p, id, true) }
}
```

## This Sprint Does Not Close

- App construction of `BrCli` (b-7).
- B7 (`supports_list_all_flag` for br), pending OQ-4 (b-9; expectation update in b-11).

## Acceptance Criteria

1. `git diff --name-only feature/sprint-b-4-btit-cli...HEAD | grep -vE '^(crates/btit-br/|docs/plans/phase-b/sprint-b-6.md$)'` prints nothing (group A non-intersection).
2. `cargo tree -e normal -p btit-br --depth 1` lists exactly `btit-beads`, `btit-cli`, `btit-types`, `log`, `serde_json`; `! grep -rn 'btit_bd\|btit-bd' crates/btit-br`; `! grep -rnE '^\s*(pub(\(crate\))? )?static ' crates/btit-br/src`.
3. `BrCli` implements `BeadsBackend`, `CliBackend`, `CloseSuggestions` (pinned by `tests/api_freeze.rs`); it does not implement `DoltOperations`.
4. The Deliverable 4 tests pass.
5. `cargo test --workspace` passes; test-preservation gate prints nothing.
6. `cargo clippy -p btit-br --all-targets -- -D warnings`, `cargo rustdoc -p btit-br -- -D missing-docs` pass; no `allow(clippy::…)` for the deny set in `src`.
7. QA-1 for this branch complete (per-branch rule); CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-br`
- `cargo clippy -p btit-br --all-targets -- -D warnings`
- `cargo rustdoc -p btit-br -- -D missing-docs`
- `cargo test --workspace`
- `cargo check --workspace --all-targets`
- `cargo tree -e normal -p btit-br --depth 1 --prefix none --format '{p}' | sed -E 's/ v.*//' | sort | diff - <(printf 'btit-beads\nbtit-br\nbtit-cli\nbtit-types\nlog\nserde_json\n')`
- `git diff --name-only feature/sprint-b-4-btit-cli...HEAD | grep -vE '^(crates/btit-br/|docs/plans/phase-b/sprint-b-6.md$)'` prints nothing
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate
