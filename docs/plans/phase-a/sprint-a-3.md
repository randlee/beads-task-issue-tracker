---
id: a-3
title: "#[instrument] — tracing-compatible attribute"
status: planned
branch: feature/sprint-a-3-instrument
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-a-3-instrument
target: develop
recommended_model: higher-effort (sync/async codegen, trace context propagation)
dependency_relations:
  - prerequisite: a-2
    dependent: a-3
    relation: must_follow
    rationale: "reuses crates/sc-observability-log-macros/src/fields.rs (EventSpec) and the a-2 expansion for trace-context inheritance; stack parent"
  - prerequisite: a-3
    dependent: a-5
    relation: must_follow
    rationale: "the review covers the complete crate API"
  - prerequisite: none
    parallel_pair: [a-3, a-4]
    relation: parallel_safe
    rationale: "non-intersecting: a-3 touches only crates/ sources, tests, docs and the tokio dev-dependency; a-4 owns src-tauri/, app/, tests/, CLAUDE.md, codebase-map, CHANGELOG; runtime dependency graph frozen by a-1"
---

# Sprint a-3 — #[instrument]: tracing-compatible attribute

## Recommended Agent / Model

Recommended model: higher-effort (sync/async codegen, trace context propagation).
Recommended agent: not set — the btit developer pool is pending the `arch-ctm` decision on PR #38.
Planning advice; team-lead assigns from the active pool.

## Goal

- Add `#[sc_observability_log::instrument]`. Its argument names and meanings match `tracing::instrument` (tracing-attributes 0.1.31), so migrating an instrumented fn only means changing the import.
- An instrumented call produces one completion `LogEvent` carrying duration and outcome.
- Every event emitted inside the call, including through the a-2 macros and the a-1 `log` bridge, carries the call's `TraceContext`.

## Hard Dependencies

- a-2 merged: `EventSpec` field parsing, `__private::build_event`, the `Level` type, and the event macros.

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA;
merge parent → child before every dev/fix round. PR-completion trigger: parent
PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- a-2 → a-3 — `must_follow` (a-3 follows a-2): reuses crates/sc-observability-log-macros/src/fields.rs (EventSpec) and the a-2 expansion for trace-context inheritance; stack parent
- a-3 → a-5 — `must_follow` (a-5 follows a-3): the review covers the complete crate API
- a-3 ↔ a-4 — `parallel_safe`: non-intersecting: a-3 touches only crates/ sources, tests, docs and the tokio dev-dependency; a-4 owns src-tauri/, app/, tests/, CLAUDE.md, codebase-map, CHANGELOG; runtime dependency graph frozen by a-1

Stack: `phase-a-core · layer 3`.

## Exact Targets

- `crates/sc-observability-log-macros/src/instrument.rs` (new)
- `crates/sc-observability-log-macros/src/lib.rs` (export `instrument`)
- `crates/Cargo.toml` and `crates/sc-observability-log/Cargo.toml` (dev-dependency `tokio` only)
- `crates/sc-observability-log/src/context.rs` (new; span context, id generation, `Instrumented` future)
- `crates/sc-observability-log/src/lib.rs` (re-export `instrument`; `__private` span helpers)
- `crates/sc-observability-log/src/mapping.rs` and `src/bridge.rs` (attach the current trace context to bridge records)
- `crates/sc-observability-log/tests/instrument_jsonl.rs` (new)
- `crates/sc-observability-log/tests/compat/instrument.rs` (new; shared fixture)
- `crates/sc-observability-log/tests/compat_instrument.rs` (new)
- `crates/sc-observability-log/tests/ui/instrument_*.rs` and `*.stderr` (new)
- `crates/sc-observability-log/docs/compatibility.md` (instrument section)

## Deliverables

Every deliverable must land production-ready for the scope this sprint claims. If that cannot happen cleanly in one sprint, split the sprint before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **`#[instrument]` attribute.** Supports every argument in the argument table, on sync and `async` free fns and methods, including `&self` and `&mut self`.
2. **Completion event.** Each call emits exactly one completion `LogEvent` when the fn returns, or when a sync fn unwinds. Its content is defined in the completion event table.
3. **Trace context.** Stored in a thread-local stack; `Instrumented<F>` re-enters it on every `poll`. Any a-2 macro event or a-1 bridge record emitted during the call carries `trace = Some(TraceContext { trace_id, span_id, parent_span_id })`. Nested instrumented calls get a new `span_id`, their parent's `span_id` as `parent_span_id`, and the same `trace_id`.
4. **Id generation.** Uses std only (no new dependency) and meets sc-observability's `TraceId` (32 lowercase hex) and `SpanId` (16 lowercase hex) validation.
5. **Unsupported arguments.** Fail to compile with a message naming the argument.
6. **Shared compatibility fixture.** `tests/compat/instrument.rs` uses every supported argument. It is compiled against `tracing::instrument` (compile-only) and against `sc_observability_log::instrument` (executed and asserted).
7. **Docs.** `docs/compatibility.md` instrument section: the argument table, the completion event table, context rules, and the unsupported arguments.

## Required Work

**Argument table** (per `tracing-attributes-0.1.31/src/lib.rs`):

