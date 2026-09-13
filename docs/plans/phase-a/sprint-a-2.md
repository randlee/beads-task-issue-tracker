---
id: a-2
title: sc-observability-log-macros — tracing-compatible event macros
status: planned
branch: feature/sprint-a-2-event-macros
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-a-2-event-macros
target: develop
recommended_model: higher-effort (proc-macro parsing of the tracing field grammar)
dependency_relations:
  - prerequisite: a-1
    dependent: a-2
    relation: must_follow
    rationale: "macros expand to a-1 __private::{enabled, emit} and are re-exported from a-1 crates/sc-observability-log/src/lib.rs; stack parent"
  - prerequisite: a-2
    dependent: a-3
    relation: must_follow
    rationale: "a-3 reuses the a-2 fields.rs EventSpec parser"
  - prerequisite: none
    parallel_pair: [a-2, a-4]
    relation: parallel_safe
    rationale: "non-intersecting: a-2 touches only crates/ sources, tests, docs and dev-dependencies; a-4 owns src-tauri/, app/, tests/, CLAUDE.md, codebase-map, CHANGELOG; runtime dependency graph frozen by a-1"
---

# Sprint a-2 — sc-observability-log-macros: tracing-compatible event macros

## Recommended Agent / Model

Recommended model: higher-effort (proc-macro parsing of the tracing field grammar).
Recommended agent: not set — the btit developer pool is pending the `arch-ctm` decision on PR #38.
Planning advice; team-lead assigns from the active pool.

## Goal

- Add the `sc-observability-log-macros` proc-macro crate.
- Re-export `trace!`, `debug!`, `info!`, `warn!`, `error!` and `event!` from `sc-observability-log`. Their call syntax matches `tracing` 0.1, so migrating an event call site only means changing the import.
- The events are real structured `LogEvent`s: fields become JSON values, not formatted text.

## Hard Dependencies

- a-1 merged: `__private::{enabled, emit}`, the `crates/` workspace and the `crates` CI job.

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA;
merge parent → child before every dev/fix round. PR-completion trigger: parent
PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- a-1 → a-2 — `must_follow` (a-2 follows a-1): macros expand to a-1 __private::{enabled, emit} and are re-exported from a-1 crates/sc-observability-log/src/lib.rs; stack parent
- a-2 → a-3 — `must_follow` (a-3 follows a-2): a-3 reuses the a-2 fields.rs EventSpec parser
- a-2 ↔ a-4 — `parallel_safe`: non-intersecting: a-2 touches only crates/ sources, tests, docs and dev-dependencies; a-4 owns src-tauri/, app/, tests/, CLAUDE.md, codebase-map, CHANGELOG; runtime dependency graph frozen by a-1

Stack: `phase-a-core · layer 2`.

## Exact Targets

- `crates/Cargo.toml` (add dev-only `trybuild = "1"`, `tracing = "0.1"` to `[workspace.dependencies]`; no runtime dependency changes)
- `crates/Cargo.lock`
- `crates/sc-observability-log-macros/src/lib.rs` (export the event macros; created empty by a-1)
- `crates/sc-observability-log-macros/src/fields.rs` (new; shared with a-3)
- `crates/sc-observability-log-macros/src/event.rs` (new)
- `crates/sc-observability-log-macros/docs/field-value-dispatch.md` (new; design record for Serialize/Debug dispatch)
- `crates/sc-observability-log/Cargo.toml` (dev-dependencies `trybuild`, `tracing` only)
- `crates/sc-observability-log/src/lib.rs` (re-exports and `__private` helpers)
- `crates/sc-observability-log/tests/macros_jsonl.rs` (new)
- `crates/sc-observability-log/tests/compat/events.rs` (new; shared fixture)
- `crates/sc-observability-log/tests/compat_events.rs` (new)
- `crates/sc-observability-log/tests/ui/*.rs` and `*.stderr` (new; trybuild compile-fail)
- `crates/sc-observability-log/docs/compatibility.md` (new; events section)

