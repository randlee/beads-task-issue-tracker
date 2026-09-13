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
    rationale: "sc-observability 1.2.0 needs Rust >= 1.94.1; PR #36 pins 1.98.1 in rust-toolchain.toml and CI (satisfied: merged, present at baseline develop@8554294)"
  - prerequisite: a-1
    dependent: a-2
    relation: must_follow
    rationale: "a-2 macros expand to a-1 __private::{EventParts, enabled, emit, record_drop} and label through the a-1 mapping.rs sanitizer (__private::{target_label, action_label, field_key_label})"
  - prerequisite: a-1
    dependent: a-4
    relation: must_follow
    rationale: "a-4 consumes the frozen a-1 public API and runtime dependency graph; a-4 starts after the a-1 PR merges"
---

# Sprint a-1 — sc-observability-log crate: log bridge

## Recommended Agent / Model

Recommended model: higher-effort (global logger lifecycle, cross-platform CI).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Create the transitional `crates/` Cargo workspace. Add the `sc-observability-log` library crate to it, built to sc-observability's standards.
- Deliver a `log::Log` implementation. It maps every `log` record to a `sc_observability_types::LogEvent` and writes it through a single process-wide `sc_observability::Logger`.
- Give a-2 and a-3 one shared emit path, and freeze the public API that a-4 consumes.

## Hard Dependencies

- PR #36 (`chore/toolchain-and-version-ssot`) — **satisfied**: merged to `develop` and present at the phase baseline `develop@8554294`. Rust 1.98.1 is pinned in `rust-toolchain.toml` at the repo root.
- Published crates: `sc-observability = "1.2.0"`, `sc-observability-types = "1.2.0"`.

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA;
merge parent → child before every dev/fix round. PR-completion trigger: parent
PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- PR #36 → a-1 — `must_follow` (a-1 follows PR #36): sc-observability 1.2.0 needs Rust >= 1.94.1; PR #36 pins 1.98.1 in rust-toolchain.toml and CI. Satisfied.
- a-1 → a-2 — `must_follow` (a-2 follows a-1): a-2 macros expand to a-1 `__private::{EventParts, enabled, emit, record_drop}` and label through the a-1 `mapping.rs` sanitizer (`__private::{target_label, action_label, field_key_label}`).
- a-1 → a-4 — `must_follow` (a-4 follows a-1): a-4 consumes the frozen a-1 public API and runtime dependency graph; a-4 starts after the a-1 PR merges.

Stack: `phase-a-core · layer 1 (trunk develop)`.

## Exact Targets

- `crates/Cargo.toml` (new; workspace root)
- `crates/Cargo.lock` (new; committed)
- `crates/runtime-deps.txt` (new; frozen runtime dependency graph)
- `crates/sc-observability-log/Cargo.toml` (new)
- `crates/sc-observability-log/src/lib.rs` (new)
- `crates/sc-observability-log/src/bridge.rs` (new)
- `crates/sc-observability-log/src/mapping.rs` (new; mapping and the single label sanitizer)
- `crates/sc-observability-log/src/handle.rs` (new; global slot, `record_drop`, `EmitScope`, `submit_guarded`, `run_bounded`, `take_sole`, shutdown and flush helpers)
- `crates/sc-observability-log/src/error.rs` (new; `InitError`, `FlushError`, `ShutdownError`, `DropCause`)
- `crates/sc-observability-log/src/error_codes.rs` (new)
- `crates/sc-observability-log/clippy.toml` (new; test allowances)
- `crates/sc-observability-log-macros/clippy.toml` (new; test allowances)
- `crates/sc-observability-log/tests/bridge_jsonl.rs` (new; one `init` test)
- `crates/sc-observability-log/tests/bridge_queue_full.rs` (new; one `init` test with `queue_capacity = 1`)
- `crates/sc-observability-log/tests/api_freeze.rs` (new; compile-time signature lock, no `init` call)
- `crates/sc-observability-log/README.md` (new)
- `crates/sc-observability-log/docs/mapping.md` (new)
- `crates/sc-observability-log-macros/Cargo.toml` (new; `proc-macro = true`)
- `crates/sc-observability-log-macros/src/lib.rs` (new; crate doc only, no exported macros until a-2)
- `.github/workflows/ci.yml` (new `crates` job; `'feature/**'` added to `on.pull_request.branches`)
- `.gitignore` (add `crates/target/`)
- `docs/plans/phase-a/sprint-a-1.md` (`status:` frontmatter only)

## Deliverables

Every deliverable must land production-ready for the scope this sprint claims. If that cannot be done cleanly in one sprint, split the sprint before implementation begins. No deliverable may be dropped or partly deferred.

1. **Workspace:** `crates/Cargo.toml` exactly as in the manifest code sample: members, `resolver = "3"`, `[workspace.package]`, `[workspace.dependencies]` and `[workspace.lints]`. The lints are sc-observability's set (`../sc-observability/Cargo.toml`: `missing_debug_implementations`, `unsafe_op_in_unsafe_fn`, clippy `pedantic`, all `warn`; `-D warnings` makes them errors) plus the **no-panic set** at `deny`: `unwrap_used`, `expect_used`, `panic`, `unreachable`, `todo`, `unimplemented`, `indexing_slicing`. `pedantic` carries `priority = -1` so the individual lints override the group. Each crate has a `clippy.toml` with `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-panic-in-tests` and `allow-indexing-slicing-in-tests` set to `true`; integration-test files additionally start with a file-level `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, reason = "…")]`, because the clippy.toml allowances cover only `#[test]` fns and `#[cfg(test)]` code, not helper fns in `tests/*.rs` (all verified on clippy 1.94.1 and 1.98.1 while planning).
2. **Crates and frozen runtime dependency graph:** both crates inherit the workspace package fields and set `description`, `publish = false` and `[lints] workspace = true`. Their `[dependencies]` are final for phase-a, so the graph btit resolves in a-4 does not change when a-2 or a-3 merge (`parallel_safe` a-4).
   - `sc-observability-log` runtime dependencies: exactly `sc-observability`, `sc-observability-types`, `log`, `serde`, `serde_json`, `thiserror`, `sc-observability-log-macros`. Dev dependencies: exactly `tempfile`.
   - `sc-observability-log-macros` runtime dependencies: exactly `syn`, `quote`, `proc-macro2`. No exported macros yet; a-2 and a-3 add them.
   - **Lockstep pin:** `sc-observability-log` depends on `sc-observability-log-macros` through an exact `=` version plus path, following the `serde` → `serde_core = "=1.0.228"` precedent (`serde-1.0.228/Cargo.toml`, `[dependencies.serde_core]`). The `#[doc(hidden)] __private` module is outside semver; the exact pin is what keeps macro expansions and `__private` in lockstep. `README.md` states this policy.
   - `crates/runtime-deps.txt` is the output of the runtime-dependency command in Required Validation. The command is host-independent (`--target all`, so `windows-sys` is listed on every host), strips path-source suffixes such as ` (/abs/path)` and ` (proc-macro)`, and de-duplicates with `LC_ALL=C sort -u`.