| Argument | Behavior |
|---|---|
| `name = "n"` | `action = "n"` (default: the fn name) |
| `target = "t"` | `target = "t"` (default: `module_path!()`), sanitized |
| `level = "info"` or `level = Level::INFO` | level of the completion event (default `INFO`) |
| `skip(a, b)` | the listed args are not recorded |
| `skip_all` | no args are recorded |
| `fields(k = v, ?x, %y)` | extra fields, same grammar as a-2 (`fields.rs`) |
| *(default)* | every non-skipped argument recorded as `fields["arg"]` using the a-2 `k = v` rule. The `self` receiver is never recorded. |
| `ret` / `ret(Debug)` / `ret(Display)` | `fields["return"]` on success |
| `err` / `err(Debug)` / `err(Display)` | when the fn returns `Err`: completion `level = ERROR` and `fields["error"]`. Default format `Display`, matching tracing. |

**Unsupported:** `parent = ..`, `follows_from = ..`, `ret(level = ..)`, `err(level = ..)`, and use on non-fn items.

**Completion event table:**

| Field | Value |
|---|---|
| `action` | `name` |
| `target` | `target` |
| `level` | `level`, or `ERROR` for an `Err` under `err` |
| `message` | `None` |
| `outcome` | `ok` / `error` (`err` present and `Err` returned) / `panicked` (sync unwind); sanitized `OutcomeLabel` |
| `fields` | recorded args, plus `fields(..)`, plus `duration_ms` (u64), plus `return`/`error` when applicable |
| `trace` | this call's `TraceContext` |

**Tests:**
- `tests/instrument_jsonl.rs` covers every argument-table row and every outcome.
- Nested sync calls, and an async call across an `.await` on a multi-thread runtime, prove `trace_id`/`parent_span_id` propagation. Use tokio as a **dev-dependency only**: add `tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }` to `[workspace.dependencies]`.
- A panicking sync fn under `catch_unwind` produces `outcome = panicked`.
- Test isolation: follows the a-1 test-isolation contract — `tests/instrument_jsonl.rs` and `tests/compat_instrument.rs` are separate binaries, each with exactly one `#[test]` fn that calls `init()` (the async case builds its tokio runtime inside that fn rather than using `#[tokio::test]`).
- trybuild has one case per unsupported argument.

## Explicit Code Samples

```rust
// before: use tracing::instrument;
use sc_observability_log::{info, instrument};

#[instrument(name = "bd_update", target = "btit.issues", skip(payload), fields(project = %cwd), err)]
async fn bd_update(id: String, cwd: String, payload: UpdatePayload) -> Result<Issue, String> {
    info!(name: "bd_update", id = %id, "updating");   // carries this call's TraceContext
    run(&id, &cwd, payload).await
}

// crates/sc-observability-log/src/lib.rs (additions)
pub use sc_observability_log_macros::instrument;

#[doc(hidden)]
pub mod __private {
    pub struct SpanGuard { /* pops context; emits completion event on drop */ }
    pub fn enter_span(
        level: sc_observability_types::Level,
        target: &'static str,
        action: &'static str,
        fields: serde_json::Map<String, serde_json::Value>,
    ) -> SpanGuard;
    impl SpanGuard {
        pub fn record_ok(&mut self, ret: Option<serde_json::Value>);
        pub fn record_err(&mut self, error: serde_json::Value);
    }
    pub fn current_trace() -> Option<sc_observability_types::TraceContext>;
    pub struct Instrumented<F> { /* re-enters the span context on each poll */ }
    impl<F: core::future::Future> core::future::Future for Instrumented<F> { type Output = F::Output; /* .. */ }
}
```

## This Sprint Does Not Close

- tracing span macros (`span!`, `info_span!`, `Span::enter`) and a public span API.
- OTLP export of spans. `sc-observability-otlp` has no network exporter in 1.2.0.
- Any btit call-site adoption of `#[instrument]` (not in phase-a; btit uses the a-1 bridge only).

## Acceptance Criteria

1. Every argument-table row and every completion-event field has a JSONL assertion in `tests/instrument_jsonl.rs`.
1a. `tests/instrument_jsonl.rs` and `tests/compat_instrument.rs` each contain exactly one `#[test]` fn that calls `init()`.
2. `tests/compat/instrument.rs` compiles unchanged against `tracing::instrument` and `sc_observability_log::instrument`, and passes its runtime assertions under the latter.
3. Nested and async-across-await tests prove each inner event carries the same `trace_id` with the correct `span_id`/`parent_span_id`. a-2 macro events and a-1 bridge records both carry it.
4. A sync panic yields `outcome = panicked` and still re-raises the panic. `Err` under `err` yields `level = ERROR` with `fields.error`.
5. Instrumented fns keep their exact signature, visibility, generics, `where` clauses, attributes and `async`-ness. The compatibility fixture covers a generic method with a `where` clause.
6. Generated ids always pass `TraceId::new`/`SpanId::new` validation (property test over at least 10k generations).
7. Every unsupported argument has a trybuild case whose checked-in stderr names it.
8. Both crates pass fmt, clippy `-D warnings` and the MSRV check; the `crates` CI job passes on all three OSes.
9. The runtime dependency graph is frozen by a-1: `cargo tree --manifest-path crates/Cargo.toml -p sc-observability-log -e normal --prefix none` output is unchanged from `crates/runtime-deps.txt`, and no `[dependencies]` table in `crates/` changes (`tokio` is a dev-dependency only; keeps `parallel_safe` with a-4).

## Required Validation

Run from the repo root:

- `cargo fmt --check --all --manifest-path crates/Cargo.toml`
- `cargo clippy --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings`
- `cargo test --manifest-path crates/Cargo.toml --workspace`
- `cargo +1.94.1 check --manifest-path crates/Cargo.toml --workspace --all-targets`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --manifest-path crates/Cargo.toml --target x86_64-pc-windows-msvc --workspace --all-targets`