## Deliverables

Every deliverable must land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, split the sprint before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **The macros crate.** `sc-observability-log-macros` (created by a-1 with its final runtime dependencies)
   exports the function-like proc-macros `trace`, `debug`, `info`, `warn`, `error` and `event`.
2. **Re-exports.** `sc-observability-log` re-exports those six macros at its crate root. Its runtime dependency set is unchanged from a-1 (which already includes `sc-observability-log-macros` and `serde`). Dev dependencies: `tempfile`, `trybuild`, `tracing`.
3. **Grammar.** Every accepted form in the grammar table below expands to `__private::enabled` / `__private::emit` with a fully built `LogEvent`, and never allocates when the level is disabled.
4. **Rejected forms.** Each form listed below fails to compile with a message that names the form and says it is unsupported.
5. **Shared compatibility fixture.** `tests/compat/events.rs` is one source file that uses every accepted form. It is included by `tests/compat_events.rs` twice: once with `use tracing::{trace, debug, info, warn, error, event, Level};` (compile-only, proving the syntax is genuine tracing syntax) and once with `use sc_observability_log::{...}` (executed and asserted against the JSONL output).
6. **`Level`.** `sc_observability_log::Level` as shown in the code samples, with a unit test for the 1:1 conversion.
7. **Docs.** `docs/compatibility.md`, events section: the grammar table, the rejected forms, and a migration how-to that is only the import change.

## Required Work

**Grammar table** (tracing 0.1 event macro syntax, per `tracing-0.1.44/src/macros.rs`):

| Form | Mapping |
|---|---|
| `info!("fmt {}", a)` | `message = format!(..)`; `target = module_path!()` sanitized; `action = options.default_action` |
| `info!(target: "t", ...)` | `target = "t"` (sanitized) |
| `info!(name: "n", ...)` | `action = "n"` (sanitized) |
| `info!(k = v, ...)` / `info!(a.b = v, ...)` | `fields["k"]` / `fields["a.b"]` = `serde_json::to_value(&v)` when `v: Serialize`, else `format!("{:?}", v)`, chosen at compile time via autoref specialization in `__private` |
| `info!(?v)` / `info!(k = ?v)` | `fields["v"|"k"] = format!("{:?}", v)` |
| `info!(%v)` / `info!(k = %v)` | `fields["v"|"k"] = format!("{}", v)` |
| `info!(v)` (shorthand) | `fields["v"]` as for `k = v` |
| fields followed by a message: `info!(k = v, "fmt {}", a)` | fields plus message |
| `event!(Level::INFO, ...)` / `event!(target: "t", Level::WARN, ...)` | level from the `Level` argument. `sc_observability_log::Level` is a new crate-owned type (a foreign `sc_observability_types::Level` cannot get associated consts) exposing tracing's `Level::TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR` constants, with `From<Level> for sc_observability_types::Level` |

**Rejected forms:** `parent: ...`, `info!(k)` with a non-identifier path, empty-key fields, any span macro name (`span!`, `info_span!` etc. are not exported), and a bare `k = v` / shorthand field whose value implements neither `serde::Serialize` nor `core::fmt::Debug`.