3. **Public API:** `init`, `BridgeOptions`, `LogGuard`, `InitError`, `FlushError`, `ShutdownError`, `DropCause`, `DroppedEvents`, `DEFAULT_DROP_SHUTDOWN_TIMEOUT`, the `error_codes` module and the re-exports, exactly as in the code samples.
4. **Errors are discriminated unions:** every error type in the crate, public or internal, is an `enum` whose variants carry typed fields (`#[source]` errors, `Duration`s), so callers `match` on the cause. No opaque `struct X(Box<ErrorContext>)` wrapper and no `String` error is used. For destination compatibility each public error enum (`InitError`, `FlushError`, `ShutdownError`) also exposes `code(&self) -> ErrorCode` (one stable code per variant, from `error_codes.rs`) and `remediation(&self) -> Remediation` (mandatory in sc-observability, `../sc-observability/docs/api-design.md:67`); variants that wrap an sc-observability error return that error's own code and remediation. The crate-private `BoundedError` and `ShutdownStep` and the `#[doc(hidden)]` `LabelError` never reach users (each is mapped to a public error or counted as a `DropCause`), so they carry no code. This deliberately differs from sc-observability's `error_wrapper!` structs (`sc-observability-types/src/errors.rs:19-51`) and is an explicit a-5 review item.
5. **Single level threshold:** the only level setting is `LoggerConfig.level`. `init` derives `log::set_max_level` and the `__private::enabled` threshold from it. `BridgeOptions` has no level field.
6. **Process identity:** `init` resolves `LoggerConfig.process_identity` once. This is required because sc-observability 1.2.0 stores the policy but never applies it: `Logger::prepare_event` only validates, filters and redacts (`runtime.rs:247-253`), and `LogEvent.identity` is a required field (`events.rs:90-91`). Resolution: `Auto` → `ProcessIdentity { hostname: None, pid: Some(std::process::id()) }`; `Fixed { hostname, pid }` → as given; `Resolver(r)` → `r.resolve()`, where an `Err` fails `init` with `InitError::IdentityResolution`. Every emitted `LogEvent.identity` carries the cached value.
7. **Mapping and the single label sanitizer:** the crate-private pure functions `record_to_parts` and `assemble_event`, specified by the mapping table below, and the label sanitizer in `mapping.rs`, which is the only sanitizer in phase-a (code sample below):
   - `sanitize_label(raw: &str) -> Cow<'_, str>` rewrites `::` to `.` and every char outside `[A-Za-z0-9._-]` to `_`. It borrows when `raw` is already valid and never fails.
   - `target_label(raw) -> Result<TargetCategory, LabelError>` sanitizes and maps an empty result to `log`.
   - `action_label(raw) -> Result<ActionName, LabelError>` sanitizes and rejects an empty result with `LabelError::Empty`.
   - `field_key_label(raw) -> Result<Cow<'_, str>, LabelError>` sanitizes, rejects an empty result with `LabelError::Empty`, and rejects a result starting with `RESERVED_FIELD_PREFIX` (`"sc_observability_log."`) with `LabelError::ReservedPrefix`.
   - A `TargetCategory::new`/`ActionName::new` error after sanitizing is `LabelError::Rejected { source }`. It cannot occur for sanitized input, but it is returned, not unwrapped.
   - The bridge labels `Record::target()` with `target_label`. a-2 and a-3 call the same functions through `__private` at runtime, the first time a callsite is used, and cache the result in the `Callsite` `OnceLock`. This covers literal and non-literal `target`/`name` (const, `module_path!()`, `concat!`) and every `{ expr } = v` field key. The proc-macro crate does not reimplement sanitization. A label that fails is counted with `record_drop(DropCause::InvalidEvent)`; a failing field key omits only that field and the event still emits.
8. **Hidden emit path:** `__private::{EventParts, enabled, emit, record_drop}` plus the sanitizer re-exports `__private::{sanitize_label, target_label, action_label, field_key_label, LabelError, LabelKind, RESERVED_FIELD_PREFIX}` (`#[doc(hidden)]`). `emit` is the only way events reach the `Logger`; a-2 and a-3 use it. `handle.rs` defines `pub(crate) fn record_drop(cause: DropCause)`, which increments the counter for `cause` (one `AtomicU64` per variant, selected by `match`); `__private::record_drop(cause: DropCause)` is a one-line `pub fn` wrapper around it for macro expansions (a `pub use` of a `pub(crate)` fn does not compile, E0364).
9. **Non-blocking, non-panicking, non-reentrant emit** (code sample below):
   - **Slot:** the global slot is a `RwLock<Option<Arc<Installed>>>`. `emit` holds the read lock only to clone the `Arc` (no I/O and no queue wait while it is held), then calls `Logger::try_log`. Every lock acquisition recovers from poisoning with `PoisonError::into_inner` (the slot holds only an `Option<Arc<_>>`, which has no invariant a panic can break).
   - **Drop accounting:** every dropped event is counted under exactly one `DropCause` variant through `record_drop`. The four `TryLogError` variants map 1:1, and an empty slot is `NotInstalled`.
   - **Panics:** sc-observability's `try_log` contains `expect` calls (`runtime.rs:128`, and `record_last_error` at `runtime.rs:392-398`). `emit` runs the slot read, `assemble_event` and `try_log` inside `submit_guarded`, which calls them as `std::panic::catch_unwind(AssertUnwindSafe(submit))` and counts a caught panic as `LoggerPanicked`.
   - **Why `AssertUnwindSafe`:** it is required because `Logger` holds `dyn LogSink`, `dyn Redactor` and `dyn ProcessIdentityResolver`, which are not `RefUnwindSafe`. Without the wrapper the closure fails with E0277 (compiled on 1.94.1). The assertion is sound: after a caught panic the bridge reads no logger state except by calling `try_log` again, and sc-observability reports its own poisoned mutexes by panicking again, which is caught and counted the same way.
   - **Reentrancy:** `submit_guarded` first enters an `EmitScope`, a guard over `thread_local! { static IN_EMIT: Cell<bool> = const { Cell::new(false) } }`. The flag is read and written only with `LocalKey::try_with`, never `with`, and it is cleared in `Drop`, so it is also cleared while a panic unwinds. A call to `emit` while the flag is set on the same thread returns at once and is counted as `ReentrantEmit`. Examples are a panic hook that logs while `try_log` panics, or a sink or redactor that logs through `log`. A `try_with` `AccessError` is also treated as reentrant. It cannot occur for a `const` `Cell<bool>`, which has no destructor, but it is handled without panicking.
10. **No panics in this crate:** production code in both crates contains no `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` or panicking indexing/slicing, enforced by the deny lints in Deliverable 1. Poisoned mutexes are recovered explicitly, never `.lock().unwrap()`. (`catch_unwind` has no effect under `panic = "abort"`; that residual is documented in `README.md`.)
11. **Bounded flush and shutdown** (code sample below):
    - **Why bounded:** the sc-observability calls are unbounded. `WriterRuntime::flush` does a blocking `send` then an untimed `recv` (`maintenance.rs:137-164`), and `WriterRuntime::shutdown` joins the writer unconditionally after its `recv_timeout` (`maintenance.rs:166-234`).
    - **Helper thread:** `LogGuard::flush(timeout)` and `LogGuard::shutdown(self, timeout)` run that work through `run_bounded` on a helper thread named `sc-observability-log-helper` and wait with `recv_timeout(timeout)`. On timeout they return the `TimedOut { timeout }` variant and detach the helper, so the caller is bounded by `timeout`.
    - **Flush:** `flush` clones the `Arc<Installed>` out of the slot and moves the clone into its helper. A helper detached by a timed-out flush keeps that clone until sc-observability's flush returns.
    - **Sole ownership for shutdown:** `Logger::shutdown(self)` consumes the logger (`runtime.rs:224`), so shutdown must own `Installed` outright. `shutdown` sets the threshold and `log::set_max_level` to `Off` and takes the `Arc` out of the slot. Its helper then calls `take_sole(arc, deadline)`, where `deadline = Instant::now().checked_add(timeout)`. `take_sole` retries `Arc::try_unwrap` with a sleep backoff (1 ms doubling to 50 ms, never sleeping past the deadline). Clones held by in-flight `emit` calls are released within one `try_log`. At the deadline `take_sole` drops its clone and the helper returns `ShutdownStep::StillShared`, so the helper does not spin forever. With sole ownership the helper flushes, then calls `Logger::shutdown`.
    - **Timeout outcome:** both `StillShared` and an outer `recv_timeout` expiry return `ShutdownError::TimedOut { timeout }`. A helper detached by an earlier timed-out `flush` still holds a clone, so a later `shutdown` cannot gain sole ownership until that flush returns and reports `TimedOut`.
    - **Drop:** `Drop` runs the same sequence with `DEFAULT_DROP_SHUTDOWN_TIMEOUT` unless `shutdown` already consumed the guard, and ignores the result. Each guard runs the flush-and-shutdown sequence exactly once.
12. **Install-once semantics** (code sample below):
    - **Entry claim:** `init` claims the process with `INSTALLED.compare_exchange(false, true, SeqCst, SeqCst)` before doing any work. A failed exchange returns `InitError::AlreadyInitialized` at once. This covers a guard that is alive, a guard already shut down (the `log` facade logger cannot be uninstalled), and an own `init` running concurrently on another thread. A concurrent second own `init` therefore reports `AlreadyInitialized`, never `ForeignLoggerInstalled`, because it never reaches `log::set_boxed_logger`.
    - **Reset on recoverable failure:** when identity resolution or `Logger::new` fails, `INSTALLED` is reset to `false` before returning `IdentityResolution` or `Logger`, so the caller can fix the configuration and call `init` again.
    - **Kept set after `ForeignLoggerInstalled`:** if `log::set_boxed_logger` fails, `init` shuts down the `Logger` it built (bounded by `DEFAULT_DROP_SHUTDOWN_TIMEOUT`, result ignored) and returns `InitError::ForeignLoggerInstalled { source }`. `INSTALLED` stays set, because the `log` facade slot belongs to the other logger for the rest of the process, so no retry can succeed. Later calls return `AlreadyInitialized` without building and tearing down another `Logger`.
    - **Install order:** after `set_boxed_logger` succeeds, `init` stores the `Arc` in the slot, then stores `THRESHOLD`, then calls `log::set_max_level`. Records arriving before that last step are filtered by the facade's initial `Off` level (`log-0.4.34/src/lib.rs:467`).
