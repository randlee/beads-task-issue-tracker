---
id: b-3
title: btit-beads crate — backend traits, BeadsError, pure beads logic, log gate
status: planned
branch: feature/sprint-b-3-btit-beads
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-3-btit-beads
target: integrate/phase-b
recommended_model: higher-effort (trait contract that every later sprint and every future transport builds on)
dependency_relations:
  - prerequisite: b-2
    dependent: b-3
    relation: must_follow
    rationale: "traits, BeadsError and the pure logic are typed with btit-types; stack parent"
  - prerequisite: b-3
    dependent: b-4
    relation: must_follow
    rationale: "btit-cli's CliRunner returns BeadsError, uses parse/gates/detect and the log_*! macros from btit-beads"
  - prerequisite: b-3
    dependent: b-9
    relation: must_follow
    rationale: "b-9 edits the pure functions this sprint moves into btit-beads, behind the API frozen by tests/api_freeze.rs"
---

# Sprint b-3 — `btit-beads`: backend traits, `BeadsError`, pure beads logic, log gate

## Recommended Agent / Model

Recommended model: higher-effort (trait contract that every later sprint and every future transport builds on).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Create `crates/btit-beads` holding (a) the backend contract: `BeadsBackend`, `CliBackend`, `DoltOperations`, `CloseSuggestions`; (b) `BeadsError`, the only error type that crosses crate boundaries; (c) the pure beads logic both CLIs share today, moved from `cli.rs` and `issues.rs` with its tests; (d) the `LOGGING_ENABLED`/`VERBOSE_LOGGING` gate and the `log_*!` macros.
- Freeze the public API with a compile-time test so b-4..b-8 build against it while b-9 changes behaviour behind it.
- The app consumes the moved pure logic through re-exports; no behaviour changes.

## Hard Dependencies

- b-2 pushed (`btit-types` with `CliVersion`, `CliProbe`, `BackendCapabilities`, `ListQuery`, `ProjectRef`, `RelationType`, `ReleaseSource`, `CliOutput`).

## Dependency Relations

Trigger definitions, per-branch QA and fix-layer rules: `plan-phase-b.md` "Dependency relations" and "Parallel groups: fork and re-merge".

- b-2 → b-3 — `must_follow`.
- b-3 → b-4 — `must_follow`: b-4 implements against this contract.
- b-3 → b-9 — `must_follow`: b-9 edits `crates/btit-beads/src/{issues,gates,compat}.rs` and their tests behind the frozen API; it starts when b-3 is pushed (plan "Execution lanes").

Stack: `phase-b-core` · layer 3.

## Exact Targets

Line numbers are at `a18c724` (`crates/btit-app/src/` after b-1).

