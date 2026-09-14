---
id: b-2
title: btit-types crate — shared data types only
status: planned
branch: feature/sprint-b-2-btit-types
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-2-btit-types
target: integrate/phase-b
recommended_model: standard (bounded code motion plus seven small new value types)
dependency_relations:
  - prerequisite: b-1
    dependent: b-2
    relation: must_follow
    rationale: "btit-types is a member of the root workspace b-1 creates; stack parent"
  - prerequisite: b-2
    dependent: b-3
    relation: must_follow
    rationale: "btit-beads traits, BeadsError and pure logic are typed with btit-types (CliClient, CliVersion, CliProbe, BdRawIssue, Issue, ListQuery, ProjectRef, …)"
---

# Sprint b-2 — `btit-types` crate: shared data types only

## Recommended Agent / Model

Recommended model: standard (bounded code motion plus seven small new value types).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Create `crates/btit-types`, the leaf crate holding every data type shared between the app, the backend contract and the backends. Data only: no I/O, no process, no Tauri, no logging.
- Move the types listed in the plan's type-move inventory out of `crates/btit-app/src/types.rs` and `cli.rs`, and add the value types the b-3 traits need (`CliVersion`, `BackendCapabilities`, `ListQuery`, `ProjectRef`, `RelationType`, `ReleaseSource`, `CliOutput`).
- The app consumes `btit-types` through `use btit_types::…`; its behaviour does not change.

## Hard Dependencies

- b-1 pushed (root workspace, `crates/btit-app`).

## Dependency Relations

Trigger definitions, per-branch QA and fix-layer rules: `plan-phase-b.md` "Dependency relations" and "Parallel groups: fork and re-merge".

- b-1 → b-2 — `must_follow` (b-2 follows b-1): workspace membership.
- b-2 → b-3 — `must_follow` (b-3 follows b-2): b-3 is typed with this crate.

Stack: `phase-b-core` · layer 2.

## Exact Targets

Line numbers are at `a18c724` (`src-tauri/src/` paths are `crates/btit-app/src/` after b-1).

- `Cargo.toml` (root): `members` gains `"crates/btit-types"`
- `Cargo.lock`
- `crates/btit-types/Cargo.toml`, `crates/btit-types/clippy.toml`, `crates/btit-types/src/lib.rs`, `crates/btit-types/src/{cli.rs,issue.rs,fs.rs,payload.rs,backend.rs}`
- `crates/btit-types/tests/api_freeze.rs` (new; compile-time pin of the public items)
- `crates/btit-app/src/types.rs` (283 lines): deleted; `crates/btit-app/src/lib.rs:3` `mod types;` removed
- `crates/btit-app/src/cli.rs:88-93` (`CliProbe`), `:597-619` (`CompatibilityInfo`), `:594-595` (orphaned doc comment, item A4): removed from `cli.rs`; `CliSelection` (`:97-111`) stays
- `use crate::types::…` in `cli.rs:3`, `issues.rs:1`, `issue_commands.rs:5`, `polling.rs:4`, `fs_commands.rs:2`, `attachments.rs:4`, `migration.rs:2`, `updates.rs:3`, `test_support.rs:2` → `use btit_types::…`
- `crates/btit-app/Cargo.toml`: `btit-types = { path = "../btit-types" }` (via `[workspace.dependencies]`)
- `.github/workflows/ci.yml`: new `rust-quality` job
- `docs/plans/phase-b/sprint-b-2.md` (`status:` frontmatter only)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Crate.** `crates/btit-types` with `[lints] workspace = true`, `clippy.toml` `allow-*-in-tests` (same four keys as `crates/sc-observability-log/clippy.toml` at the phase-a baseline), `publish = false`, inheriting `version`, `edition`, `rust-version`, `license` from the workspace. Dependencies: exactly `serde` (derive) and `serde_json`. `#![deny(missing_docs)]` with a one-line doc comment on every public item.
2. **Moved types, unchanged shape.** `CliClient` (gains `Eq`, `Hash`), `BdRawDependency`, `BdRawDependent`, `BdRawIssue`, `BdRawComment`, `Issue`, `Comment`, `ChildIssue`, `ParentIssue`, `Relation`, `CountResult`, `DirectoryEntry`, `PurgeResult`, `FsListResult`, `ListOptions`, `CwdOptions`, `CreatePayload`, `UpdatePayload` (`types.rs:5-281`) and `CompatibilityInfo` (`cli.rs:597-619`) move with every `#[serde(...)]` attribute and derive intact. All fields become `pub`. Serialized JSON is byte-identical: pinned by moving `compatibility_info_serializes_camel_case`, `compatibility_info_arrays_and_null_tuple_serialize`, `compatibility_info_empty_arrays_serialize_as_empty_not_null` (`cli.rs:818,1169,1199`) into `crates/btit-types/tests/` (they construct `CompatibilityInfo` literals and check JSON; the `check_bd_compatibility` function they do not call).
3. **`CliProbe`** moves (`cli.rs:88-93`) with `pub` fields and `version: Option<CliVersion>`. `From<(u32, u32, u32)> for CliVersion` and `From<CliVersion> for (u32, u32, u32)` keep every `_for` core call site and the `test_support::probe` helper (`test_support.rs:4-11`) compiling with a `.into()`.
4. **New value types** exactly as in the code samples: `CliVersion`, `BackendCapabilities`, `ListQuery`, `ProjectRef`, `RelationType`, `ReleaseSource`, `CliOutput`. No methods beyond derives, `From` impls and `Display` for `CliVersion` (`"{major}.{minor}.{patch}"`, the format `get_cli_client_info` logs today, `cli.rs:358`).
5. **App consumes the crate.** `types.rs` deleted; every `use crate::types::…` replaced; `CompatibilityInfo` constructed in `check_bd_compatibility` with the same field values. `cargo test --workspace` passes the same test set (142 minus the three moved to `btit-types/tests`, which appear there).
6. **API freeze.** `crates/btit-types/tests/api_freeze.rs` names every public type, field and derive it depends on (`let _: fn(CliProbe) -> Option<CliVersion> = |p| p.version;` style pins, plus `serde_json::to_string` round trips for each DTO), so b-3..b-8 cannot change the contract silently. b-9, b-10 and b-11 do not touch this crate.
7. **CI.** `rust-quality` job (3 OSes): `cargo fmt --check -p btit-types`, `cargo clippy -p btit-types --all-targets -- -D warnings`, `cargo rustdoc -p btit-types -- -D missing-docs`, and the dependency gate `cargo tree -e normal -p btit-types --depth 1 --prefix none --format '{p}' | sed -E 's/ v.*//' | sort | diff - <(printf 'btit-types\nserde\nserde_json\n')`. Later sprints append their crates to the same job.