13. **API freeze:** `tests/api_freeze.rs` pins every a-1 public signature by coercing each item to its exact type (including `code`/`remediation` on `InitError`, `FlushError` and `ShutdownError`), pins the derives of `DroppedEvents` and `DropCause` with trait-bound assertions, and matches every error and `DropCause` variant exhaustively (code sample below). After a-1 merges the file is frozen: a-2 and a-3 may add public items but must not change it.
14. **Docs:** `README.md` (quick start, lockstep and `__private` policy, no-panic policy, the `catch_unwind`/`panic = "abort"` residual, and the rule that `log::logger().flush()` does nothing: call `LogGuard::flush(timeout)`) and `docs/mapping.md` (the mapping table, the sanitizer rules, the emit semantics of Deliverable 9 and the error inventory, verbatim from this sprint). The rustdoc of `__private::emit` and of the `log::Log` impl states the reentrancy and flush rules. Every public item has rustdoc (the `-D missing-docs` gate).
15. **CI:** a new `crates` job in btit `.github/workflows/ci.yml`, and `- 'feature/**'` added to `on.pull_request.branches`, exactly as in the workflow code sample. Without that entry the stacked PRs whose base is a `feature/*` branch (a-2, a-3, a-5, a-6) trigger no CI at all: the existing list is `main`, `develop`, `integrate/*`, `sprint/*`, `fix/*`, and the workflow has no `push` trigger. Local-only commands (xwin, the macOS LLVM path) are not CI steps.
16. **`log::Log` implementation (`bridge.rs`):** `enabled` compares the record level against `THRESHOLD`. `log` returns at once when the record is below the threshold; otherwise it calls `record_to_parts` and `emit`, and never blocks or panics. `flush(&self)` is a deliberate no-op. `log::Log::flush` has no timeout and no error channel, and delegating to sc-observability's unbounded flush (`maintenance.rs:137-164`) would let any `log::logger().flush()` caller hang. The bounded flush is `LogGuard::flush(timeout)`.

## Required Work

- **Record mapping** (implemented in `mapping.rs`):

  | `log::Record` | `LogEvent` |
  |---|---|
  | `level()` Error/Warn/Info/Debug/Trace | `Level::Error/Warn/Info/Debug/Trace` |
  | `target()` | `target = target_label(target())`: `::` → `.`; each char outside `[A-Za-z0-9._-]` → `_`; empty → `log` |
  | leading `[tag]` of `args()` when `options.parse_bracket_action` and `tag` matches `[A-Za-z0-9._-]+` (`sanitize_label(tag)` borrows and is non-empty) | `action = action_label(tag)`; the tag and one following space are removed from `message` |
  | otherwise | `action = options.default_action` |
  | formatted `args()` | `message = Some(..)` |
  | `key_values()` (the `log` `kv` feature) | `fields[key] = serde_json::Value` (numbers, bools and strings typed; others `to_string()`) |
  | `module_path()`, `file()`, `line()` | `fields["code.module"]`, `fields["code.file"]`, `fields["code.line"]` (omitted when `None`) |
  | none | `service` = `LoggerConfig.service_name`; `identity` = the value `init` resolved (Deliverable 6); `version` = envelope version; `timestamp` = `Timestamp::now_utc()` |
  | none | `trace` = `None` (a-3 replaces this row: ambient span context at emit time) |
  | none | `request_id`, `correlation_id`, `outcome`, `diagnostic`, `state_transition` = `None` |
  | `log::Log::flush()` | no-op; nothing is flushed (use `LogGuard::flush(timeout)`) |

- **Global state** (`handle.rs`): `static SLOT: RwLock<Option<Arc<Installed>>>`, `static INSTALLED: AtomicBool` (claimed by `compare_exchange` in `init`), `static THRESHOLD: AtomicU8` (encodes the `LevelFilter`; `Off` before `init` and after shutdown), one `AtomicU64` per `DropCause` variant selected by `match` inside `record_drop` (no array indexing), and the `const` thread-local `IN_EMIT: Cell<bool>` behind `EmitScope`. `sc_observability::Logger<Running>` is `Send + Sync` (compile-checked while planning), so `Arc<Installed>` needs no extra lock around the logger. Blocking work for `flush`/`shutdown` goes through one crate-private helper returning the internal enum `BoundedError`, shown in the code samples.
- **Error inventory** (authoritative for a-1, organized by enum and variant; codes live in `error_codes.rs` with an `ALL` slice, the same shape as `../sc-observability/crates/sc-observability/src/error_codes.rs`):

  | Enum::Variant (typed fields) | Returned by | `code()` | Cause | `remediation()` |
  |---|---|---|---|---|
  | `InitError::AlreadyInitialized` | `init` | `SC_OBSERVABILITY_LOG_ALREADY_INITIALIZED` | `INSTALLED.compare_exchange` failed: an own `init` succeeded earlier (guard alive or shut down), is running concurrently, or earlier returned `ForeignLoggerInstalled` | not recoverable: the `log` facade logger cannot be replaced; keep the first `LogGuard` |
  | `InitError::ForeignLoggerInstalled { source: log::SetLoggerError }` | `init` | `SC_OBSERVABILITY_LOG_FOREIGN_LOGGER_INSTALLED` | another `log::Log` (for example `tauri-plugin-log`) is installed; the built `Logger` is shut down and `INSTALLED` stays set | remove the other logger, or call `init` before it is installed |
  | `InitError::IdentityResolution { source: sc_observability_types::IdentityError }` | `init` | `SC_OBSERVABILITY_LOG_IDENTITY_RESOLUTION_FAILED` | `ProcessIdentityPolicy::Resolver` failed; `INSTALLED` is reset | fix the resolver, or use `Auto`/`Fixed`, then call `init` again |
  | `InitError::Logger { source: sc_observability_types::InitError }` | `init` | the source's code (for example `SC_OBSERVABILITY_LOGGER_INIT_FAILED`) | `Logger::new` failed (log root not creatable, invalid config); `INSTALLED` is reset | the source's remediation, then call `init` again |
  | `FlushError::TimedOut { timeout: Duration }` | `LogGuard::flush` | `SC_OBSERVABILITY_LOG_FLUSH_TIMED_OUT` | the writer did not acknowledge within `timeout`; the helper is detached and keeps its `Arc<Installed>` clone until sc-observability's flush returns | retry later or raise `timeout`; a `shutdown` before that flush returns reports `ShutdownError::TimedOut` |
  | `FlushError::Logger { source: sc_observability_types::FlushError }` | `LogGuard::flush` | the source's code (`SC_OBSERVABILITY_LOGGER_FLUSH_FAILED`, `SC_OBSERVABILITY_LOGGER_WRITER_DEGRADED`) | a sink flush failed or the writer disconnected | the source's remediation |
  | `FlushError::HelperSpawn { source: std::io::Error }` | `LogGuard::flush` | `SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED` | `std::thread::Builder::spawn` failed | retry; check process thread limits |
  | `FlushError::HelperLost` | `LogGuard::flush` | `SC_OBSERVABILITY_LOG_HELPER_LOST` | the helper thread ended without a result (a panic inside sc-observability) | shut the guard down; the logger may be degraded |
  | `ShutdownError::TimedOut { timeout: Duration }` | `LogGuard::shutdown` | `SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT` | sole ownership, flush and writer join did not finish within `timeout`; the helper is detached. Also forced when `take_sole` reaches the deadline (`ShutdownStep::StillShared`) because another `Arc<Installed>` clone is still alive, most often the helper of an earlier timed-out `flush` | none needed at process exit; still-queued events may be lost |
  | `ShutdownError::FinalFlush { source: sc_observability_types::FlushError }` | `LogGuard::shutdown` | the source's code | the final flush failed; the logger was still shut down | the source's remediation |
  | `ShutdownError::HelperSpawn { source: std::io::Error }` | `LogGuard::shutdown` | `SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED` | helper thread could not start; the slot is already empty | none needed at process exit |
  | `ShutdownError::HelperLost` | `LogGuard::shutdown` | `SC_OBSERVABILITY_LOG_HELPER_LOST` | the helper ended without a result | none needed at process exit |
  | `DropCause::{QueueFull, WriterDegraded, ShutdownTimedOut}` | emit path (counted through `record_drop`, not returned) | — | the matching `TryLogError` variant | raise `queue_capacity`; inspect sc-observability health |
  | `DropCause::InvalidEvent` | emit path and a-2/a-3 emission (counted through `record_drop`) | — | two causes: (1) `TryLogError::InvalidEvent` from `try_log`, the whole event dropped; (2) a runtime label (target, name or field key) failing `target_label`/`action_label`/`field_key_label` validation or the reserved-prefix check in a-2/a-3 emission, a failing field key omitting only that field | (1) inspect the event contents; (2) fix the label or the key |
  | `DropCause::NotInstalled` | emit path (counted) | — | record arrived before `init` or after shutdown | keep the guard alive for the process lifetime |
  | `DropCause::LoggerPanicked` | emit path (counted) | — | a panic inside sc-observability `try_log`, caught by `catch_unwind(AssertUnwindSafe(..))` | report upstream; shut the guard down |
  | `DropCause::ReentrantEmit` | emit path (counted) | — | `emit` was called on a thread already inside `emit` (`IN_EMIT` set): a panic hook, sink or redactor that logs through `log` or the macros | do not log from panic hooks, sinks or redactors that run inside the logger |
  | `BoundedError::{TimedOut, Spawn { source: std::io::Error }, WorkerLost}` (crate-private) | `run_bounded` | — | mapped 1:1 into the `FlushError`/`ShutdownError` variants above | — |
  | `ShutdownStep::{StillShared, FinalFlush { source: sc_observability_types::FlushError }}` (crate-private) | shutdown helper | — | `StillShared` → `ShutdownError::TimedOut`; `FinalFlush` → `ShutdownError::FinalFlush` | — |
  | `LabelError::{Empty { kind: LabelKind }, ReservedPrefix { kind: LabelKind }, Rejected { kind: LabelKind, source: sc_observability_types::ValueValidationError }}` (`#[doc(hidden)]` via `__private`) | `target_label`, `action_label`, `field_key_label` | — | an empty label or key, a reserved-prefix field key, or a validation error after sanitizing | counted as `DropCause::InvalidEvent` by the caller; never returned to users |