- `Cargo.toml` (root): member `crates/btit-beads`; `[workspace.dependencies]` unchanged (`log`, `serde_json` already present)
- `crates/btit-beads/Cargo.toml`, `clippy.toml`, `src/lib.rs`, `src/backend.rs`, `src/error.rs`, `src/detect.rs`, `src/compat.rs`, `src/gates.rs`, `src/issues.rs`, `src/parse.rs`, `src/logging.rs`, `src/test_support.rs` (`#[cfg(test)]`), `tests/api_freeze.rs`, `docs/backend-contract.md`
- Moved out of `crates/btit-app/src/cli.rs`: lines 76-84 (`CLI_CANDIDATES`, `CLI_FALLBACK`, `MIN_SUPPORTED_BD_MAJOR`), 97-178 (`CliSelection`, `is_legacy_bd`, `rank_cli_candidate`, `select_default_binary`, `parse_cli_probe`), 209-253 (`cli_client_name`, `cli_compatibility_warnings`), 290-323 (`detect_cli_client`, `parse_bd_version`), 372-378, 392-398, 413-419, 434-439, 453-459 (the five `_for` cores), and their tests at 676-753, 759-803, 818-845 (the JSON ones went to b-2), 875-1115, 1162-1168, 1307-1421
- Moved out of `crates/btit-app/src/issues.rs`: the whole file (production 1-312, tests 315-665) and `crates/btit-app/src/test_support.rs:13-22` (`minimal_issue_json`, `issue_json_with_metadata`) plus `:4-11` (`probe`)
- Moved out of `crates/btit-app/src/logging.rs:32-66` (`LOGGING_ENABLED`, `VERBOSE_LOGGING`, four macros)
- `crates/btit-app/src/lib.rs`: `#[macro_use] mod logging;` (lines 1-2) becomes `#[macro_use] extern crate btit_beads;` plus `mod logging;`; `mod issues;` and `mod test_support;` removed
- `crates/btit-app/src/cli.rs`: `pub(crate) use btit_beads::{detect::*, compat::*, gates::*};` at the top so `config.rs:1`, `updates.rs:1`, `migration.rs:4`, `issue_commands.rs:2`, `polling.rs:1`, `lib.rs:41-64` keep compiling unchanged; wrappers `supports_*()`/`uses_*()` call the moved `_for` cores
- `crates/btit-app/src/issue_commands.rs:3`, `polling.rs:2`, `attachments.rs:3`: `use crate::issues::…` → `use btit_beads::{issues::…, parse::parse_issues_tolerant}`
- `.github/workflows/ci.yml`: `rust-quality` job gains `btit-beads`
- `docs/architecture.md`: ADR-008 appended (after b-1's ADR-009)
- `docs/plans/phase-b/sprint-b-3.md` (`status:` frontmatter only)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Crate.** `crates/btit-beads` with `[lints] workspace = true`, `clippy.toml` test allowances, `#![deny(missing_docs)]`, `publish = false`. Dependencies: `btit-types`, `serde_json`, `log`. Nothing else (gate: `cargo tree -e normal -p btit-beads --depth 1` = `btit-beads`, `btit-types`, `log`, `serde_json`).
2. **Traits** exactly as in the code samples: `BeadsBackend`, `CliBackend`, `DoltOperations`, `CloseSuggestions`, in `src/backend.rs`, all `Send + Sync`, object-safe (`fn _assert(_: &dyn CliBackend) {}` in `tests/api_freeze.rs`). No implementation in this crate. Every method carries a rustdoc line citing the `a18c724` source it replaces (table in the plan's trait inventory).
3. **`BeadsError`** exactly as in the code samples and the error table in Required Work: `#[non_exhaustive] enum`, manual `Display` reproducing today's strings, `std::error::Error::source()` for variants carrying `io::Error`/`serde_json::Error`, `code()` and `remediation()` per variant. No `String` error type anywhere in the crate.
4. **Pure logic moved, not rewritten.** `detect.rs`: `CLI_CANDIDATES`, `CLI_FALLBACK`, `MIN_SUPPORTED_BD_MAJOR`, `CliSelection` (fields stay private; `binary()`/`probe()`/`is_legacy()` become `pub`), `is_legacy_bd`, `rank_cli_candidate`, `select_default_binary`, `parse_cli_probe`, `detect_cli_client`, `parse_bd_version`, `cli_client_name`. `compat.rs`: `cli_compatibility_warnings`. `gates.rs`: the five `_for` cores with today's `(CliClient, u32, u32, u32)` signatures plus `capabilities_for(client: CliClient, version: Option<CliVersion>) -> BackendCapabilities` (a `None` version yields all `false`, today's `None => false` arms). `issues.rs`: `priority_to_string`, `priority_to_number`, `normalize_issue_type`, `normalize_issue_status`, `transform_issue`, `normalize_metadata`. `parse.rs`: `parse_issues_tolerant` returning `Result<Vec<BdRawIssue>, BeadsError>` (`InvalidJson`, `UnexpectedShape`). Bodies are byte-identical except: `is_legacy_bd`/`rank_cli_candidate`/`cli_compatibility_warnings` take `Option<CliVersion>` where they took the tuple (call sites use `.into()`), and `parse_bd_version` returns `Option<CliVersion>` and replaces `parts[0]`, `parts[1]`, `parts[2]` (`cli.rs:314-317`) with `parts.get(n)` (the workspace `indexing_slicing = deny` lint). `log_*!` calls inside the moved code are unchanged.
5. **Tests moved with the code.** The cli.rs tests listed in Exact Targets and all 31 issues.rs tests move into the corresponding `#[cfg(test)] mod tests`, with `crate::test_support::*` fixtures (`probe`, `minimal_issue_json`, `issue_json_with_metadata`). Test bodies change only for tuple → `CliVersion` conversions. The plan's test-preservation gate passes.
6. **Log gate.** `src/logging.rs`: `pub static LOGGING_ENABLED: AtomicBool`, `pub static VERBOSE_LOGGING: AtomicBool`, `pub use log;` and the four `#[macro_export]` macros expanding to `$crate::logging::log::info!(..)` etc. under the same conditions as `logging.rs:36-66`. The app's `logging.rs` re-exports the statics (`pub(crate) use btit_beads::logging::{LOGGING_ENABLED, VERBOSE_LOGGING};`) so `get_logging_enabled`/`set_logging_enabled`/`get_verbose_logging`/`set_verbose_logging` (`logging.rs:166-191`) are unchanged.
7. **App consumes the crate.** `issues.rs`, `test_support.rs` deleted; `cli.rs` shrinks to the process-spawning and global-state parts (`get_extended_path`, `new_command`, `probe_cli_binary`, `extended_path_entries`, `default_cli_binary`, `get_cli_client_info`, the five wrappers, `project_uses_dolt(_for)`, `reset_bd_version_cache`, `execute_bd`, `check_bd_compatibility`, statics) plus re-exports. `cargo test --workspace` passes the full set.
8. **API freeze.** `crates/btit-beads/tests/api_freeze.rs` pins every public function signature (as `let _: fn(..) -> .. = path;` items), every trait method (via a `struct Probe; impl BeadsBackend for Probe { .. }` with `todo!()`-free bodies returning fixed values, under the test allowances), every `BeadsError` variant and the `code()` strings. b-9 must leave this file byte-identical.
9. **Contract doc.** `crates/btit-beads/docs/backend-contract.md`: the trait inventory (from the plan, with signatures) split into a **transport-neutral** list (`BeadsBackend`: `project_uses_dolt`, the issue operations, `relation_types`, `sync`, the three accessors; `DoltOperations` with `DoltOpResult`) and a **CLI-only** list (`CliBackend`: `binary`, `probe`, `client`, `version`, `capabilities`, `run_raw`, `release_source`; `CloseSuggestions`), the error table, the `ProjectRef` headroom note, the "sync trait, blocking calls" note, the rule that only `btit-app` depends on `tauri` and `sc-observability-log`, the "no process-global client state in library crates" rule (backends are instances; two may coexist for two projects), and a "future backend-specific traits" paragraph naming issue #51 (Dolt commit log / `AS OF` snapshots / `dolt_diff`) as bd-only headroom behind an accessor like `dolt()`, with no method planned; it states that a SQL transport can implement `dolt()` because `DoltOpResult` carries no process output.
10. **No global state.** `crates/btit-beads/src` declares no `static` other than `LOGGING_ENABLED` and `VERBOSE_LOGGING`; every function in `detect`, `compat`, `gates`, `issues`, `parse` is pure (inputs → outputs, plus `log_*!`).
11. **CI.** `rust-quality` runs fmt/clippy/rustdoc/`cargo tree` gates for `btit-beads` too (`-p btit-types -p btit-beads`).
12. **ADR-008.** `docs/architecture.md` gains ADR-008 "Beads backend contract" (Status: Accepted by maintainer direction 2026-09-13; Context: one Tauri crate with process-global CLI state, `config.rs:8-13`, `cli.rs:14-20`, and maintainer requirements 2–5; Decision: the trait family `BeadsBackend` / `CliBackend` / `DoltOperations` / `CloseSuggestions`, the optional-accessor pattern including `cli()`, the `CliBackend` split of `client`/`version`/`capabilities`, transport-neutral `DoltOperations` returning `DoltOpResult`, library crates hold no process-global client state, the app owns the single slot; Alternatives considered: traits inside `btit-types`, one fat trait returning `Unsupported`, `Any` downcasting from `dyn BeadsBackend`; Consequences: a SQL/DoltHub transport implements `BeadsBackend` (+ optionally `dolt()`) with no caller change, per-project selection is a slot change (OQ-8), #51 is a further accessor; Links: this sprint doc, `backend-contract.md`). The ADR table gets its row.

## Required Work

- **Error inventory** (authoritative; `Display` must reproduce the third column byte-for-byte):

  | Variant | Fields | `Display` (today's string, source) | `code()` | `remediation()` |
  |---|---|---|---|---|
  | `Spawn` | `binary: String`, `operation: Option<&'static str>`, `source: io::Error` | `operation = None`: `Failed to execute {binary}: {source}` (`cli.rs:563`); `Some(op)`: `Failed to run {binary} {op}: {source}` (`migration.rs:288`) | `BTIT_BEADS_SPAWN` | "Install the CLI or point Settings at its path; the searched directories are in check_bd_compatibility.searchedPaths." |
  | `CommandFailed` | `binary`, `status: Option<i32>`, `status_display: String` (`ExitStatus`'s `Display`), `stderr: String` | `stderr` when non-empty (`cli.rs:577`); else `bd command failed with status: {status_display}` (`cli.rs:579`, literal `bd`) | `BTIT_BEADS_COMMAND_FAILED` | "Read stderr; run the same command in a terminal from the project directory." |
  | `SchemaMigration` | `binary` | `SCHEMA_MIGRATION_ERROR: Database schema is incompatible. Please use the repair function to fix this issue.` (`cli.rs:573`; the frontend matches the prefix, `bd-api.ts:253,256`) | `BTIT_BEADS_SCHEMA_MIGRATION` | "Use Repair database (bd_repair_database)." |
  | `InvalidJson` | `context: String`, `source: serde_json::Error` | `Invalid JSON: {source}` (`issues.rs:256`) | `BTIT_BEADS_INVALID_JSON` | "Upgrade the CLI; the output is not JSON even with --json." |
  | `UnexpectedShape` | `context`, `expected: ExpectedShape` | `ExpectedShape::Array` → `Expected JSON array` (`issues.rs:275`); `ArrayOrEnvelope` → `Expected JSON array or paginated envelope` (`issues.rs:270`) | `BTIT_BEADS_UNEXPECTED_SHAPE` | "Upgrade the CLI; the JSON shape is not one btit knows." |
  | `ParseFailed` | `target: ParseTarget`, `id: Option<String>`, `source: serde_json::Error` | per `ParseTarget`: `Status` → `Failed to parse status: {e}` (`issue_commands.rs:152`); `Issue` with `id: None` → `Failed to parse issue: {e}` (`:186`), with `Some(id)` → `Failed to parse issue {id}: {e}` (`:202`); `CreatedIssue` → `Failed to parse created issue: {e}` (`:274`); `UpdatedIssueFetch` → `Failed to fetch updated issue: {e}` (`:362`); `UpdatedIssue` → `Failed to parse updated issue: {e}` (`:380`); `CloseResult` → `Failed to parse close result: {e}` (`:422`); `SearchResults` → `Failed to parse search results: {e}` (`:446`) | `BTIT_BEADS_PARSE_FAILED` | "Report the CLI version and the raw output from the log." |
  | `Unsupported` | `operation: &'static str`, `client: CliClient` | `{operation} is not supported by the {client} client` (new; used only where today's code cannot reach, e.g. Dolt repair on br) | `BTIT_BEADS_UNSUPPORTED` | "Switch the CLI binary in Settings." |

  Every variant is `Debug`; `source()` returns the inner `io::Error`/`serde_json::Error` where present.
- `parse_issues_tolerant` keeps its `context: &str` parameter and its log lines (`issues.rs:251-309`).
- The `_for` cores keep their doc comments, including the B7 comment at `cli.rs:409` ("br: NO") — b-9 owns its correction.
- Changelog lines (collated by b-12): "New crate `btit-beads`: the `BeadsBackend`/`CliBackend` contract shared by bd and br, the bd-only `DoltOperations` and br-only `CloseSuggestions` traits, the discriminated-union `BeadsError`, and the pure version/capability/issue logic."

## Explicit Code Samples

```toml
# crates/btit-beads/Cargo.toml
[package]
name = "btit-beads"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Beads backend contract (traits), BeadsError, and the pure beads logic shared by every backend."
publish = false

[dependencies]
btit-types = { path = "../btit-types" }   # or via [workspace.dependencies]
serde_json.workspace = true
log.workspace = true

[lints]
workspace = true
```

```rust
// crates/btit-beads/src/backend.rs
use std::path::Path;
use btit_types::{
    BackendCapabilities, BdRawIssue, CliClient, CliOutput, CliProbe, CliVersion, CreatePayload,
    DoltOpResult, ListQuery, ProjectRef, RelationType, ReleaseSource, UpdatePayload,
};
use crate::error::BeadsError;

/// Transport-neutral operations every beads backend supports. Implemented by `btit-bd` and
/// `btit-br`; a future SQL/DoltHub transport implements this trait alone (no CLI notion here).
pub trait BeadsBackend: Send + Sync {
    /// Whether the project's `.beads` is a Dolt project (today: `project_uses_dolt`, cli.rs:473-515,
    /// called with `<working_dir>/.beads` by every caller). br: always `false`.
    fn project_uses_dolt(&self, project: &ProjectRef) -> bool;

    fn list(&self, project: &ProjectRef, query: &ListQuery) -> Result<Vec<BdRawIssue>, BeadsError>;
    fn ready(&self, project: &ProjectRef) -> Result<Vec<BdRawIssue>, BeadsError>;
    fn status(&self, project: &ProjectRef) -> Result<serde_json::Value, BeadsError>;
    fn show(&self, project: &ProjectRef, id: &str) -> Result<Option<BdRawIssue>, BeadsError>;
    fn create(&self, project: &ProjectRef, payload: &CreatePayload) -> Result<BdRawIssue, BeadsError>;
    fn update(&self, project: &ProjectRef, id: &str, updates: &UpdatePayload) -> Result<Option<BdRawIssue>, BeadsError>;
    fn close(&self, project: &ProjectRef, id: &str) -> Result<serde_json::Value, BeadsError>;
    fn search(&self, project: &ProjectRef, query: &str) -> Result<Vec<BdRawIssue>, BeadsError>;
    fn label_add(&self, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError>;
    fn label_remove(&self, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError>;
    /// `delete <id> --force [--hard]`; the caller removes the attachment folder (issue_commands.rs:480-504).
    fn delete(&self, project: &ProjectRef, id: &str) -> Result<(), BeadsError>;
    fn comment_add(&self, project: &ProjectRef, id: &str, content: &str) -> Result<(), BeadsError>;
    /// `dep add <issue> <depends_on> [--type <t>]` (issue_commands.rs:520-522, 538-540).
    fn dep_add(&self, project: &ProjectRef, issue_id: &str, depends_on_id: &str, relation_type: Option<&str>) -> Result<(), BeadsError>;
    fn dep_remove(&self, project: &ProjectRef, issue_id: &str, depends_on_id: &str) -> Result<(), BeadsError>;
    /// Relation types offered to the UI (issue_commands.rs:555-581).
    fn relation_types(&self) -> Vec<RelationType>;
    /// `sync [--no-daemon]` without `--json` and without the project lock (migration.rs:222-232).
    /// A non-zero exit is `Err(CommandFailed)`; the caller keeps today's log/return text.
    fn sync(&self, project: &ProjectRef) -> Result<(), BeadsError>;

    /// CLI-only facts and raw invocations; `None` for a non-CLI transport.
    fn cli(&self) -> Option<&dyn CliBackend> { None }
    /// Dolt operations; `None` for backends without Dolt (br today).
    fn dolt(&self) -> Option<&dyn DoltOperations> { None }
    /// br-only `--suggest-next`; `None` for other backends.
    fn close_suggestions(&self) -> Option<&dyn CloseSuggestions> { None }
}

/// What only a spawned CLI has. Implemented by `btit-bd` and `btit-br`; not by remote transports.
pub trait CliBackend: BeadsBackend {
    /// Configured binary name or path (today `config::get_cli_binary`).
    fn binary(&self) -> &str;
    /// Fresh `<binary> --version` from the temp dir with the extended PATH (today `probe_cli_binary`, cli.rs:185-196).
    fn probe(&self) -> Option<CliProbe>;
    /// Detected client kind (today: `get_cli_client_info().0`, cli.rs:326-365); `Unknown` after a failed probe.
    fn client(&self) -> CliClient;
    /// Detected version, `None` when `--version` failed or did not parse.
    fn version(&self) -> Option<CliVersion>;
    /// The five version gates (today: `supports_*`/`uses_*` wrappers, cli.rs:380-466).
    fn capabilities(&self) -> BackendCapabilities;
    /// Raw invocation: `<binary> <args…>` in the project dir with `PATH` and `BEADS_PATH` set, no `--json`, no lock
    /// (the pattern at migration.rs:227-232, 282-288, 333-339, 403-408, 623-629, 665-671, 771-777, 879-885, 944-950, 1003-1009, 1081-1087).
    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError>;
    /// GitHub release location for `check_bd_cli_update` (updates.rs:285-292).
    fn release_source(&self) -> ReleaseSource;
}

/// Dolt operations (bd only today). Transport-neutral: returns what migration.rs reads
/// (exit status, trimmed stdout, trimmed stderr), never raw process output.
pub trait DoltOperations: Send + Sync {
    /// `doctor --fix --yes` (migration.rs:333-339)
    fn doctor_fix(&self, project: &ProjectRef) -> Result<DoltOpResult, BeadsError>;
    /// `migrate --to-dolt --yes` (migration.rs:623-629)
    fn migrate_to_dolt(&self, project: &ProjectRef) -> Result<DoltOpResult, BeadsError>;
    /// `init --prefix <prefix>` (migration.rs:665-671, 771-777)
    fn init(&self, project: &ProjectRef, prefix: &str) -> Result<DoltOpResult, BeadsError>;
    /// `import -i <file>` (migration.rs:879-885)
    fn import_jsonl(&self, project: &ProjectRef, file: &Path) -> Result<DoltOpResult, BeadsError>;
}

/// br-only: `close <id> --suggest-next` (issue_commands.rs:410-413).
pub trait CloseSuggestions: Send + Sync {
    fn close_suggesting_next(&self, project: &ProjectRef, id: &str) -> Result<serde_json::Value, BeadsError>;
}
```

```rust
// crates/btit-beads/src/error.rs
use btit_types::CliClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedShape { Array, ArrayOrEnvelope }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseTarget { Status, Issue, CreatedIssue, UpdatedIssue, UpdatedIssueFetch, CloseResult, SearchResults }

#[derive(Debug)]
#[non_exhaustive]
pub enum BeadsError {
    Spawn { binary: String, operation: Option<&'static str>, source: std::io::Error },
    CommandFailed { binary: String, status: Option<i32>, status_display: String, stderr: String },
    SchemaMigration { binary: String },
    InvalidJson { context: String, source: serde_json::Error },
    UnexpectedShape { context: String, expected: ExpectedShape },
    ParseFailed { target: ParseTarget, id: Option<String>, source: serde_json::Error },
    Unsupported { operation: &'static str, client: CliClient },
}

impl BeadsError {
    pub fn code(&self) -> &'static str { /* table in Required Work */ }
    pub fn remediation(&self) -> &'static str { /* table in Required Work */ }
}
impl std::fmt::Display for BeadsError { /* table in Required Work, byte-exact */ }
impl std::error::Error for BeadsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn { source, .. } => Some(source),
            Self::InvalidJson { source, .. } | Self::ParseFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}
```

```rust
// crates/btit-beads/src/gates.rs (addition; the five `_for` cores move verbatim)
pub fn capabilities_for(client: CliClient, version: Option<CliVersion>) -> BackendCapabilities {
    match version {
        None => BackendCapabilities::default(),               // today's `None => false` arms
        Some(v) => {
            let (ma, mi, pa) = v.into();
            BackendCapabilities {
                supports_daemon_flag: supports_daemon_flag_for(client, ma, mi, pa),
                uses_jsonl_files: uses_jsonl_files_for(client, ma, mi, pa),
                uses_dolt_backend: uses_dolt_backend_for(client, ma, mi, pa),
                supports_list_all_flag: supports_list_all_flag_for(client, ma, mi, pa),
                supports_delete_hard_flag: supports_delete_hard_flag_for(client, ma, mi, pa),
            }
        }
    }
}
```

```rust
// crates/btit-beads/src/logging.rs
use std::sync::atomic::AtomicBool;
pub use log;
pub static LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);
pub static VERBOSE_LOGGING: AtomicBool = AtomicBool::new(false);

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        if $crate::logging::LOGGING_ENABLED.load(::std::sync::atomic::Ordering::Relaxed) {
            $crate::logging::log::info!($($arg)*);
        }
    };
}
// log_warn!, log_error! identical with warn!/error!; log_debug! additionally requires VERBOSE_LOGGING (logging.rs:60-66)
```

```rust
// crates/btit-app/src/lib.rs (top)
#[macro_use]
extern crate btit_beads;   // brings log_info!/log_warn!/log_error!/log_debug! into textual scope, as `#[macro_use] mod logging;` did
mod logging;
```

## This Sprint Does Not Close

- Any `impl BeadsBackend` (b-5, b-6) or the process-spawning transport (b-4).
- Removal of the app's global-state wrappers, `execute_bd`, `project_uses_dolt(_for)` (b-7, b-8).
- Behaviour fixes to the moved pure functions (b-9).

## Acceptance Criteria

1. `cargo tree -e normal -p btit-beads --depth 1` lists exactly `btit-types`, `log`, `serde_json`; `! grep -rnE 'std::(process|fs)|tauri|sc_observability' crates/btit-beads/src`; `grep -rnE '^\s*(pub(\(crate\))? )?static ' crates/btit-beads/src` lists only `LOGGING_ENABLED` and `VERBOSE_LOGGING`.
2. Every trait, method and `BeadsError` variant in the code samples exists with the stated signature; `tests/api_freeze.rs` compiles; `fn _obj(_: &dyn CliBackend) {}` compiles (object safety).
3. For every row of the error inventory, a unit test constructs the variant and asserts the exact `Display` string and `code()`; `SchemaMigration`'s string starts with `SCHEMA_MIGRATION_ERROR`.
4. `git diff -M origin/integrate/phase-b...HEAD` shows the moved functions as renames/moves; the only content deltas inside moved production code are the `CliVersion`/`.into()` adaptations and the `parts.get(n)` change in `parse_bd_version`, listed in the PR description.
5. `crates/btit-app/src/issues.rs` and `test_support.rs` do not exist; `cli.rs` contains no function from the Exact Targets "moved" list; the app's `logging.rs` defines no macro and no `AtomicBool`.
6. `cargo test --workspace` passes; the test-preservation gate prints nothing.
7. `cargo clippy -p btit-beads --all-targets -- -D warnings` and `cargo rustdoc -p btit-beads -- -D missing-docs` pass; `! grep -rnE 'allow\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)' crates/btit-beads/src`.
8. `crates/btit-beads/docs/backend-contract.md` exists with the sections in Deliverable 9, including the transport-neutral vs CLI-only method lists.
9. `grep -c '^### ADR-008' docs/architecture.md` is `1` and the ADR table lists ADR-008 (Deliverable 12); `tests/api_freeze.rs` pins `BeadsBackend::cli`, `dolt`, `close_suggestions` default bodies returning `None`.
10. CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-types -p btit-beads`
- `cargo clippy -p btit-beads --all-targets -- -D warnings`
- `cargo rustdoc -p btit-beads -- -D missing-docs`
- `cargo test --workspace`
- `cargo check --workspace --all-targets`
- `cargo tree -e normal -p btit-beads --depth 1 --prefix none --format '{p}' | sed -E 's/ v.*//' | sort | diff - <(printf 'btit-beads\nbtit-types\nlog\nserde_json\n')`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate (plan "Test preservation")
