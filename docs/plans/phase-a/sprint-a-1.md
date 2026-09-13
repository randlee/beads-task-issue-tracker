---
id: a-1
title: sc-observability-log crate — log bridge
status: planned
branch: feature/sprint-a-1-log-bridge
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-a-1-log-bridge
target: develop
recommended_model: higher-effort (global logger lifecycle, cross-platform CI)
dependency_relations:
  - prerequisite: PR #36
    dependent: a-1
    relation: must_follow
    rationale: "sc-observability 1.2.0 needs Rust >= 1.94.1; PR #36 pins 1.98.1 in rust-toolchain.toml and CI"
  - prerequisite: a-1
    dependent: a-2
    relation: must_follow
    rationale: "a-2 macros expand to a-1 __private::{enabled, emit}"
  - prerequisite: a-1
    dependent: a-4
    relation: must_follow
    rationale: "a-4 consumes the a-1 init API and frozen runtime dependency graph; a-4 starts after the a-1 PR merges"
---

# Sprint a-1 — sc-observability-log crate: log bridge

## Recommended Agent / Model

Recommended model: higher-effort (global logger lifecycle, cross-platform CI).
Recommended agent: not set — the btit developer pool is pending the `arch-ctm` decision on PR #38.
Planning advice; team-lead assigns from the active pool.

## Goal

- Create the transitional `crates/` Cargo workspace. Add the `sc-observability-log` library crate to it, built to sc-observability's standards.
- Deliver a `log::Log` implementation. It maps every `log` record to a `sc_observability_types::LogEvent` and writes it through a single process-wide `sc_observability::Logger`.
- Give a-2 and a-3 one shared emit path to use.

## Hard Dependencies

- PR #36 (`chore/toolchain-and-version-ssot`) merged to `develop`: Rust 1.98.1 pinned in `rust-toolchain.toml` at the repo root.
- Published crates: `sc-observability = "1.2.0"`, `sc-observability-types = "1.2.0"`.

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA;
merge parent → child before every dev/fix round. PR-completion trigger: parent
PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- PR #36 → a-1 — `must_follow` (a-1 follows PR #36): sc-observability 1.2.0 needs Rust >= 1.94.1; PR #36 pins 1.98.1 in rust-toolchain.toml and CI
- a-1 → a-2 — `must_follow` (a-2 follows a-1): a-2 macros expand to a-1 __private::{enabled, emit}
- a-1 → a-4 — `must_follow` (a-4 follows a-1): a-4 consumes the a-1 init API and frozen runtime dependency graph; a-4 starts after the a-1 PR merges

Stack: `phase-a-core · layer 1 (trunk develop)`.

## Exact Targets

- `crates/Cargo.toml` (new; workspace root)
- `crates/Cargo.lock` (new; committed)
- `crates/sc-observability-log/Cargo.toml` (new)
- `crates/sc-observability-log-macros/Cargo.toml` (new; `proc-macro = true`)
- `crates/sc-observability-log-macros/src/lib.rs` (new; crate doc only, no exported macros until a-2)
- `crates/runtime-deps.txt` (new; frozen runtime dependency graph)
- `crates/sc-observability-log/src/lib.rs` (new)
- `crates/sc-observability-log/src/bridge.rs` (new)
- `crates/sc-observability-log/src/mapping.rs` (new)
- `crates/sc-observability-log/src/handle.rs` (new)
- `crates/sc-observability-log/tests/bridge_jsonl.rs` (new)
- `crates/sc-observability-log/README.md` (new)
- `crates/sc-observability-log/docs/mapping.md` (new)
- `.github/workflows/ci.yml` (new `crates` job)
- `.gitignore` (add `crates/target/`)

## Deliverables

Every deliverable must land production-ready for the scope this sprint claims. If that cannot be done cleanly in one sprint, split the sprint before implementation begins. No deliverable may be dropped or partly deferred.

1. **Workspace:** `crates/Cargo.toml` with `members = ["sc-observability-log", "sc-observability-log-macros"]` and `resolver = "3"`.
   - `[workspace.package]`: `version = "0.1.0"`, `edition = "2024"`, `rust-version = "1.94.1"`, `license = "MIT"`.
   - `[workspace.dependencies]` pins `sc-observability = "1.2.0"`, `sc-observability-types = "1.2.0"`, `log = { version = "0.4", features = ["kv"] }`, `serde = "1"`, `serde_json = "1"`, `thiserror = "2"`, `syn = { version = "2", features = ["full"] }`, `quote = "1"`, `proc-macro2 = "1"`, and `sc-observability-log-macros = { path = "sc-observability-log-macros" }`.
