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
    rationale: "BrCli wraps CliRunner and delegates to btit_cli::ops (content dependency)"
  - prerequisite: b-5
    dependent: b-6
    relation: must_follow
    rationale: "stack parent only: GitHub stacks are linear, so b-6 sits above b-5; development starts as soon as the b-5 branch exists"
  - prerequisite: b-6
    dependent: b-7
    relation: must_follow
    rationale: "the app constructs BrCli"
  - prerequisite: none
    parallel_pair: [b-6, b-5]
    relation: parallel_safe
    rationale: "disjoint crates; only the root Cargo.toml member list is shared and is resolved in the rebase"
  - prerequisite: none
    parallel_pair: [b-6, b-8]
    relation: parallel_safe
    rationale: "b-6 owns crates/btit-br/**; b-8 owns crates/btit-beads/** behind the frozen API"
---

# Sprint b-6 — `btit-br`: `BrCli`

## Recommended Agent / Model

Recommended model: standard (thin implementation over btit-cli, mirror of b-5).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Create `crates/btit-br` with `BrCli`, the backend for the Rust `br` CLI (beads_rust), implementing `BeadsBackend`, `CliBackend` and `CloseSuggestions`, with today's br-specific choices: never Dolt, `--suggest-next` on close, the common relation types only, the beads_rust release repository.

## Hard Dependencies

- b-4 pushed. The b-5 branch exists (stack parent; its content is not needed).

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA; merge parent → child before every dev/fix round. PR-completion trigger: parent PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- b-4 → b-6 — `must_follow` (content); b-5 → b-6 — `must_follow` (stack parent).
- b-6 → b-7 — `must_follow`.
- b-6 ↔ b-5, b-6 ↔ b-8 — `parallel_safe`.

Stack: `phase-b-core` · layer 6.

## Exact Targets

Line numbers are at `a18c724`.

- `Cargo.toml` (root): member `crates/btit-br`
- `crates/btit-br/Cargo.toml`, `clippy.toml`, `src/lib.rs`, `src/backend.rs`, `tests/api_freeze.rs`
- Source of the br-specific arms it implements: `cli.rs:374,394,416,437,455,487` (`Br` arms of the gates and Dolt detection), `issue_commands.rs:410-413` (`--suggest-next`), `:571-572` (common relation types), `updates.rs:286,290` (release URLs)
- `.github/workflows/ci.yml`: `rust-quality` gains `btit-br`
- `docs/plans/phase-b/sprint-b-6.md` (`status:` frontmatter only)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Crate.** `crates/btit-br`, `[lints] workspace = true`, `#![deny(missing_docs)]`, `publish = false`. Dependencies: `btit-types`, `btit-beads`, `btit-cli`, `serde_json`, `log`. Never `btit-bd`.
2. **`BrCli`** as in the code sample. `BeadsBackend`: `project_uses_dolt` → `false` (the `Br` arm, `cli.rs:487`); `close` → `self.close_suggesting_next(..)` (today br always passes `--suggest-next`, `issue_commands.rs:410-413`); `delete` → `hard: capabilities().supports_delete_hard_flag` (always `false` for br, `cli.rs:437`; kept data-driven); `relation_types` → `ops::relation_types(CliClient::Br)` (the common seven, `:556-564,571-572`); `sync` → `ops::sync(.., false)` via `capabilities().supports_daemon_flag` (`cli.rs:374`); `dolt()` → `None`; `close_suggestions()` → `Some(self)`.
3. **`CliBackend`.** `release_source()` → `BR_RELEASE_SOURCE` (`https://api.github.com/repos/Dicklesworthstone/beads_rust/releases/latest`, `https://github.com/Dicklesworthstone/beads_rust/releases`, `updates.rs:286,290`), exported `pub const`.
4. **`CloseSuggestions`.** `close_suggesting_next` → `ops::close(&self.runner, p, id, true)`.
5. **Unit tests** through a pre-seeded `CliRunner` (`#[cfg(test)] BrCli::with_probe`): `project_uses_dolt` is `false` even when `<dir>/.dolt` exists; `relation_types()` is exactly the seven common entries in order; `release_source()`; `dolt().is_none()`; `close_suggestions().is_some()`; `capabilities()` for `(Br, 0.1.33)` equals `{ supports_daemon_flag: false, uses_jsonl_files: true, uses_dolt_backend: false, supports_list_all_flag: true, supports_delete_hard_flag: false }` (`cli.rs:374,394,416,437,455`; the `supports_list_all_flag` value is B7/OQ-4 and this test is the one b-8 updates if the maintainer confirms the doc comment); `close` through the recording invoker issues `close <id> --suggest-next`.
6. **API freeze and CI.** `crates/btit-br/tests/api_freeze.rs` pins `BrCli::new`, `BR_RELEASE_SOURCE` and `fn _obj(b: &BrCli) -> &dyn CliBackend { b }`; `rust-quality` covers `btit-br`.

## Required Work

- No app changes in this sprint.
- Changelog lines (collated by b-10): "New crate `btit-br`: the `br` (beads_rust) backend (`BeadsBackend`, `CliBackend`, `CloseSuggestions`)."

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
- B7 (`supports_list_all_flag` for br), pending OQ-4 (b-8).

## Acceptance Criteria

1. `cargo tree -e normal -p btit-br --depth 1` lists exactly `btit-beads`, `btit-cli`, `btit-types`, `log`, `serde_json`; `! grep -rn 'btit_bd\|btit-bd' crates/btit-br`; `! grep -rnE '^\s*(pub(\(crate\))? )?static ' crates/btit-br/src`.
2. `BrCli` implements `BeadsBackend`, `CliBackend`, `CloseSuggestions` (pinned by `tests/api_freeze.rs`); it does not implement `DoltOperations`.
3. The Deliverable 5 tests pass.
4. `cargo test --workspace` passes; test-preservation gate prints nothing.
5. `cargo clippy -p btit-br --all-targets -- -D warnings`, `cargo rustdoc -p btit-br -- -D missing-docs` pass; no `allow(clippy::…)` for the deny set in `src`.
6. `git diff --exit-code feature/sprint-b-5-btit-bd...HEAD -- crates/btit-beads crates/btit-types crates/btit-cli crates/btit-bd crates/btit-app` is empty.
7. CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-types -p btit-beads -p btit-cli -p btit-bd -p btit-br`
- `cargo clippy -p btit-br --all-targets -- -D warnings`
- `cargo rustdoc -p btit-br -- -D missing-docs`
- `cargo test --workspace`
- `cargo check --workspace --all-targets`
- `cargo tree -e normal -p btit-br --depth 1 --prefix none --format '{p}' | sed -E 's/ v.*//' | sort | diff - <(printf 'btit-beads\nbtit-br\nbtit-cli\nbtit-types\nlog\nserde_json\n')`
- `git diff --exit-code feature/sprint-b-5-btit-bd...HEAD -- crates/btit-beads crates/btit-types crates/btit-cli crates/btit-bd crates/btit-app`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate
