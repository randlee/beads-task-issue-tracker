---
id: b-6
title: btit-br crate — BrCli (BeadsBackend, CliBackend, CloseSuggestions)
status: complete
branch: feature/sprint-b-6-btit-br
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-6-btit-br
target: integrate/phase-b
recommended_model: standard (thin implementation over btit-cli, mirror of b-5)
dependency_relations:
  - prerequisite: b-4
    dependent: b-6
    relation: must_follow
    rationale: "BrCli wraps a Box<dyn CliInvoker>, delegates to btit_cli::ops, and fills the btit-br skeleton b-4 registered; group A, forked from the b-4 head"
  - prerequisite: b-6
    dependent: b-7
    relation: must_follow
    rationale: "the app constructs BrCli; b-7 is the join layer of group A"
  - prerequisite: none
    parallel_pair: [b-6, b-5]
    relation: parallel_safe
    rationale: "crates/btit-br/** vs crates/btit-bd/**; shared manifests, lockfile, workflow, the test-support feature and the recording invoker were written by b-4"
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

- Fill the `crates/btit-br` skeleton with `BrCli`, the backend for the Rust `br` CLI (beads_rust), implementing `BeadsBackend`, `CliBackend` and `CloseSuggestions` over a `Box<dyn CliInvoker>`, with today's br-specific choices: never Dolt, `--suggest-next` on close, the common relation types only, the beads_rust release repository.

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

- `crates/btit-br/src/lib.rs` (module declarations and re-exports), `src/backend.rs`, `tests/api_freeze.rs`, `tests/parity.rs`
- Source of the br-specific arms it implements: `cli.rs:374,394,416,437,455,487` (`Br` arms of the gates and Dolt detection), `issue_commands.rs:410-413` (`--suggest-next`), `:571-572` (common relation types), `updates.rs:286,290` (release URLs)
- `docs/plans/phase-b/sprint-b-6.md` (`status:` frontmatter only)

No edit to the root `Cargo.toml`, `Cargo.lock`, `crates/btit-br/Cargo.toml`, `crates/btit-br/clippy.toml`, `.github/workflows/ci.yml` or anything under `crates/btit-app/`.

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **`BrCli { inv: Box<dyn CliInvoker> }`** as in the code sample; `new(binary, locks)` wraps `CliRunner::new`; `with_invoker(inv)` behind `#[cfg(feature = "test-support")]` (the crate feature b-4 declared).
2. **`BeadsBackend`** (transport-neutral methods only): `project_uses_dolt(_)` → `false` (the `Br` arm, `cli.rs:487`; no working-dir resolution needed); `close` → `self.close_suggesting_next(..)` (today br always passes `--suggest-next`, `issue_commands.rs:410-413`); `delete` → `hard: self.capabilities().supports_delete_hard_flag` (always `false` for br, `cli.rs:437`; kept data-driven); `relation_types` → `ops::relation_types(CliClient::Br)` (the common seven, `:556-564,571-572`); `sync` → `ops::sync(.., self.capabilities().supports_daemon_flag)` (`false`, `cli.rs:374`); accessors `cli()` → `Some(self)`, `dolt()` → `None`, `close_suggestions()` → `Some(self)`.
3. **`CliBackend`** (CLI-only facts): `binary()`, `probe()` (via `inv.probe()`), `client()`, `version()`, `capabilities()`, `run_raw()`, `release_source()` → `BR_RELEASE_SOURCE` (`https://api.github.com/repos/Dicklesworthstone/beads_rust/releases/latest`, `https://github.com/Dicklesworthstone/beads_rust/releases`, `updates.rs:286,290`), exported `pub const`.
4. **`CloseSuggestions`.** `close_suggesting_next` → `ops::close(self.inv.as_ref(), p, id, true)`.
5. **Full-method parity tests** (`tests/parity.rs`, through `BrCli::with_invoker(Box::new(RecordingInvoker::new(Some(probe))))` for `Br 0.1.33` and `RecordingInvoker::new(None)` for the no-probe case): every `BeadsBackend` method is called once and the recorded full argv equals the b-4 Required Work table's `Br 0.1.33` column (`close` records `close <id> --suggest-next --json`; `list(include_all)` records `--all` or the two-call form according to `capabilities_for(Br, Some(0.1.33)).supports_list_all_flag`, computed in the test, not a literal — so a b-9 flip of B7/OQ-4 changes no file in this crate). **No-probe column for `BrCli`:** `list(include_all)` → the two calls `list --limit=0 --json`, `list --limit=0 --status=closed --json` (`capabilities_for(Unknown, None)` is all `false`); `close` → `close <id> --suggest-next --json` (the flag is br's, not version-gated); every other row as `Br 0.1.33`. `project_uses_dolt` is `false` even when `<dir>/.beads/.dolt` exists; `relation_types()` is exactly the seven common entries in order; `release_source()`; `dolt().is_none()`; `cli().is_some()`; `close_suggestions().is_some()`; `capabilities()` equals `capabilities_for(Br, Some(0.1.33))` (whose fields at `a18c724` are `{ supports_daemon_flag: false, uses_jsonl_files: true, uses_dolt_backend: false, supports_list_all_flag: true, supports_delete_hard_flag: false }`, `cli.rs:374,394,416,437,455`).
6. **API freeze.** `crates/btit-br/tests/api_freeze.rs` pins `BrCli::new`, `BrCli::with_seeded_probe`, `BrCli::with_invoker` (with `--features test-support`), `BR_RELEASE_SOURCE`, `fn _obj(b: &BrCli) -> &dyn BeadsBackend { b }` and `fn _cli(b: &BrCli) -> &dyn CliBackend { b }`.

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
pub struct BrCli { inv: Box<dyn CliInvoker> }

impl BrCli {
    pub fn new(binary: impl Into<String>, locks: Arc<ProjectLocks>) -> Self { Self { inv: Box::new(CliRunner::new(binary, locks)) } }
    /// `new` with the probe cache pre-seeded (b-7's factory and `check_bd_compatibility` rebuild; no second `--version` spawn).
    pub fn with_seeded_probe(binary: impl Into<String>, locks: Arc<ProjectLocks>, probe: CliProbe) -> Self { Self { inv: Box::new(CliRunner::with_probe(binary, locks, probe)) } }
    #[cfg(feature = "test-support")]
    pub fn with_invoker(inv: Box<dyn CliInvoker>) -> Self { Self { inv } }
}

impl BeadsBackend for BrCli {
    fn project_uses_dolt(&self, _project: &ProjectRef) -> bool { false }                    // cli.rs:487
    fn close(&self, p: &ProjectRef, id: &str) -> Result<serde_json::Value, BeadsError> { self.close_suggesting_next(p, id) }
    fn delete(&self, p: &ProjectRef, id: &str) -> Result<(), BeadsError> { ops::delete(self.inv.as_ref(), p, id, self.capabilities().supports_delete_hard_flag) }
    fn relation_types(&self) -> Vec<RelationType> { ops::relation_types(CliClient::Br) }
    fn sync(&self, p: &ProjectRef) -> Result<(), BeadsError> { ops::sync(self.inv.as_ref(), p, self.capabilities().supports_daemon_flag) }
    fn cli(&self) -> Option<&dyn CliBackend> { Some(self) }
    fn close_suggestions(&self) -> Option<&dyn CloseSuggestions> { Some(self) }
    // list, ready, status, show, create, update, search, label_add, label_remove, comment_add, dep_add, dep_remove: direct delegation
}