**Field value dispatch (design record — `crates/sc-observability-log-macros/docs/field-value-dispatch.md`):**
- Mechanism: autoref-based method resolution. The macro expands a bare field to `(&&::sc_observability_log::__private::FieldValue(&v)).__sc_field_value()`. `__private` defines two traits with that method: `SerializeField` implemented for `&FieldValue<'_, T> where T: Serialize` (resolves at the first autoref level) and `DebugField` implemented for `FieldValue<'_, T> where T: Debug` (resolves one autoderef later). The compiler picks the first candidate that satisfies its bound, so a type implementing both always takes the `Serialize` path.
- Stability: this relies only on Rust's documented method-call autoref/autoderef probing order, not on the unstable `specialization` feature. It is verified on both pinned toolchains in CI (`cargo +1.94.1` MSRV check and the pinned 1.98.1 test run) by the dispatch tests below.
- Neither trait: expansion fails with rustc's `no method named __sc_field_value found` error. The macro attaches a span on the field value so the error points at the offending field; the trybuild case `tests/ui/field_not_serialize_or_debug.rs` checks in the exact stderr, and `docs/compatibility.md` tells users to use `?v` (Debug) or `%v` (Display) instead.
- Size: field JSON is inserted into `LogEvent.fields` as produced by `serde_json::to_value`, with no size or depth cap (matching `LogEvent.fields: serde_json::Map<String, Value>`).
- Serialization failure never panics: `SerializeField::__sc_field_value` returns `Value`, not `Result`. When `serde_json::to_value` returns `Err` (read in serde_json 1.0.151 from the local cargo registry while planning; the resolved version in `crates/Cargo.lock` may differ, so the NaN and map-key dispatch tests below are the binding check: a map key that is not a string, `KeyMustBeAString`, or a custom `Serialize` impl returning `Err`), the field is recorded as `Value::String(format!("<serialize error: {err}>"))`. Non-finite floats are not errors: `to_value(f64::NAN)` yields `Value::Null` (`serde_json/src/value/from.rs`, `impl From<f64> for Value`). No `unwrap`/`expect` appears on this path, preserving a-1's "logging never blocks and never panics" guarantee.


**Implementation constraints:**
- `fields.rs` owns parsing of `target:`, `name:`, the field list and the format tail into one `EventSpec`, so a-3 can reuse it.
- `event.rs` expands `EventSpec`.
- Expanded code references only `::sc_observability_log::__private::*`, never `sc_observability`, so users need a single dependency.

**Tests:**
- `tests/macros_jsonl.rs` initializes the a-1 guard and asserts each grammar row against the JSONL output: exact `target`, `action`, `message` and `fields` JSON types.
- trybuild `tests/ui/` has one case per rejected form, with the expected stderr checked in, including `field_not_serialize_or_debug.rs`.
- Dispatch tests in `tests/macros_jsonl.rs`: a type implementing both `Serialize` and `Debug` is recorded as its Serialize JSON; a Debug-only type is recorded as its Debug string; a `HashMap<(i32, i32), i32>` (non-string keys) and a custom `Serialize` impl returning `Err` are recorded as `"<serialize error: …>"` strings without panicking; `f64::NAN` is recorded as `null`.
- Test isolation: follows the a-1 test-isolation contract — `tests/macros_jsonl.rs` and `tests/compat_events.rs` are separate binaries, each with exactly one `#[test]` fn that calls `init()`; the disabled-level (no evaluation) case runs inside `macros_jsonl.rs`'s single test using a level below the configured max.

## Explicit Code Samples