2. **Crates and frozen runtime dependency graph:** both crates inherit the workspace package fields and set `publish = false`. Their `[dependencies]` are final for phase-a, so the graph btit resolves in a-4 does not change when a-2 or a-3 merge (`parallel_safe` a-4).
   - `sc-observability-log` runtime dependencies: exactly `sc-observability`, `sc-observability-types`, `log`, `serde`, `serde_json`, `thiserror`, `sc-observability-log-macros`. Dev dependencies: exactly `tempfile`.
   - `sc-observability-log-macros` runtime dependencies: exactly `syn`, `quote`, `proc-macro2`. No exported macros yet; a-2 and a-3 add them.
   - `crates/runtime-deps.txt` holds the sorted output of `cargo tree --manifest-path crates/Cargo.toml -p sc-observability-log -e normal --prefix none`. The `crates` CI job fails if the live output differs.
3. **Public API:** `init`, `BridgeOptions`, `LogGuard` and `InitError`, exactly as in the code samples. There is one global handle; a second `init` returns `InitError::AlreadyInstalled`.
4. **Mapping:** the pure function `record_to_event`, as specified in the mapping table below.
5. **Hidden emit path:** `__private::{emit, enabled}` (`#[doc(hidden)]`). It is the only way events reach the `Logger`, and a-2 and a-3 use it.
6. **Non-blocking delivery:** events are sent with `Logger::try_log`. When `TryLogError::QueueFull` or another error occurs, the event is dropped and counted. `LogGuard::dropped_events()` reports the count. Logging never blocks and never panics.
7. **Guard:** `LogGuard` flushes on `flush()`. On `Drop` it shuts down the logger and uninstalls the handle, so later records are dropped silently.
8. **Docs:** `README.md` (quick start) and `docs/mapping.md` (the mapping table, verbatim from this sprint).
9. **CI:** a new `crates` job in btit `.github/workflows/ci.yml` on `ubuntu-latest`, `windows-latest`, `macos-latest`. It runs every command listed under Required Validation for the `crates/` workspace, including the runtime-dependency check.

## Required Work

- **Record mapping** (implemented in `mapping.rs`):

  | `log::Record` | `LogEvent` |
  |---|---|
  | `level()` Error/Warn/Info/Debug/Trace | `Level::Error/Warn/Info/Debug/Trace` |
  | `target()` | `target`: `::` → `.`; each char outside `[A-Za-z0-9._-]` → `_`; empty → `log` |
  | leading `[tag]` of `args()` when `options.parse_bracket_action` and `tag` matches `[A-Za-z0-9._-]+` | `action = tag`; the tag and one following space are removed from `message` |
  | otherwise | `action = options.default_action` |
  | formatted `args()` | `message = Some(..)` |
  | `key_values()` (the `log` `kv` feature) | `fields[key] = serde_json::Value` (numbers, bools and strings typed; others `to_string()`) |
  | `module_path()`, `file()`, `line()` | `fields["code.module"]`, `fields["code.file"]`, `fields["code.line"]` (omitted when `None`) |
  | none | `trace`, `request_id`, `correlation_id`, `outcome`, `diagnostic`, `state_transition` = `None` |

- **Service and timestamp:** `service` comes from the `LoggerConfig.service_name` passed to `init`. `timestamp` is `Timestamp::now_utc()`, and `version` is the envelope version.
- **Level filtering:** `log::set_max_level` is set from `options.max_level`, and `Log::enabled` honors it.
- **Global state:** it lives in `handle.rs` as `static HANDLE: OnceLock<Mutex<Option<Logger<Running>>>>`. The `Mutex` requires only `Send`, so no `Sync` bound is placed on `Logger`.
- **Tests:**
  - Unit tests cover every mapping row, including invalid targets and tags, an empty target, a tag without a space, and `kv` values of each JSON type.
  - `tests/bridge_jsonl.rs` initializes into a `tempfile` log root and logs one record at each level, plus one tagged record and one `kv` record. It then flushes and reads the records back with `Logger::query` or the JSONL file. It asserts `target`, `action`, `message` and `fields`, and that a second `init` fails with `AlreadyInstalled`.