impl CliBackend for BrCli {
    fn binary(&self) -> String { self.inv.binary() }
    fn probe(&self) -> Option<CliProbe> { self.inv.probe() }
    fn client(&self) -> CliClient { self.inv.client() }
    fn version(&self) -> Option<CliVersion> { self.inv.version() }
    fn capabilities(&self) -> BackendCapabilities { self.inv.capabilities() }
    fn run_raw(&self, p: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> { self.inv.run_raw(p, args) }
    fn release_source(&self) -> ReleaseSource { BR_RELEASE_SOURCE }
}

impl CloseSuggestions for BrCli {
    fn close_suggesting_next(&self, p: &ProjectRef, id: &str) -> Result<serde_json::Value, BeadsError> { ops::close(self.inv.as_ref(), p, id, true) }
}
```

## This Sprint Does Not Close

- App construction of `BrCli` (b-7).
- B7 (`supports_list_all_flag` for br), pending OQ-4 (b-9); no follow-up edit here because the tests derive their expectations from `capabilities_for`.

## Acceptance Criteria

1. `git diff --name-only feature/sprint-b-4-btit-cli...HEAD | grep -vE '^(crates/btit-br/|docs/plans/phase-b/sprint-b-6.md$)'` prints nothing (group A non-intersection).
2. `cargo tree -e normal -p btit-br --depth 1` lists exactly `btit-beads`, `btit-cli`, `btit-types`, `log`, `serde_json`; `! grep -rn 'btit_bd\|btit-bd' crates/btit-br`; `! grep -rnE '^\s*(pub(\(crate\))? )?static ' crates/btit-br/src`; `grep -nE '^pub struct BrCli \{ inv: Box<dyn CliInvoker> \}' crates/btit-br/src/backend.rs` matches and `! grep -nE ':\s*CliRunner\b|<CliRunner>' crates/btit-br/src`.
3. `BrCli` implements `BeadsBackend`, `CliBackend`, `CloseSuggestions` (pinned by `tests/api_freeze.rs`); it does not implement `DoltOperations`; `with_invoker` exists only with `--features test-support`.
4. The Deliverable 5 parity tests pass; `cargo test -p btit-br --features test-support` and `cargo test -p btit-br` both pass.
5. `cargo test --workspace` passes; test-preservation gate prints nothing.
6. `cargo clippy -p btit-br --all-targets --all-features -- -D warnings`, `cargo rustdoc -p btit-br -- -D missing-docs` pass; no `allow(clippy::…)` for the deny set in `src`.
7. QA-1 for this branch complete (per-branch rule); CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-br`
- `cargo clippy -p btit-br --all-targets --all-features -- -D warnings`
- `cargo rustdoc -p btit-br -- -D missing-docs`
- `cargo test --workspace`
- `cargo test -p btit-br --features test-support`
- `cargo check --workspace --all-targets`
- `cargo tree -e normal -p btit-br --depth 1 --prefix none --format '{p}' | sed -E 's/ v.*//' | sort | diff - <(printf 'btit-beads\nbtit-br\nbtit-cli\nbtit-types\nlog\nserde_json\n')`
- `git diff --name-only feature/sprint-b-4-btit-cli...HEAD | grep -vE '^(crates/btit-br/|docs/plans/phase-b/sprint-b-6.md$)'` prints nothing
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate

## Implementation Notes

Implemented `BrCli { inv: Box<dyn CliInvoker> }` in `crates/btit-br/src/backend.rs`
per the Explicit Code Samples, plus `crates/btit-br/src/lib.rs` (module declaration
and doc-inline re-exports of `BrCli`, `BR_RELEASE_SOURCE`),
`crates/btit-br/tests/api_freeze.rs` and `crates/btit-br/tests/parity.rs`.

### Deviations from the sprint doc

- **`#[derive(Debug)]` does not compile on `BrCli`.** `CliInvoker` (`btit-cli`) has
  no `Debug` bound (`Send + Sync` only), so `Box<dyn CliInvoker>` is not `Debug` and
  the literal code sample's `#[derive(Debug)]` fails with E0277. Replaced with a
  manual `impl std::fmt::Debug for BrCli` that prints the configured binary name
  (`f.debug_struct("BrCli").field("binary", &self.inv.binary())`). The pinned line
  `pub struct BrCli { inv: Box<dyn CliInvoker> }` (AC2) is preserved verbatim, with
  `#[rustfmt::skip]` added so `cargo fmt` does not expand it to a multi-line struct
  (rustfmt's default one-field-per-line style would otherwise break that grep).
- **`tests/api_freeze.rs` cannot pin `BrCli::new`/`BrCli::with_seeded_probe` as bare
  fn pointers.** Both take `binary: impl Into<String>`, which is generic per call
  site and cannot unify with a single `fn(&str, ...) -> BrCli` pointer type (E0308,
  same shape as why `btit-cli`'s own `tests/api_freeze.rs` calls `CliRunner::new`
  directly instead of pinning it as a fn pointer). The test calls both constructors
  directly with an explicit `: BrCli` binding and converts the result to `&dyn
  BeadsBackend`/`&dyn CliBackend` instead; a signature change still fails to
  compile. `with_invoker` (no generic parameter) is pinned as a literal fn pointer
  as the doc describes.
- **`tests/parity.rs`'s per-method helper (`call_every_method`) returns `Result`
  instead of unwrapping inline.** `clippy.toml`'s `allow-unwrap-in-tests` only
  exempts code directly inside `#[test]` functions (or `#[cfg(test)]` modules), not
  a plain helper fn called from them, so `-D warnings` rejected the original
  `.unwrap()`-per-call helper. `call_every_method` now propagates with `?` and
  returns `Result<(), BeadsError>`; the two `#[test]` callers unwrap the result
  themselves, where the allowance applies.
- **`tests/parity.rs` wraps `RecordingInvoker` in an `Arc` behind a local
  `CliInvoker` adapter (`ArcInvoker`)**, rather than moving a bare
  `RecordingInvoker` straight into `BrCli::with_invoker` as the deliverable's prose
  literally reads. `with_invoker` takes ownership of the `Box<dyn CliInvoker>`, so
  without keeping a second handle the test could never call `.calls()` afterward to
  assert the recorded argv the deliverable requires. `ArcInvoker(Arc<RecordingInvoker>)`
  is a local type implementing the foreign `CliInvoker` trait (orphan-rule-legal,
  since `Arc<RecordingInvoker>` itself is foreign-on-foreign and cannot carry the
  impl); the `Arc` is cloned once into the box and once kept by the test.

No other deviations; no behaviour delta from the spec (br: never Dolt,
`--suggest-next` always on close, common-seven relation types, no
`--all`-vs-two-call ambiguity difference from `bd` beyond what `capabilities_for`
already encodes).

### Gate results (this worktree, `feature/sprint-b-6-btit-br`)

- `cargo fmt --check -p btit-br` — clean.
- `cargo clippy -p btit-br --all-targets --all-features -- -D warnings` — clean, no
  `#[allow(clippy::...)]` in `src`.
- `cargo rustdoc -p btit-br -- -D missing-docs` — clean (0 warnings after fixing two
  `clippy::doc_markdown` findings on `beads_rust` in doc comments).
- `cargo test -p btit-br --features test-support` — 2 (api_freeze) + 9 (parity) = 11
  tests, all pass.
- `cargo test -p btit-br` (no `test-support`) — 1 test (`api_freeze::constructors`);
  `parity.rs` is `#![cfg(feature = "test-support")]`-gated and compiles to 0 tests,
  as required by Deliverable 6 ("`with_invoker` exists only with
  `--features test-support`").
- `cargo test --workspace` — passes (exit 0); all crates green.
- `cargo check --workspace --all-targets` — clean.
- `cargo tree -e normal -p btit-br --depth 1 --prefix none --format '{p}' | sed -E
  's/ v.*//' | sort | diff - <(printf 'btit-beads\nbtit-br\nbtit-cli\nbtit-types\nlog\nserde_json\n')`
  — empty diff.
- `cargo tree -e normal,features -p beads-issue-tracker | grep -c test-support` — 0.
- `git diff --name-only feature/sprint-b-4-btit-cli...HEAD | grep -vE
  '^(crates/btit-br/|docs/plans/phase-b/sprint-b-6.md$)'` — empty (confirmed after
  reverting an accidental workspace-wide `cargo fmt --all` that reformatted
  `crates/btit-app/*`; only `-p btit-br` fmt/clippy invocations were used from then
  on, per the sprint's "touch only b-6 files" scope).
- `python3 scripts/check_version_sync.py` — `version sync OK: app 1.24.5, ...`.
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target
  x86_64-pc-windows-msvc --all-targets` — clean for `btit-br`; the only warnings
  are pre-existing `btit-app` unused-import/unused-variable warnings tracked in
  #49, not touched here.
- `git diff --check` — clean (exit 0).
- Test-preservation gate (`IMPLEMENTATION_BASELINE=94e44d3`,
  `BASELINE_TEST_COUNT=155`): `/tmp/baseline-tests.txt` has exactly 155 lines,
  `/tmp/after-tests.txt` has 351 lines (workspace, `--all-features`), and
  `comm -23 /tmp/baseline-tests.txt /tmp/after-tests.txt` prints nothing.

### Ambiguity resolved

The Explicit Code Samples' `RecordingInvoker::new(Some(probe))` boxed directly into
`BrCli::with_invoker` in Deliverable 5's prose does not by itself let a test read
back `calls()` (ownership moves into the `Box<dyn CliInvoker>` `BrCli` holds
privately). Resolved by keeping the scripted invoker in an `Arc` and boxing a thin
local adapter around a clone of it (see Deviations); the assertions are unchanged —
every recorded argv is checked against the same b-4 Required Work table cells the
doc specifies.