```rust
// Migration is an import swap:
// before: use tracing::{info, warn};
use sc_observability_log::{info, warn};

fn sync_project(project: &str, issues: usize, err: &std::io::Error) {
    info!(target: "btit.sync", name: "sync", project, issues, "synced {} issues", issues);
    warn!(name: "sync", error = %err, retry = true, "sync failed");
}

// crates/sc-observability-log/src/lib.rs (additions)
pub use sc_observability_log_macros::{debug, error, event, info, trace, warn};

/// tracing-compatible level type (mirrors `tracing::Level`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Level(/* private */);
impl Level {
    pub const TRACE: Level; pub const DEBUG: Level; pub const INFO: Level;
    pub const WARN: Level;  pub const ERROR: Level;
}
impl From<Level> for sc_observability_types::Level { /* 1:1 */ }

#[doc(hidden)]
pub mod __private {
    pub use serde_json::Value;
    pub fn enabled(level: sc_observability_types::Level) -> bool;          // from a-1 (macros convert crate Level first)
    pub fn emit(event: sc_observability_types::LogEvent);                  // from a-1
    pub fn build_event(
        level: sc_observability_types::Level,
        target: &str,
        action: Option<&str>,
        message: Option<String>,
        fields: serde_json::Map<String, Value>,
    ) -> sc_observability_types::LogEvent;                                  // new in a-2; sanitizes target/action
    pub struct FieldValue<'a, T: ?Sized>(pub &'a T);

    // Autoref dispatch: macro expands a bare field `k = v` / shorthand `v` to
    //     (&&::sc_observability_log::__private::FieldValue(&v)).__sc_field_value()
    pub trait SerializeField {
        fn __sc_field_value(&self) -> serde_json::Value;
    }
    impl<T: ?Sized + serde::Serialize> SerializeField for &FieldValue<'_, T> {
        fn __sc_field_value(&self) -> serde_json::Value {
            match serde_json::to_value(self.0) {
                Ok(value) => value,
                Err(err) => serde_json::Value::String(format!("<serialize error: {err}>")),
            }
        }
    }
    pub trait DebugField {
        fn __sc_field_value(&self) -> serde_json::Value;
    }
    impl<T: ?Sized + core::fmt::Debug> DebugField for FieldValue<'_, T> {
        fn __sc_field_value(&self) -> serde_json::Value {
            serde_json::Value::String(format!("{:?}", self.0))
        }
    }
    // Macro expansion brings both traits into scope: `use ::sc_observability_log::__private::{DebugField as _, SerializeField as _};`
}
```

## This Sprint Does Not Close

- `#[instrument]`, spans and trace context (a-3).
- A `log`-style `key = value; "msg"` syntax in these macros. `log` call sites stay on `log::` macros through the a-1 bridge.
- Any btit `src-tauri/` or `app/` change (a-4).

## Acceptance Criteria

1. Every grammar-table row has a JSONL assertion in `tests/macros_jsonl.rs`, with the exact JSON type (number, bool, string, object) for `Serialize` values and strings for `?`/`%`.
2. `tests/compat/events.rs` compiles unchanged against `tracing` 0.1 and against `sc_observability_log`. The `sc_observability_log` build passes its runtime assertions.
3. Every rejected form has a trybuild case whose checked-in stderr names the unsupported form (for the neither-trait case, the stderr points at the offending field span).
3a. `docs/field-value-dispatch.md` exists and matches the design record above, including the literal trait signatures in Explicit Code Samples; serialization-failure cases (non-string map keys, custom `Serialize` error) record `"<serialize error: …>"` and never panic, and `f64::NAN` records `null`; the both-traits and Debug-only dispatch tests pass on the MSRV check and the pinned toolchain.
3b. `tests/macros_jsonl.rs` and `tests/compat_events.rs` each contain exactly one `#[test]` fn that calls `init()`.
4. With the max level below the call's level, arguments are not evaluated, formatted or allocated. A test uses an argument whose `Debug`/`Serialize` implementation panics.
5. Expanded code compiles in a consumer that depends only on `sc-observability-log`. The trybuild pass cases do not declare `sc-observability` as a dependency.
6. Both crates meet the fmt/clippy `-D warnings`/MSRV gates, and the `crates` CI job passes on all three OSes.
7. The runtime dependency graph is frozen by a-1: `cargo tree --manifest-path crates/Cargo.toml -p sc-observability-log -e normal --prefix none` output is unchanged from `crates/runtime-deps.txt`, and no `[dependencies]` table in `crates/` changes (keeps `parallel_safe` with a-4).

## Required Validation

Run from the repo root:

- `cargo fmt --check --all --manifest-path crates/Cargo.toml`
- `cargo clippy --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings`
- `cargo test --manifest-path crates/Cargo.toml --workspace`
- `cargo +1.94.1 check --manifest-path crates/Cargo.toml --workspace --all-targets`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --manifest-path crates/Cargo.toml --target x86_64-pc-windows-msvc --workspace --all-targets`