## Required Work

- Module layout inside the crate: `cli.rs` (`CliClient`, `CliVersion`, `CliProbe`, `BackendCapabilities`, `CompatibilityInfo`, `ReleaseSource`, `CliOutput`), `issue.rs` (raw and normalized issue types), `fs.rs` (`DirectoryEntry`, `FsListResult`, `PurgeResult`), `payload.rs` (`ListOptions`, `CwdOptions`, `CreatePayload`, `UpdatePayload`, `ListQuery`), `backend.rs` (`ProjectRef`, `RelationType`). `lib.rs` re-exports every type at the crate root, so consumers write `btit_types::Issue`.
- Item A4: the orphaned doc comment at `cli.rs:594-595` ("Auto-run refs migration v3 …") is deleted here, because the lines around it (`CompatibilityInfo`) move. b-8 adds the doc comment to `ensure_refs_migrated_v3`.
- `CliSelection` (`cli.rs:97-111`) is **not** moved: it carries `is_legacy()` logic and goes to `btit-beads::detect` in b-3.
- Changelog lines (collated by b-12): "New crate `btit-types`: the frontend data contract and CLI probe/capability value types, data only."

## Explicit Code Samples

```toml
# crates/btit-types/Cargo.toml
[package]
name = "btit-types"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Shared data types for the Beads Task-Issue Tracker (no I/O, no CLI, no Tauri)."
publish = false

[dependencies]
serde.workspace = true
serde_json.workspace = true

[lints]
workspace = true
```

```rust
// crates/btit-types/src/cli.rs (new items; moved items keep their a18c724 definitions)

/// Which beads CLI answered `--version`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CliClient {
    /// Go `bd` (steveyegge/beads), the primary CLI.
    Bd,
    /// Rust `br` (beads_rust), secondary.
    Br,
    /// `--version` output not recognized.
    Unknown,
}

/// Parsed `major.minor.patch` of a CLI. Replaces the `(u32, u32, u32)` tuple at crate boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CliVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}
impl From<(u32, u32, u32)> for CliVersion { fn from((major, minor, patch): (u32, u32, u32)) -> Self { Self { major, minor, patch } } }
impl From<CliVersion> for (u32, u32, u32) { fn from(v: CliVersion) -> Self { (v.major, v.minor, v.patch) } }
impl std::fmt::Display for CliVersion { /* "{major}.{minor}.{patch}" */ }

/// Result of running `<bin> --version` (moved from cli.rs:88-93).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliProbe {
    pub client: CliClient,
    pub version: Option<CliVersion>,
    /// Trimmed first line of `--version` output, for logs and the UI.
    pub raw: String,
}

/// The five version-gated capabilities (today the `supports_*`/`uses_*` wrappers, cli.rs:380-466).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BackendCapabilities {
    pub supports_daemon_flag: bool,
    pub uses_jsonl_files: bool,
    pub uses_dolt_backend: bool,
    pub supports_list_all_flag: bool,
    pub supports_delete_hard_flag: bool,
}

/// GitHub release location used by `check_bd_cli_update` (updates.rs:285-292).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSource {
    pub api_url: &'static str,
    pub releases_url: &'static str,
}

/// Raw process result for un-JSON'd CLI invocations (`CliBackend::run_raw`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    /// `ExitStatus::code()`; `None` when terminated by a signal.
    pub status: Option<i32>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}
```