- **Test-isolation contract for the process-global logger** (applies to every test that calls `init()`; inherited by a-2 and a-3):
  - `log::set_boxed_logger` can be called once per process and the `log` crate has no uninstall API (`log-0.4.34/src/lib.rs`). `cargo test` runs the `#[test]` fns of one test binary on parallel threads.
  - Each integration test file that calls `init()` is its own test binary (own process) and contains **exactly one** `#[test]` fn. All sub-cases for that configuration run sequentially inside that fn through plain helper functions sharing the one guard.
  - A test needing a different `LoggerConfig`/`BridgeOptions` (for example a small `queue_capacity`) gets its **own** test file.
  - Pure mapping and unit tests never call `init()`.
  - No `--test-threads=1` and no `serial_test` dependency: isolation comes from the one-`init`-per-binary rule, so `cargo test --workspace` stays the only test command. The test-isolation command in Required Validation enforces the rule mechanically (every `crates/*/tests/*.rs` file that contains `init(` has exactly one test attribute, counting `#[test]`, `#[tokio::test]` and any other `#[path::test]`).
- **Tests:**
  - Unit tests cover every mapping row, including invalid targets and tags, an empty target, a tag without a space, `kv` values of each JSON type, and each `ProcessIdentityPolicy` variant including a failing `Resolver`.
  - Unit tests assert `code()` and a non-empty `remediation()` for every error variant in the inventory.
  - A `handle.rs` unit test calls `run_bounded(Duration::from_millis(100), || std::thread::sleep(Duration::from_secs(2)))` and asserts `Err(BoundedError::TimedOut)` in under 600 ms; a second calls it with a closure that panics and asserts `Err(BoundedError::WorkerLost)`.
  - `handle.rs` unit tests for `take_sole`: with a clone released by another thread after 30 ms it returns `Some`; with a clone held for the whole call and a 100 ms deadline it returns `None` in under 400 ms and leaves the held clone as the only strong reference.
  - One `handle.rs` unit test, `emit_core_counts_panics_and_reentry`, drives `submit_guarded` (no `init`). It is the only unit test that calls `submit_guarded` or `record_drop`, so its counter deltas cannot race. In sequence it checks four things. First, a nested `submit_guarded` inside the closure is counted once as `ReentrantEmit`. Second, with a panic hook that calls `submit_guarded` installed, a closure that panics is counted once as `LoggerPanicked` and the hook's call once more as `ReentrantEmit`. Third, the hook is removed with `take_hook`. Fourth, a fresh call is not counted (the `EmitScope` was released during unwinding).
  - `mapping.rs` unit tests for the sanitizer: `app_lib::sync` → `app_lib.sync`; `a b` → `a_b`; `target_label("")` → `log`; `action_label("")` → `LabelError::Empty`; `field_key_label("sc_observability_log.x")` and `field_key_label("sc_observability_log::x")` → `LabelError::ReservedPrefix`; `field_key_label("a.b")` borrows.
  - `tests/bridge_jsonl.rs` initializes into a `tempfile` log root with `LoggerConfig.level = LevelFilter::Debug` and logs one record at each level, plus one tagged record and one `kv` record. It flushes and reads the file at `guard.active_log_path()`, and asserts `log::logger().flush()` returns immediately. It asserts `target`, `action`, `message`, `fields` and `identity.pid == Some(std::process::id())`, and that the `Trace` record is absent. In the same single `#[test]` fn it then asserts a second `init` matches `Err(InitError::AlreadyInitialized)`, calls `guard.shutdown(Duration::from_secs(5))`, logs again, asserts that record is absent and nothing panicked, and asserts `init` still matches `Err(InitError::AlreadyInitialized)`.
  - `tests/bridge_queue_full.rs` (its own binary) initializes with `queue_capacity = 1`, floods records from several threads without blocking, and asserts that `dropped_events().get(DropCause::QueueFull)` increased and that `total()` equals the sum over `DropCause::ALL`.

## Explicit Code Samples

```toml
# crates/Cargo.toml
[workspace]
members = ["sc-observability-log", "sc-observability-log-macros"]
resolver = "3"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.94.1"
license = "MIT"

[workspace.dependencies]
sc-observability = "1.2.0"
sc-observability-types = "1.2.0"
log = { version = "0.4", features = ["std", "kv"] }   # `std` enables log::set_boxed_logger (cfg(feature = "alloc") in log-0.4.34)
serde = "1"
serde_json = "1"
thiserror = "2"
syn = { version = "2", features = ["full"] }
quote = "1"
proc-macro2 = "1"
tempfile = "3"
sc-observability-log-macros = { version = "=0.1.0", path = "sc-observability-log-macros" }

[workspace.lints.rust]
missing_debug_implementations = "warn"
unsafe_op_in_unsafe_fn = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
unreachable = "deny"
todo = "deny"
unimplemented = "deny"
indexing_slicing = "deny"
```

```toml
# crates/sc-observability-log/clippy.toml and crates/sc-observability-log-macros/clippy.toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-indexing-slicing-in-tests = true
```

```rust
// crates/sc-observability-log/src/lib.rs
pub mod error_codes;
mod error;
pub use error::{DropCause, FlushError, InitError, ShutdownError};
pub use sc_observability::LoggerConfig;
// Re-exported so consumers (btit in a-4) need no direct sc-observability-types dependency.
pub use sc_observability_types::{
    ActionName, ErrorCode, LevelFilter, ProcessIdentityPolicy, Remediation, ServiceName, TargetCategory,
};
// `Level` is intentionally NOT re-exported: a-2 defines a tracing-style
// `sc_observability_log::Level` (associated consts TRACE..ERROR).

/// Bridge behavior. The level threshold is `LoggerConfig.level` only.
#[derive(Debug, Clone)]
pub struct BridgeOptions {
    /// Action used when a record carries no leading `[tag]`.
    pub default_action: ActionName,
    /// Strip a leading `[tag] ` from the message into `LogEvent.action`.
    pub parse_bracket_action: bool,
}

/// Snapshot of dropped-event counters, keyed by `DropCause`. The derives are pinned by `tests/api_freeze.rs`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DroppedEvents { /* private: one u64 per DropCause variant */ }
impl DroppedEvents {
    #[must_use]
    pub fn get(&self, cause: DropCause) -> u64;   // exhaustive match, no indexing
    #[must_use]
    pub fn total(&self) -> u64;                   // saturating sum over DropCause::ALL
}

/// Timeout used by `Drop for LogGuard` when `shutdown` was not called.
pub const DEFAULT_DROP_SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[must_use = "dropping the guard shuts the logger down"]
#[derive(Debug)]
pub struct LogGuard { /* private: active_log_path: Option<PathBuf>, shut_down: bool */ }

impl LogGuard {
    /// Flushes on a helper thread, bounded by `timeout`.
    pub fn flush(&self, timeout: std::time::Duration) -> Result<(), FlushError>;
    /// Threshold → Off, empty the slot, flush, `Logger::shutdown`; bounded by `timeout`.
    pub fn shutdown(self, timeout: std::time::Duration) -> Result<(), ShutdownError>;
    #[must_use]
    pub fn dropped_events(&self) -> DroppedEvents;
    /// `Logger::health().active_log_path` captured at init; `None` when
    /// `LoggerConfig.enable_file_sink` is false.
    #[must_use]
    pub fn active_log_path(&self) -> Option<&std::path::Path>;
}
impl Drop for LogGuard { /* unless already shut down: shutdown sequence with DEFAULT_DROP_SHUTDOWN_TIMEOUT, result ignored */ }

/// Installs the bridge.
pub fn init(config: LoggerConfig, options: BridgeOptions) -> Result<LogGuard, InitError>;

// crates/sc-observability-log/src/error.rs
use std::time::Duration;
use sc_observability_types::{ErrorCode, Remediation};

/// Error returned by [`init`](crate::init).
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("sc-observability-log is already initialized in this process")]
    AlreadyInitialized,
    #[error("another log::Log implementation is already installed")]
    ForeignLoggerInstalled { #[source] source: log::SetLoggerError },
    #[error("process identity resolution failed")]
    IdentityResolution { #[source] source: sc_observability_types::IdentityError },
    #[error("sc-observability logger construction failed")]
    Logger { #[source] source: sc_observability_types::InitError },
}

/// Error returned by [`LogGuard::flush`](crate::LogGuard::flush).
#[derive(Debug, thiserror::Error)]
pub enum FlushError {
    #[error("flush did not complete within {timeout:?}")]
    TimedOut { timeout: Duration },
    #[error("sc-observability flush failed")]
    Logger { #[source] source: sc_observability_types::FlushError },
    #[error("could not start the flush helper thread")]
    HelperSpawn { #[source] source: std::io::Error },
    #[error("the flush helper thread ended without a result")]
    HelperLost,
}

/// Error returned by [`LogGuard::shutdown`](crate::LogGuard::shutdown).
#[derive(Debug, thiserror::Error)]
pub enum ShutdownError {
    #[error("shutdown did not complete within {timeout:?}")]
    TimedOut { timeout: Duration },
    #[error("final flush failed; the logger was still shut down")]
    FinalFlush { #[source] source: sc_observability_types::FlushError },
    #[error("could not start the shutdown helper thread")]
    HelperSpawn { #[source] source: std::io::Error },
    #[error("the shutdown helper thread ended without a result")]
    HelperLost,
}

// Same pair of accessors on InitError, FlushError and ShutdownError:
impl InitError {
    /// Stable code per variant; wrapped sc-observability errors return their own code.
    #[must_use]
    pub fn code(&self) -> ErrorCode;
    /// Mandatory remediation per variant; wrapped errors return their own remediation.
    #[must_use]
    pub fn remediation(&self) -> Remediation;
}

/// Why an event was dropped on the non-blocking emit path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropCause {
    QueueFull,          // TryLogError::QueueFull
    InvalidEvent,       // TryLogError::InvalidEvent, or a runtime label/field key failing the sanitizer (a-2/a-3)
    WriterDegraded,     // TryLogError::WriterDegraded
    ShutdownTimedOut,   // TryLogError::ShutdownTimedOut
    NotInstalled,       // no logger installed (before init / after shutdown)
    LoggerPanicked,     // panic inside sc-observability try_log, caught by catch_unwind(AssertUnwindSafe(..))
    ReentrantEmit,      // emit called on a thread already inside emit (panic hook, sink, redactor)
}
impl DropCause {
    pub const ALL: [DropCause; 7] = [
        Self::QueueFull, Self::InvalidEvent, Self::WriterDegraded, Self::ShutdownTimedOut,
        Self::NotInstalled, Self::LoggerPanicked, Self::ReentrantEmit,
    ];
}

// crates/sc-observability-log/src/handle.rs (crate-private).
// Bodies below compiled and clippy-clean (pedantic + the no-panic deny set) on 1.94.1 and 1.98.1.
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, PoisonError, RwLock};
use std::time::{Duration, Instant};
use sc_observability::TryLogError;

pub(crate) struct Installed {
    pub(crate) logger: sc_observability::Logger,
    pub(crate) service: sc_observability_types::ServiceName,
    pub(crate) identity: sc_observability_types::ProcessIdentity,
    pub(crate) options: crate::BridgeOptions,
}

pub(crate) static SLOT: RwLock<Option<Arc<Installed>>> = RwLock::new(None);
pub(crate) static INSTALLED: AtomicBool = AtomicBool::new(false);
pub(crate) static THRESHOLD: AtomicU8 = AtomicU8::new(0); // 0 = Off
static QUEUE_FULL: AtomicU64 = AtomicU64::new(0);
// … INVALID_EVENT, WRITER_DEGRADED, SHUTDOWN_TIMED_OUT, NOT_INSTALLED, LOGGER_PANICKED, REENTRANT_EMIT

/// Counts one dropped event. Re-exported as `__private::record_drop`.
pub(crate) fn record_drop(cause: DropCause) {
    let counter = match cause {
        DropCause::QueueFull => &QUEUE_FULL,
        DropCause::InvalidEvent => &INVALID_EVENT,
        DropCause::WriterDegraded => &WRITER_DEGRADED,
        DropCause::ShutdownTimedOut => &SHUTDOWN_TIMED_OUT,
        DropCause::NotInstalled => &NOT_INSTALLED,
        DropCause::LoggerPanicked => &LOGGER_PANICKED,
        DropCause::ReentrantEmit => &REENTRANT_EMIT,
    };
    counter.fetch_add(1, Ordering::Relaxed);
}

thread_local! {
    static IN_EMIT: Cell<bool> = const { Cell::new(false) };
}

/// Per-thread reentrancy guard. Only `try_with` is used: `with` can panic.
pub(crate) struct EmitScope(());
impl EmitScope {
    pub(crate) fn enter() -> Option<Self> {
        match IN_EMIT.try_with(|flag| flag.replace(true)) {
            Ok(false) => Some(Self(())),
            Ok(true) | Err(_) => None, // reentrant, or TLS unavailable: treated as reentrant
        }
    }
}
impl Drop for EmitScope {
    fn drop(&mut self) {
        let _ = IN_EMIT.try_with(|flag| flag.set(false)); // also runs while a panic unwinds
    }
}

/// Emit core: reentrancy guard plus panic containment. Every dropped event is counted exactly once.
pub(crate) fn submit_guarded(submit: impl FnOnce() -> Result<(), DropCause>) {
    let Some(_scope) = EmitScope::enter() else {
        record_drop(DropCause::ReentrantEmit);
        return;
    };
    // AssertUnwindSafe: the closure reaches Logger, which holds dyn LogSink / dyn Redactor /
    // dyn ProcessIdentityResolver; none is RefUnwindSafe (E0277 without the wrapper). After a
    // caught panic nothing reads logger state except try_log itself, which reports its
    // poisoned mutexes by panicking again.
    match catch_unwind(AssertUnwindSafe(submit)) {
        Ok(Ok(())) => {}
        Ok(Err(cause)) => record_drop(cause),
        Err(_payload) => record_drop(DropCause::LoggerPanicked),
    }
}

// crates/sc-observability-log/src/lib.rs, inside `pub mod __private`
/// Never blocks on I/O or queue capacity, never panics, and drops reentrant calls.
pub fn emit(parts: EventParts) {
    crate::handle::submit_guarded(|| {
        let Some(installed) = crate::handle::SLOT.read().unwrap_or_else(PoisonError::into_inner).clone() else {
            return Err(DropCause::NotInstalled);
        };
        let event = mapping::assemble_event(parts, &installed.service, &installed.identity, &installed.options.default_action);
        installed.logger.try_log(event).map_err(|error| match error {
            TryLogError::QueueFull(_) => DropCause::QueueFull,
            TryLogError::InvalidEvent(_) => DropCause::InvalidEvent,
            TryLogError::WriterDegraded(_) => DropCause::WriterDegraded,
            TryLogError::ShutdownTimedOut(_) => DropCause::ShutdownTimedOut,
        })
    });
}

// crates/sc-observability-log/src/handle.rs (continued)
#[derive(Debug)]
pub(crate) enum BoundedError {
    TimedOut,
    Spawn { source: std::io::Error },
    WorkerLost,   // channel disconnected: the work closure panicked
}

pub(crate) fn run_bounded<T: Send + 'static>(
    timeout: Duration,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, BoundedError> {
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("sc-observability-log-helper".to_owned())
        .spawn(move || {
            let _ = tx.send(work());
        })
        .map_err(|source| BoundedError::Spawn { source })?;
    match rx.recv_timeout(timeout) {
        Ok(value) => Ok(value),
        Err(RecvTimeoutError::Timeout) => Err(BoundedError::TimedOut),
        Err(RecvTimeoutError::Disconnected) => Err(BoundedError::WorkerLost),
    }
}

const UNWRAP_BACKOFF_START: Duration = Duration::from_millis(1);
const UNWRAP_BACKOFF_MAX: Duration = Duration::from_millis(50);

/// Retries `Arc::try_unwrap` with a sleep backoff until `deadline` (`None`: no deadline).
pub(crate) fn take_sole<T>(mut shared: Arc<T>, deadline: Option<Instant>) -> Option<T> {
    let mut backoff = UNWRAP_BACKOFF_START;
    loop {
        match Arc::try_unwrap(shared) {
            Ok(value) => return Some(value),
            Err(still_shared) => {
                let now = Instant::now();
                let remaining = match deadline {
                    Some(d) if now >= d => return None, // drops this clone
                    Some(d) => d.saturating_duration_since(now),
                    None => UNWRAP_BACKOFF_MAX,
                };
                shared = still_shared;
                std::thread::sleep(backoff.min(remaining));
                backoff = backoff.saturating_mul(2).min(UNWRAP_BACKOFF_MAX);
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum ShutdownStep {
    StillShared,
    FinalFlush { source: sc_observability_types::FlushError },
}

/// Shutdown helper. `Logger::shutdown(self)` consumes the logger (runtime.rs:224), so the
/// helper first gains sole ownership. A helper detached by a timed-out `LogGuard::flush`
/// holds an `Arc` clone and forces `ShutdownError::TimedOut`.
pub(crate) fn shutdown_installed(installed: Arc<Installed>, timeout: Duration) -> Result<(), ShutdownError> {
    let deadline = Instant::now().checked_add(timeout);
    let outcome = run_bounded(timeout, move || {
        let Some(sole) = take_sole(installed, deadline) else {
            return Err(ShutdownStep::StillShared);
        };
        let flushed = sole.logger.flush();
        let _stopped = sole.logger.shutdown();
        flushed.map_err(|source| ShutdownStep::FinalFlush { source })
    });
    match outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(ShutdownStep::StillShared)) | Err(BoundedError::TimedOut) => Err(ShutdownError::TimedOut { timeout }),
        Ok(Err(ShutdownStep::FinalFlush { source })) => Err(ShutdownError::FinalFlush { source }),
        Err(BoundedError::Spawn { source }) => Err(ShutdownError::HelperSpawn { source }),
        Err(BoundedError::WorkerLost) => Err(ShutdownError::HelperLost),
    }
}

/// `LogGuard::shutdown` and `Drop for LogGuard` both call this once.
pub(crate) fn shutdown_sequence(timeout: Duration) -> Result<(), ShutdownError> {
    THRESHOLD.store(0, Ordering::SeqCst);
    log::set_max_level(log::LevelFilter::Off);
    let taken = SLOT.write().unwrap_or_else(PoisonError::into_inner).take();
    match taken {
        Some(installed) => shutdown_installed(installed, timeout),
        None => Ok(()),
    }
}

/// `LogGuard::flush`: the helper owns an `Arc` clone until sc-observability's flush returns.
pub(crate) fn flush_installed(timeout: Duration) -> Result<(), FlushError> {
    let Some(installed) = SLOT.read().unwrap_or_else(PoisonError::into_inner).clone() else {
        return Ok(());
    };
    match run_bounded(timeout, move || installed.logger.flush()) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(source)) => Err(FlushError::Logger { source }),
        Err(BoundedError::TimedOut) => Err(FlushError::TimedOut { timeout }),
        Err(BoundedError::Spawn { source }) => Err(FlushError::HelperSpawn { source }),
        Err(BoundedError::WorkerLost) => Err(FlushError::HelperLost),
    }
}

// crates/sc-observability-log/src/lib.rs — `init` install-once skeleton
pub fn init(config: LoggerConfig, options: BridgeOptions) -> Result<LogGuard, InitError> {
    if INSTALLED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return Err(InitError::AlreadyInitialized); // includes a concurrent own init
    }
    let identity = match resolve_identity(&config.process_identity) {
        Ok(identity) => identity,
        Err(source) => {
            INSTALLED.store(false, Ordering::SeqCst); // recoverable: allow a retry
            return Err(InitError::IdentityResolution { source });
        }
    };
    let (level, service) = (config.level, config.service_name.clone());
    let logger = match sc_observability::Logger::new(config) {
        Ok(logger) => logger,
        Err(source) => {
            INSTALLED.store(false, Ordering::SeqCst); // recoverable: allow a retry
            return Err(InitError::Logger { source });
        }
    };
    let active_log_path = /* logger.health().active_log_path when the file sink is enabled */;
    let installed = Arc::new(Installed { logger, service, identity, options });
    if let Err(source) = log::set_boxed_logger(Box::new(Bridge)) {
        // INSTALLED stays set: the facade slot belongs to the other logger for the process lifetime.
        let _ = shutdown_installed(installed, DEFAULT_DROP_SHUTDOWN_TIMEOUT);
        return Err(InitError::ForeignLoggerInstalled { source });
    }
    *SLOT.write().unwrap_or_else(PoisonError::into_inner) = Some(installed);
    THRESHOLD.store(encode_threshold(level), Ordering::SeqCst);
    log::set_max_level(to_log_level_filter(level));
    Ok(LogGuard { active_log_path, shut_down: false })
}

// crates/sc-observability-log/src/bridge.rs
#[derive(Debug)]
struct Bridge;
impl log::Log for Bridge {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool; // metadata.level() vs THRESHOLD
    fn log(&self, record: &log::Record<'_>);                 // enabled check, record_to_parts, __private::emit
    /// No-op by design: `log::Log::flush` has no timeout or error channel, and sc-observability's
    /// flush is unbounded (maintenance.rs:137-164). Use `LogGuard::flush(timeout)`.
    fn flush(&self) {}
}

// crates/sc-observability-log/src/error_codes.rs
pub const SC_OBSERVABILITY_LOG_ALREADY_INITIALIZED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_ALREADY_INITIALIZED");
pub const SC_OBSERVABILITY_LOG_FOREIGN_LOGGER_INSTALLED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_FOREIGN_LOGGER_INSTALLED");
pub const SC_OBSERVABILITY_LOG_IDENTITY_RESOLUTION_FAILED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_IDENTITY_RESOLUTION_FAILED");
pub const SC_OBSERVABILITY_LOG_FLUSH_TIMED_OUT: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_FLUSH_TIMED_OUT");
pub const SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT");
pub const SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED");
pub const SC_OBSERVABILITY_LOG_HELPER_LOST: ErrorCode =
    ErrorCode::new_static("SC_OBSERVABILITY_LOG_HELPER_LOST");
pub const ALL: &[ErrorCode] = &[/* the seven codes above */];

// crates/sc-observability-log/src/mapping.rs (pure). The sanitizer is the single phase-a sanitizer;
// its items are `pub` inside the private `mapping` module (so `pub use` from `__private` compiles)
// and are reachable from outside the crate only through `__private`.
pub const RESERVED_FIELD_PREFIX: &str = "sc_observability_log.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind { Target, Action, FieldKey }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelError {
    Empty { kind: LabelKind },
    ReservedPrefix { kind: LabelKind },
    Rejected { kind: LabelKind, source: sc_observability_types::ValueValidationError },
}

/// `::` → `.`; each char outside `[A-Za-z0-9._-]` → `_`. Borrows when already valid; never fails.
#[must_use]
pub fn sanitize_label(raw: &str) -> Cow<'_, str> {
    let valid = |ch: char| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-');
    if !raw.contains("::") && raw.chars().all(valid) {
        return Cow::Borrowed(raw);
    }
    Cow::Owned(raw.replace("::", ".").chars().map(|ch| if valid(ch) { ch } else { '_' }).collect())
}

/// Empty after sanitizing → `log`.
pub fn target_label(raw: &str) -> Result<TargetCategory, LabelError> {
    let clean = sanitize_label(raw);
    let clean = if clean.is_empty() { Cow::Borrowed("log") } else { clean };
    TargetCategory::new(clean.into_owned()).map_err(|source| LabelError::Rejected { kind: LabelKind::Target, source })
}

/// Empty after sanitizing → `LabelError::Empty`.
pub fn action_label(raw: &str) -> Result<ActionName, LabelError> {
    let clean = sanitize_label(raw);
    if clean.is_empty() {
        return Err(LabelError::Empty { kind: LabelKind::Action });
    }
    ActionName::new(clean.into_owned()).map_err(|source| LabelError::Rejected { kind: LabelKind::Action, source })
}

/// Empty → `LabelError::Empty`; starts with `RESERVED_FIELD_PREFIX` after sanitizing → `LabelError::ReservedPrefix`.
pub fn field_key_label(raw: &str) -> Result<Cow<'_, str>, LabelError> {
    let clean = sanitize_label(raw);
    if clean.is_empty() {
        return Err(LabelError::Empty { kind: LabelKind::FieldKey });
    }
    if clean.starts_with(RESERVED_FIELD_PREFIX) {
        return Err(LabelError::ReservedPrefix { kind: LabelKind::FieldKey });
    }
    Ok(clean)
}

pub(crate) fn record_to_parts(record: &log::Record<'_>, options: &BridgeOptions) -> __private::EventParts;
pub(crate) fn assemble_event(
    parts: __private::EventParts,
    service: &ServiceName,
    identity: &sc_observability_types::ProcessIdentity,
    default_action: &ActionName,
) -> sc_observability_types::LogEvent;

#[doc(hidden)]
pub mod __private {
    pub use serde_json::{Map, Value};
    /// Everything a call site controls; `emit` fills version, timestamp, service, identity and trace.
    #[derive(Debug)]
    pub struct EventParts {
        pub level: sc_observability_types::Level,
        pub target: sc_observability_types::TargetCategory,
        pub action: Option<sc_observability_types::ActionName>, // None → BridgeOptions.default_action
        pub message: Option<String>,
        pub outcome: Option<sc_observability_types::OutcomeLabel>,
        pub fields: Map<String, Value>,
    }
    /// Lock-free read of THRESHOLD.
    pub fn enabled(level: sc_observability_types::Level) -> bool;
    /// Never blocks on I/O or queue capacity, never panics, drops reentrant calls (body above).
    pub fn emit(parts: EventParts);
    /// Counts one dropped event; a-2/a-3 call it for `DropCause::InvalidEvent` label failures.
    /// A wrapper, not `pub use`: re-exporting the `pub(crate)` fn is E0364 (compiled on 1.94.1).
    pub fn record_drop(cause: crate::DropCause) {
        crate::handle::record_drop(cause);
    }
    /// The single label sanitizer (mapping.rs).
    pub use crate::mapping::{
        action_label, field_key_label, sanitize_label, target_label, LabelError, LabelKind, RESERVED_FIELD_PREFIX,
    };
}
```