## Explicit Code Samples

```rust
// crates/sc-observability-log/src/lib.rs
pub use sc_observability::LoggerConfig;
// Re-exported so consumers (btit in a-4) need no direct sc-observability-types dependency.
pub use sc_observability_types::{ActionName, LevelFilter, ServiceName, TargetCategory};
// `Level` is intentionally NOT re-exported: a-2 defines a tracing-style
// `sc_observability_log::Level` (associated consts TRACE..ERROR).

pub struct BridgeOptions {
    /// Action used when a record carries no leading `[tag]`.
    pub default_action: ActionName,
    /// Strip a leading `[tag] ` from the message into `LogEvent.action`.
    pub parse_bracket_action: bool,
    /// Maximum level admitted by the `log` facade.
    pub max_level: log::LevelFilter,
}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error(transparent)]
    Logger(#[from] sc_observability_types::InitError),
    #[error("a global logger is already installed")]
    AlreadyInstalled,
}

#[must_use = "dropping the guard shuts the logger down"]
pub struct LogGuard { /* private */ }

impl LogGuard {
    pub fn flush(&self) -> Result<(), sc_observability_types::FlushError>;
    pub fn dropped_events(&self) -> u64;
}
impl Drop for LogGuard { /* flush, shutdown, uninstall handle */ }

pub fn init(config: LoggerConfig, options: BridgeOptions) -> Result<LogGuard, InitError>;

// crates/sc-observability-log/src/mapping.rs
pub fn record_to_event(
    record: &log::Record<'_>,
    service: &ServiceName,
    options: &BridgeOptions,
) -> sc_observability_types::LogEvent;

#[doc(hidden)]
pub mod __private {
    pub fn enabled(level: sc_observability_types::Level) -> bool;
    pub fn emit(event: sc_observability_types::LogEvent);
}
```

## This Sprint Does Not Close

- Event macros (a-2) and `#[instrument]` (a-3).
- Any btit `src-tauri/` or `app/` change, including removing `tauri-plugin-log` (a-4).
- Adding the crate to `../sc-observability` or publishing it (a-5 / sc-observability release).
- Ambient span or trace context for bridge records. Records carry no `trace`.

## Acceptance Criteria

1. `crates/` is a standalone Cargo workspace, and this sprint's diff touches no file under `src-tauri/` or `app/`.
2. `sc-observability-log` declares exactly the runtime and dev dependency sets in Deliverable 2, and uses edition 2024 with `rust-version = "1.94.1"`.
3. The public API matches the code samples (names, signatures, error variants). Nothing is exported beyond them except `#[doc(hidden)] __private`.
4. Every row of the mapping table has at least one unit test. Invalid or empty targets and tags never panic, and produce a valid `TargetCategory`/`ActionName`.
5. The integration test proves the JSONL on disk contains the mapped `target`, `action`, `message` and `fields` for tagged, untagged and `kv` records at all five levels.
6. A second `init` returns `InitError::AlreadyInstalled`. Records logged after `LogGuard` is dropped do not panic and are not written.
7. Logging while the queue is full does not block. `dropped_events()` increases, and this is covered by a test with `queue_capacity` set to the smallest accepted value.
8. The `crates` CI job passes on ubuntu, windows and macOS.
9. `crates/runtime-deps.txt` exists and the CI runtime-dependency check passes. Both crates declare exactly the runtime dependency sets in Deliverable 2.

## Required Validation

Run from the repo root:

- `cargo fmt --check --all --manifest-path crates/Cargo.toml`
- `cargo clippy --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings`
- `cargo test --manifest-path crates/Cargo.toml --workspace`
- `diff <(cargo tree --manifest-path crates/Cargo.toml -p sc-observability-log -e normal --prefix none | sort -u) crates/runtime-deps.txt`
- `cargo +1.94.1 check --manifest-path crates/Cargo.toml --workspace --all-targets` (MSRV proof for sc-observability)
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --manifest-path crates/Cargo.toml --target x86_64-pc-windows-msvc --workspace --all-targets` (local Windows cross-check; CI windows runner is authoritative)