```rust
// crates/btit-types/src/backend.rs
/// Where a beads project lives. Today only the local working directory that every
/// Tauri command receives as `cwd: Option<String>`; the enum is non-exhaustive so a
/// remote transport (beads Dolt server, DoltHub) can add a variant without changing
/// callers that pass a `ProjectRef` through.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectRef {
    Local { cwd: Option<String> },
}
impl ProjectRef {
    pub fn local(cwd: Option<String>) -> Self { Self::Local { cwd } }
}

/// A dependency relation type offered by `bd_available_relation_types` (issue_commands.rs:556-569).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RelationType {
    pub value: &'static str,
    pub label: &'static str,
}
```

```rust
// crates/btit-types/src/payload.rs (addition)
/// Filters for `BeadsBackend::list`; built by the app from `ListOptions` (issue_commands.rs:41-62).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListQuery {
    pub status: Option<Vec<String>>,
    pub issue_type: Option<Vec<String>>,
    /// Priority strings as the frontend sends them (`"p0"`..); the backend converts with `priority_to_number`.
    pub priority: Option<Vec<String>>,
    pub assignee: Option<String>,
    pub include_all: bool,
}
```

```yaml
# .github/workflows/ci.yml — new job (3-OS matrix like `backend`)
  rust-quality:
    name: rust quality (${{ matrix.os }})
    steps:
      # checkout, toolchain 1.98.1 with clippy+rustfmt, rust-cache (as in `backend`)
      - run: cargo fmt --check -p btit-types
      - run: cargo clippy -p btit-types --all-targets -- -D warnings
      - run: cargo rustdoc -p btit-types -- -D missing-docs
      - if: runner.os == 'Linux'
        shell: bash
        run: cargo tree -e normal -p btit-types --depth 1 --prefix none --format '{p}' | sed -E 's/ v.*//' | sort | diff - <(printf 'btit-types\nserde\nserde_json\n')
```

## This Sprint Does Not Close

- Traits, `BeadsError`, pure logic (b-3).
- Any change to serialized shapes or field names (none planned in phase-b).
- App-only DTOs (`PollData`, `RepairResult`, …) stay in `btit-app` by design (plan type-move inventory).

## Acceptance Criteria

1. `cargo tree -e normal -p btit-types --depth 1` lists exactly `serde` and `serde_json`; `! grep -rnE 'std::(process|fs|env|io)|tauri|log::|impl .* fn .*\(' crates/btit-types/src` (no I/O modules, no Tauri, no logging; the regex for `impl` methods is refined in the PR to allow `From`/`Display`/`local()` only).
2. Every type in the plan's type-move inventory with destination `btit-types` exists at `btit_types::<Name>` with the listed derives and attributes; `crates/btit-app/src/types.rs` does not exist.
3. `git diff origin/integrate/phase-b...HEAD -- crates/btit-app/src` shows only `use` line changes, the `mod types;` removal, the `CliProbe`/`CompatibilityInfo`/orphaned-comment removal from `cli.rs`, `.into()`/`CliVersion` adaptations at `_for` call sites, and `CompatibilityInfo { .. }` construction edits; no other line changes.
4. `cargo test --workspace` passes; the three JSON tests from Deliverable 2 pass under `btit-types`; the test-preservation gate (plan "Test preservation") prints nothing.
5. `crates/btit-types/tests/api_freeze.rs` compiles and pins every public type.
6. `python3 scripts/check_version_sync.py` prints `(beads-issue-tracker, btit-types)` in its OK line.
7. CI green, including the new `rust-quality` job.
8. Every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-types`
- `cargo clippy -p btit-types --all-targets -- -D warnings`
- `cargo rustdoc -p btit-types -- -D missing-docs`
- `cargo test --workspace`
- `cargo check --workspace --all-targets`
- `python3 scripts/check_version_sync.py`
- the `cargo tree` dependency gate from Deliverable 7
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