```rust
// crates/sc-observability-log/tests/api_freeze.rs — frozen after a-1 merges
use sc_observability_log::{
    BridgeOptions, DropCause, DroppedEvents, FlushError, InitError, LogGuard, LoggerConfig, ShutdownError,
};
use std::time::Duration;

#[test]
fn a1_public_api_is_frozen() {
    let _: fn(LoggerConfig, BridgeOptions) -> Result<LogGuard, InitError> = sc_observability_log::init;
    let _: fn(&LogGuard, Duration) -> Result<(), FlushError> = LogGuard::flush;
    let _: fn(LogGuard, Duration) -> Result<(), ShutdownError> = LogGuard::shutdown;
    let _: fn(&LogGuard) -> DroppedEvents = LogGuard::dropped_events;
    let _: for<'a> fn(&'a LogGuard) -> Option<&'a std::path::Path> = LogGuard::active_log_path;
    let _: fn(&DroppedEvents, DropCause) -> u64 = DroppedEvents::get;
    let _: fn(&DroppedEvents) -> u64 = DroppedEvents::total;
    let _: fn(&InitError) -> sc_observability_log::ErrorCode = InitError::code;
    let _: fn(&InitError) -> sc_observability_log::Remediation = InitError::remediation;
    let _: fn(&FlushError) -> sc_observability_log::ErrorCode = FlushError::code;
    let _: fn(&FlushError) -> sc_observability_log::Remediation = FlushError::remediation;
    let _: fn(&ShutdownError) -> sc_observability_log::ErrorCode = ShutdownError::code;
    let _: fn(&ShutdownError) -> sc_observability_log::Remediation = ShutdownError::remediation;
    // Derives: removing one breaks compilation.
    fn dropped_events_derives<T: std::fmt::Debug + Clone + Copy + Default + PartialEq + Eq>() {}
    fn drop_cause_derives<T: std::fmt::Debug + Clone + Copy + PartialEq + Eq + std::hash::Hash>() {}
    dropped_events_derives::<DroppedEvents>();
    drop_cause_derives::<DropCause>();
    let _ = |a: sc_observability_log::ActionName| BridgeOptions { default_action: a, parse_bracket_action: true };
    let _: Duration = sc_observability_log::DEFAULT_DROP_SHUTDOWN_TIMEOUT;
    let _: &[sc_observability_log::ErrorCode] = sc_observability_log::error_codes::ALL;
    // Exhaustive matches: adding, removing or reshaping a variant breaks this test.
    let _ = |e: InitError| match e {
        InitError::AlreadyInitialized => (),
        InitError::ForeignLoggerInstalled { source: _ } => (),
        InitError::IdentityResolution { source: _ } => (),
        InitError::Logger { source: _ } => (),
    };
    let _ = |e: FlushError| match e {
        FlushError::TimedOut { timeout: _ } | FlushError::Logger { source: _ } => (),
        FlushError::HelperSpawn { source: _ } | FlushError::HelperLost => (),
    };
    let _ = |e: ShutdownError| match e {
        ShutdownError::TimedOut { timeout: _ } | ShutdownError::FinalFlush { source: _ } => (),
        ShutdownError::HelperSpawn { source: _ } | ShutdownError::HelperLost => (),
    };
    let _ = |c: DropCause| match c {
        DropCause::QueueFull | DropCause::InvalidEvent | DropCause::WriterDegraded => (),
        DropCause::ShutdownTimedOut | DropCause::NotInstalled => (),
        DropCause::LoggerPanicked | DropCause::ReentrantEmit => (),
    };
    let _: [DropCause; 7] = DropCause::ALL;
}
```

```yaml
# .github/workflows/ci.yml — trigger change: add 'feature/**' so stacked PRs
# (base feature/sprint-a-N-*) run CI. There is no push trigger.
on:
  pull_request:
    branches:
      - main
      - develop
      - 'integrate/*'
      - 'sprint/*'
      - 'fix/*'
      - 'feature/**'

# new job; existing jobs unchanged
jobs:
  crates:
    name: crates (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      # Keep in sync with rust-toolchain.toml (enforced by the version-sync job).
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: "1.98.1"
          components: clippy, rustfmt

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: crates

      - name: Format
        run: cargo fmt --check --all --manifest-path crates/Cargo.toml

      - name: Clippy
        run: cargo clippy --locked --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings

      - name: Test
        run: cargo test --locked --manifest-path crates/Cargo.toml --workspace

      # The remaining steps are host-independent, so they run once, on Linux, in bash.
      - name: Runtime dependency graph
        if: runner.os == 'Linux'
        shell: bash
        run: cargo tree --locked --manifest-path crates/Cargo.toml -p sc-observability-log -e normal --target all --prefix none --format '{p}' | sed -E 's/ \(.*$//' | LC_ALL=C sort -u | diff - crates/runtime-deps.txt

      - name: Test-isolation contract
        if: runner.os == 'Linux'
        shell: bash
        run: |
          for f in crates/*/tests/*.rs; do if grep -qE '\binit\(' "$f"; then n=$(grep -cE '#\[([A-Za-z_]+::)*test\b' "$f"); [ "$n" -eq 1 ] || { echo "isolation violation: $f has $n test fns"; exit 1; }; fi; done

      - name: Rustdoc missing-docs
        if: runner.os == 'Linux'
        shell: bash
        run: |
          cargo rustdoc --locked --manifest-path crates/Cargo.toml -p sc-observability-log -- -D missing-docs
          cargo rustdoc --locked --manifest-path crates/Cargo.toml -p sc-observability-log-macros -- -D missing-docs

      # The MSRV toolchain is installed with rustup rather than a workflow `toolchain` key:
      # scripts/check_version_sync.py:108-114 requires every such key to equal rust-toolchain.toml.
      - name: MSRV check (1.94.1)
        if: runner.os == 'Linux'
        run: |
          rustup toolchain install 1.94.1 --profile minimal --no-self-update
          cargo +1.94.1 check --locked --manifest-path crates/Cargo.toml --workspace --all-targets
```

## This Sprint Does Not Close

- Event macros (a-2) and `#[instrument]` (a-3).
- Any btit `src-tauri/` or `app/` change, including removing `tauri-plugin-log` (a-4).
- Copying the crates into `../sc-observability` (a-6) or publishing them (sc-observability release workflow).
- Ambient span or trace context for bridge records. Records carry `trace = None` until a-3.
- Hostname resolution under `ProcessIdentityPolicy::Auto`: std has no hostname API and the runtime dependency set is frozen. Raised for the a-5 review.
- Panics inside sc-observability itself (outside this crate's lint scope). The emit path contains them with `catch_unwind` and `flush`/`shutdown` contain them on the helper thread.

## Acceptance Criteria

1. `crates/` is a standalone Cargo workspace whose manifest matches the manifest code sample (members, resolver, package fields, dependencies, lints), each crate has the `clippy.toml` sample, and this sprint's diff touches no file under `src-tauri/` or `app/`.
2. Both crates declare exactly the runtime and dev dependency sets in Deliverable 2, plus `description`, `publish = false` and `[lints] workspace = true`. `sc-observability-log-macros` is pinned with `=0.1.0`.
3. The public API matches the code samples (names, signatures, enum variants and their fields). Nothing is exported beyond them except `#[doc(hidden)] __private`, and `tests/api_freeze.rs` compiles and passes.
4. Every error type in both crates is an `enum` (checked in review: `grep -rnE 'pub struct [A-Za-z]*Error' crates/*/src` prints nothing), and every variant in the error inventory has a unit test asserting its `code()` and a non-empty `remediation()`.
5. The no-panic lint set (`unwrap_used`, `expect_used`, `panic`, `unreachable`, `todo`, `unimplemented`, `indexing_slicing` = `deny`) passes under `cargo clippy … --all-targets --all-features -- -D warnings`, with allowances only through the `clippy.toml` sample and file-level `#![allow]` in `tests/*.rs`. No `#[allow]`/`#[expect]` of those lints appears under `crates/*/src`.
6. Every mapping-table row has at least one unit test. Invalid or empty targets and tags never panic and produce a valid `TargetCategory`/`ActionName`. Every `ProcessIdentityPolicy` variant is covered.
7. `tests/bridge_jsonl.rs` proves the JSONL on disk contains the mapped `target`, `action`, `message`, `fields` and `identity.pid` for tagged, untagged and `kv` records at the four enabled levels, and that the record below `LoggerConfig.level` is absent.
8. A second `init` matches `Err(InitError::AlreadyInitialized)` both while the guard is alive and after `shutdown`. Records logged after `shutdown` do not panic and are not written.
9. Logging while the queue is full does not block. `dropped_events().get(DropCause::QueueFull)` increases and `total()` equals the sum over `DropCause::ALL` (`tests/bridge_queue_full.rs`, `queue_capacity = 1`).
10. The `run_bounded` unit tests prove a blocked call returns `BoundedError::TimedOut` within its timeout and a panicking call returns `BoundedError::WorkerLost` without propagating the panic.
11. The test-isolation command in Required Validation exits 0.
12. The `crates` CI job passes on ubuntu, windows and macOS, and the existing `version sync` job still passes with the new job present.
13. `crates/runtime-deps.txt` exists and the runtime-dependency command exits 0.
14. `.github/workflows/ci.yml` `on.pull_request.branches` contains `'feature/**'`, and the a-2 stacked PR (base `feature/sprint-a-1-log-bridge`) triggers the `crates` job: its checks list shows `crates (ubuntu-latest)`, `crates (windows-latest)` and `crates (macos-latest)`.
15. `emit` calls `try_log` only inside `submit_guarded`, which uses `catch_unwind(AssertUnwindSafe(..))`, and the thread-local reentrancy guard uses `try_with` only (`! grep -rn 'IN_EMIT.with' crates/sc-observability-log/src`). `emit_core_counts_panics_and_reentry` proves three things: a panic is counted once as `LoggerPanicked`, a reentrant call from a panic hook or a nested call is counted once as `ReentrantEmit`, and the guard is released afterwards.
16. The `take_sole` unit tests prove that shutdown gains sole ownership once another clone is released, and returns `None` at the deadline while a clone is held. That path maps to `ShutdownError::TimedOut`.
17. `init` claims `INSTALLED` with `compare_exchange` before any work, resets it on `IdentityResolution` and `Logger` failures, and keeps it set after `ForeignLoggerInstalled`, with the reason in a code comment (checked in review against the `init` sample; not tested, because a retry test would call `init` more than once to install).
18. `log::Log::flush` for the bridge is a no-op (checked in review), and `tests/bridge_jsonl.rs` asserts `log::logger().flush()` returns immediately.
19. `tests/api_freeze.rs` pins `code`/`remediation` on `InitError`, `FlushError` and `ShutdownError`, the `DroppedEvents` and `DropCause` derives, and every `DropCause` variant including `ReentrantEmit`.
20. The sanitizer unit tests pass, and `__private` re-exports `record_drop`, `sanitize_label`, `target_label`, `action_label`, `field_key_label`, `LabelError`, `LabelKind` and `RESERVED_FIELD_PREFIX`.

## Required Validation

Run from the repo root in bash. The MSRV line reproduces the CI step locally. The xwin line is a local macOS cross-check only; the CI windows runner is authoritative.

- `cargo fmt --check --all --manifest-path crates/Cargo.toml`
- `cargo clippy --locked --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings`
- `cargo test --locked --manifest-path crates/Cargo.toml --workspace`
- `cargo tree --locked --manifest-path crates/Cargo.toml -p sc-observability-log -e normal --target all --prefix none --format '{p}' | sed -E 's/ \(.*$//' | LC_ALL=C sort -u | diff - crates/runtime-deps.txt`
- `for f in crates/*/tests/*.rs; do if grep -qE '\binit\(' "$f"; then n=$(grep -cE '#\[([A-Za-z_]+::)*test\b' "$f"); [ "$n" -eq 1 ] || { echo "isolation violation: $f has $n test fns"; exit 1; }; fi; done`
- `! grep -rnE 'pub struct [A-Za-z]*Error|allow\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)|expect\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)' crates/*/src`
- `cargo rustdoc --locked --manifest-path crates/Cargo.toml -p sc-observability-log -- -D missing-docs && cargo rustdoc --locked --manifest-path crates/Cargo.toml -p sc-observability-log-macros -- -D missing-docs`
- `cargo +1.94.1 check --locked --manifest-path crates/Cargo.toml --workspace --all-targets`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --manifest-path crates/Cargo.toml --target x86_64-pc-windows-msvc --workspace --all-targets`
- `! grep -rn 'IN_EMIT.with' crates/sc-observability-log/src`
- `git diff --check`
