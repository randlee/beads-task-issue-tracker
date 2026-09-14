---
id: a-3
title: "#[instrument] — tracing-compatible attribute"
status: complete
branch: feature/sprint-a-3-instrument
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-a-3-instrument
target: integrate/phase-a
recommended_model: higher-effort (sync/async codegen, trace context propagation)
dependency_relations:
  - prerequisite: a-2
    dependent: a-3
    relation: must_follow
    rationale: "reuses crates/sc-observability-log-macros/src/fields.rs (EventSpec), the a-2 field-dispatch helpers and Callsite; stack parent"
  - prerequisite: a-3
    dependent: a-5
    relation: must_follow
    rationale: "the review covers the complete crate API"
  - prerequisite: none
    parallel_pair: [a-3, a-4]
    relation: parallel_safe
    rationale: "non-intersecting: a-3 touches only crates/ (sources, tests, docs, the tokio dev-dependency, the consumer-check member) and its own sprint doc; a-4 owns src-tauri/, app/, tests/, CLAUDE.md, codebase-map, CHANGELOG; a-1 public API and runtime dependency graph frozen"
---

# Sprint a-3 — #[instrument]: tracing-compatible attribute

## Recommended Agent / Model

Recommended model: higher-effort (sync/async codegen, trace context propagation).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Add `#[sc_observability_log::instrument]`, accepting the `tracing::instrument` (tracing-attributes 0.1.31) arguments listed in the argument table with the same meanings.
- Migration from tracing 0.1 is an import rename for every supported form. The rejected-forms table lists the tracing-valid forms that fail loudly at compile time.
- An instrumented call produces exactly one completion `LogEvent` carrying duration and outcome, including when the call returns early, panics, or an async call is cancelled after its first poll.
- Every event emitted inside the call, including through the a-2 macros, the a-1 `log` bridge and the completion event itself, carries the call's `TraceContext`.

## Hard Dependencies

- a-2 pushed (stack parent): `EventSpec` field parsing (with `FieldContext::Instrument`), `Callsite` and its runtime label cache, `record_dynamic_field`, the field-dispatch helpers, the `Level` type, the event macros and `sc-observability-log-consumer-check`.
- a-1: `__private::{EventParts, emit, record_drop, target_label, action_label, field_key_label, LabelError, LabelKind}` and `DropCause::InvalidEvent`, used unchanged.

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA;
merge parent → child before every dev/fix round. PR-completion trigger: parent
PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- a-2 → a-3 — `must_follow` (a-3 follows a-2): reuses `crates/sc-observability-log-macros/src/fields.rs` (`EventSpec`), the a-2 field-dispatch helpers and `Callsite`; stack parent.
- a-3 → a-5 — `must_follow` (a-5 follows a-3): the review covers the complete crate API.
- a-3 ↔ a-4 — `parallel_safe`: a-3 touches only `crates/` and its own sprint doc; a-4 owns `src-tauri/`, `app/`, `tests/`, `CLAUDE.md`, `.claude/codebase-map.md`, `CHANGELOG.md`. a-3 does not change `tests/api_freeze.rs`, any library `[dependencies]` table or `crates/runtime-deps.txt`, so a-4's `src-tauri/Cargo.lock` stays valid (checked with `--locked` below). Attaching trace context to bridge records changes record *content* only, not the a-1 API a-4 compiles against.

Stack: `phase-a-core · layer 3`.

## Exact Targets

- `crates/Cargo.toml` (add dev-only `tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }` to `[workspace.dependencies]`)
- `crates/Cargo.lock`
- `crates/sc-observability-log/Cargo.toml` (`[dev-dependencies]` `tokio` only)
- `crates/sc-observability-log-macros/src/instrument.rs` (new)
- `crates/sc-observability-log-macros/src/lib.rs` (export `instrument`)
- `crates/sc-observability-log/src/context.rs` (new; `CallSpan`, `Entered`, `CallOutcome`, outcome-label cache, thread-local stack, id generation)
- `crates/sc-observability-log/src/lib.rs` (re-export `instrument`; `__private` span helpers; `__private::emit` attaches `current_trace()` to every event, bridge records included — see Implementation Note 3)
- `crates/sc-observability-log/docs/mapping.md` (replace the a-1 `trace = None` row with the ambient-context rule)
- `crates/sc-observability-log/tests/instrument_jsonl.rs` (new)
- `crates/sc-observability-log/tests/compat/instrument.rs` (new; shared fixture module)
- `crates/sc-observability-log/tests/compat_instrument.rs` (new)
- `crates/sc-observability-log/tests/ui/instrument_*.rs` and `*.stderr` (new; one per rejected-forms row plus `instrument_entered_across_await.rs`)
- `crates/sc-observability-log/docs/compatibility.md` (instrument section)
- `crates/sc-observability-log-consumer-check/src/lib.rs` (also `include!`s `tests/compat/instrument.rs`)
- `docs/plans/phase-a/sprint-a-3.md` (`status:` frontmatter only)

## Deliverables

Every deliverable must land production-ready for the scope this sprint claims. If that cannot happen cleanly in one sprint, split the sprint before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **`#[instrument]` attribute.** Supports every argument in the argument table, on sync and `async` free fns and methods, including `self`, `&self` and `&mut self` receivers.
2. **Completion event.** Each call emits exactly one completion `LogEvent`, whose content is defined in the completion event table, for every outcome in `CallOutcome`: normal or early (`return`, `?`) return, `Err` under `err`, a sync or async panic, and an async future dropped **after its first poll** and before completion. An async future dropped before its first poll has not run its body, so it creates no `CallSpan` and emits nothing.
3. **Trace context.** Stored in a thread-local `RefCell` stack accessed only through `LocalKey::try_with` and `RefCell::try_borrow`/`try_borrow_mut`: if the thread-local is gone (thread teardown) or already borrowed, `enter` is a no-op and `current_trace()` returns `None`. No `RefCell` borrow is held across `emit` or across any user code. `CallSpan` (`Send + Sync`, owned by the call frame or the async body) records the context; `Entered<'_>` (`!Send + !Sync`) pushes it for one synchronous section or one `poll` and pops only its own entry on drop. Any a-2 macro event, a-1 bridge record or completion event emitted for the call carries `trace = Some(TraceContext { trace_id, span_id, parent_span_id })`. Nested instrumented calls get a new `span_id`, their parent's `span_id` as `parent_span_id`, and the same `trace_id`.
4. **Sync body wrapped in a closure.** A sync fn body is run as `(move || #body)()` inside the entered section, as tracing-attributes does (`expand.rs:386-414`), so `return` and `?` leave only the closure and the outcome match always runs. The fn signature is unchanged.
5. **Async without unsafe or allocation.** An `async fn` body is wrapped as in the expansion sample: the original body is pinned on the stack with `core::pin::pin!` and driven by `core::future::poll_fn`, re-entering the context on every poll. There is no public `Instrumented` type, no `unsafe`, and no per-call `Box`.
6. **Completion emitted inside the call's context.** `finish_ok`, `finish_err` and `Drop for CallSpan` run after the body's `Entered` guard is gone, so each re-enters (`let _g = self.enter();`) around the a-1 `emit` call; the completion event's `trace.span_id` is the call's own `span_id`.
7. **`!Send` entry guard.** `Entered` contains `PhantomData<*const ()>`, so holding it across `.await` makes the future `!Send` and moving it to another thread fails to compile. Verified while planning: the thread case fails with `` `*const ()` cannot be sent between threads safely ``, the await case with `future cannot be sent between threads safely`, and the `poll_fn` expansion stays `Send`.
8. **Id generation.** Uses std only (no new dependency) and meets sc-observability's `TraceId` (32 lowercase hex) and `SpanId` (16 lowercase hex) validation. Construction goes through `TraceId::new`/`SpanId::new`; if validation ever failed, the call records `trace = None` rather than panicking (the property test proves it never happens).
9. **No-panic outcome labels.** `CallOutcome::label()` is converted to `OutcomeLabel` (whose `new` returns `Result`, `sc-observability-types-1.2.0/src/events.rs:99`, `validation.rs:204-208`) once, into `static OUTCOME_LABELS: OnceLock<[Option<OutcomeLabel>; 4]>`, selected by destructuring (no indexing). A `None` entry calls `__private::record_drop(DropCause::InvalidEvent)` and the completion event emits with `outcome = None`. A unit test proves all four labels validate.
10. **No panics.** Production code passes the a-1 no-panic lint set. The instrumented function's own panic is **never caught and never re-raised**: it unwinds through the drop guards, which only record `CallOutcome::Panicked`. This is propagation of the user's panic, not a new panic.
11. **Rejected forms.** Each row of the rejected-forms table fails to compile with a `syn::Error` naming the form.
12. **Shared compatibility fixture.** `tests/compat/instrument.rs` uses every argument-table row. It is compiled against `tracing::instrument` (compile-only), against `sc_observability_log::instrument` (executed and asserted), and inside `sc-observability-log-consumer-check`.
13. **Docs.** `docs/compatibility.md` instrument section: the argument table, the completion event table, context rules, outcomes (including the never-polled rule) and the rejected-forms table headed "valid in tracing, deliberately rejected". `docs/mapping.md`: the `trace` row updated to the ambient-context rule.

## Required Work

**Argument table** (supported forms; verified against `tracing-attributes-0.1.31/src/attr.rs` and `src/expand.rs`, and by compiling every row against tracing-attributes 0.1.31 on 1.94.1 and 1.98.1 while planning):

| Argument | Behavior |
|---|---|
| `name = "n"`, `name = NAME` (a `const &'static str` path), or a positional string literal `#[instrument("n")]` (`attr.rs:87-96`) | `action` = the value (default: the fn name), placed unchanged in the `static` `Callsite`; the a-2 `Callsite` runs a-1 `__private::action_label` on first use and caches it (no sanitization in the proc-macro) |
| `target = "t"`, `target = TGT` | `target` = the value (default: `module_path!()`), labeled at runtime by a-1 `__private::target_label` through the same `Callsite` cache |
| `level = "info"` (case-insensitive), `level = Level::INFO`, `level = LVL` (a `const` path of type `sc_observability_log::Level`), or `level = 1..=5` (1 = trace … 5 = error, `attr.rs:439-470`) | level of the completion event (default `INFO`) |
| `skip(a, b)` | the listed args (including `self`) are not recorded |
| `skip_all` | no args are recorded |
| `fields(k = v, ?x, %y, a.b = 1, { C } = 1, { C } = ?v, r#type = 1)` | extra fields, parsed by a-2 `fields.rs` with `FieldContext::Instrument`, same dispatch as a-2; `{ C } = v` gets a `static` `DynamicKey` and goes through `record_dynamic_field` (a-1 `field_key_label`, cached; an empty or reserved key omits that field and is counted); `r#type` records `"type"` |
| *(default)* | every non-skipped typed argument recorded as `fields["arg"]` using the a-2 bare-field dispatch. The receiver (`self`, `&self`, `&mut self`) **is** recorded as `fields["self"]` with Debug (`DebugKind.record(&self)`, so a non-`Debug` receiver gets the `FieldDebug` diagnostic), as tracing does (`expand.rs:165-166`); `skip(self)` removes it |
| `ret` / `ret(Debug)` / `ret(Display)` | `fields["return"]`; bare `ret` formats with **Debug** (`expand.rs:283-293`). With `err` present it records the `Ok` value; without `err` it records the whole return value (`expand.rs:386-414`) |
| `ret(level = L)`, `ret(Debug, level = L)`, `ret(Display, level = L)` | as `ret`; the completion level is `L` when the outcome is `ok` |
| `err` / `err(Debug)` / `err(Display)` | when the fn returns `Err`: outcome `error`, completion `level = ERROR` and `fields["error"]`; bare `err` formats with **Display** (`expand.rs:268-280`) |
| `err(level = L)`, `err(Debug, level = L)`, `err(Display, level = L)` | as `err`; the completion level is `L` instead of `ERROR` |

Label failures follow a-2 Deliverables 5 and 6 exactly: a `name` failing `action_label` → the completion event **still emits** with `action = None` (bridge default action) and `__private::record_drop(DropCause::InvalidEvent)` is called; a target failing `target_label` (unreachable) → the completion event is not emitted and is counted; a `{ C }` key failing `field_key_label` → only that field is omitted and counted.

`L` accepts the same forms as `level = ..`. Completion level precedence: `ok` → the `ret(level = ..)` level if given, else `level`; `error` → the `err(level = ..)` level if given, else `ERROR`; `panicked`/`cancelled` → `level`.

**Rejected forms** (one trybuild case each; checked-in stderr contains the message):

| Form | Valid in tracing-attributes 0.1.31? | trybuild case | Message |
|---|---|---|---|
| `parent = ..` | yes — deliberately rejected | `ui/instrument_parent.rs` | `` `parent` is not supported (no span parents) `` |
| `follows_from = ..` | yes — deliberately rejected | `ui/instrument_follows_from.rs` | `` `follows_from` is not supported (no span links) `` |
| deferred field without a value: `fields(x)`, `fields(a.b)` | yes — deliberately rejected | `ui/instrument_deferred_field.rs` | `` deferred field `x` without a value is not supported (tracing `field::Empty`) `` |
| deferred field `fields(x = tracing::field::Empty)` | yes — deliberately rejected | `ui/instrument_field_empty.rs` | `` deferred fields (`field::Empty`) are not supported `` |
| reserved dotted key `fields(sc_observability_log.x = 1)` | yes — deliberately rejected | `ui/instrument_reserved_key.rs` | `` field keys starting with `sc_observability_log.` are reserved `` |
| string-literal key `fields("k" = 1)` (covers `""` and reserved strings) | no (`expected ident`, verified) | `ui/instrument_literal_key.rs` | `` string-literal field keys are not supported in `#[instrument(fields(..))]`; use an identifier `` |
| applied to a non-fn item | no | `ui/instrument_non_fn.rs` | `` `#[instrument]` can only be applied to functions `` |

`ui/instrument_entered_across_await.rs` (holding `Entered` across `.await` in a `Send`-required future) is the additional `!Send` case (Deliverable 7).

**Completion event table:**

| Field | Value |
|---|---|
| `action` | `name` (sanitized and cached by the `Callsite`) |
| `target` | `target` (sanitized and cached by the `Callsite`) |
| `level` | by the completion level precedence above |
| `message` | `None` |
| `outcome` | `CallOutcome` label from `OUTCOME_LABELS`: `ok` / `error` (`err` present and `Err` returned) / `panicked` (sync or async unwind) / `cancelled` (async future dropped after its first poll, before completion) |
| `fields` | recorded args (including `self` unless skipped), plus `fields(..)`, plus `duration_ms` (u64, saturating), plus `return`/`error` when applicable |
| `trace` | this call's `TraceContext`: `emit` reads it from the stack while the completion path holds its re-entered guard |

**Outcome mechanics** (prototyped on a 4-worker tokio runtime while planning with the code in Explicit Code Samples; observed outcomes: `ok`, `error`, `cancelled` for a `tokio::time::timeout` drop, `panicked` for a spawned task that panicked after an `.await`, `panicked` for a sync panic, `ok`/`error` for early `return` and `?` in sync fns, and no event for a future dropped before its first poll):
- `Entered::drop` checks `std::thread::panicking()`; when true it sets `CallSpan.panicked` (an `AtomicBool`) before popping its own stack entry.
- `CallSpan::finish_ok`/`finish_err` consume the span, build the completion `EventParts`, set `finished`, then re-enter (`let _g = self.enter();`) and call `emit` once.
- `Drop for CallSpan` runs the same completion path only when no `finish_*` ran: `Panicked` if the flag is set or the thread is panicking, otherwise `Cancelled`. This covers a sync unwind, an async panic that the runtime catches and whose future it drops later, and a future cancelled after its first poll.
- `CallSpan::new` runs inside the async fn's body, so a never-polled future has no `CallSpan`; this is documented, not an outcome.
- Completion emission during unwinding goes through the never-panicking a-1 `emit`.

**Error inventory** (authoritative for a-3, by enum and variant):

| Enum::Variant | Where | Cause | Recovery |
|---|---|---|---|
| `CallOutcome::Ok` / `Error` / `Panicked` / `Cancelled` (recorded, not returned) | completion event `outcome` | see the completion event table | none; it is data |
| compile-time `syn::Error` per rejected-forms row | `#[instrument]` expansion (`instrument.rs`, `fields.rs`) | a form from the rejected-forms table | remove it, or keep that fn on `tracing` |
| rustc E0277 `FieldDebug` diagnostic (from a-2) | recorded argument, `self`, `ret`/`err` value or `fields(..)` value | the value implements neither `Serialize` nor `Debug` | `skip(arg)`, or record it with `?`/`%` in `fields(..)` |
| `LabelError::{Empty, Rejected} { kind: Action }` (a-1 `action_label`) → `DropCause::InvalidEvent` via `__private::record_drop` | a-2 `Callsite` cache → completion emission | `name` empty after sanitizing or rejected | completion event emits with `action = None`; counted |
| `LabelError::Rejected { kind: Target }` (a-1 `target_label`) → `DropCause::InvalidEvent` | a-2 `Callsite` cache | sanitized target rejected (unreachable) | completion event not emitted; counted |
| `LabelError::{Empty, ReservedPrefix} { kind: FieldKey }` (a-1 `field_key_label`) → `DropCause::InvalidEvent` | a-2 `DynamicKey` cache → `record_dynamic_field` | a `fields({ C } = v)` key empty or reserved after sanitizing | that field omitted, event emits; counted |
| `OutcomeLabel::new` failure (a `ValueValidationError`, discarded) → `DropCause::InvalidEvent` | `outcome_label` in `context.rs` | a `CallOutcome::label()` string rejected (unreachable; unit test proves all four validate) | completion event emits with `outcome = None`; counted |
| `trace = None` fallback (no error type) | `CallSpan::new` | generated id failed `TraceId::new`/`SpanId::new` validation (proven unreachable by AC6) | none |
| thread-local unavailable (no error type) | `enter`, `Entered::drop`, `current_trace` | `LocalKey::try_with` returns `AccessError` (thread teardown) or `try_borrow`/`try_borrow_mut` fails | `enter` pushes nothing and its guard pops nothing; `current_trace()` returns `None` |

**Tests:**
- `tests/instrument_jsonl.rs` covers every argument-table row and every `CallOutcome`.
- Nested sync calls, and an async call across an `.await` on a multi-thread runtime, prove `trace_id`/`parent_span_id` propagation, including that the context is restored after the `.await` resumes on a different worker, and that each completion event's `trace.span_id` equals the span of its own call. tokio is a **dev-dependency only**.
- Early exit in sync fns with `err`: `return Ok(..)` → `ok`; `return Err(..)` → `error`; `?` on an `Err` → `error`; `?` on the `Ok` path → `ok`; a sync fn without `err` that returns early → `ok`. Each emits exactly one completion event.
- A panicking sync fn under `catch_unwind` (in the test, not in the crate) produces `outcome = panicked`, and the panic still reaches the test's `catch_unwind`.
- An async fn that panics after an `.await`, spawned on tokio, produces `outcome = panicked`; an async fn dropped by `tokio::time::timeout` after its first poll produces `outcome = cancelled`; an instrumented future dropped without being polled produces no completion event.
- Generic methods: `fn generic_debug<T: Debug>(&self, t: T)` called with a `Serialize + Debug` value records `fields["t"]` as its Debug **string**; `fn generic_both<T: Debug + Serialize>(&self, t: T)` records JSON (a-2 static-type dispatch rule).
- `context.rs` unit tests (no `init`, and no path that reaches `record_drop`, per the a-1 counter-test rule): all four entries of `OUTCOME_LABELS` are `Some(OutcomeLabel)` equal to `label()` (asserted on `outcome_labels()`, not through `outcome_label`); `current_trace()` is `None` on a fresh thread and the stack is empty after nested enter/drop sequences.
- trybuild has exactly the cases named in the rejected-forms table, plus `instrument_entered_across_await.rs`.
- Test isolation: follows the a-1 test-isolation contract — `tests/instrument_jsonl.rs` and `tests/compat_instrument.rs` are separate binaries, each with exactly one test fn that calls `init()` (the async cases build their tokio runtime inside that fn rather than using `#[tokio::test]`).

## Explicit Code Samples

```rust
// before: use tracing::instrument;
use sc_observability_log::{info, instrument};

const ISSUES: &str = "btit.issues";

#[instrument(name = "bd_update", target = ISSUES, skip(payload), fields(project = %cwd), err(level = "warn"))]
async fn bd_update(id: String, cwd: String, payload: UpdatePayload) -> Result<Issue, String> {
    info!(name: "bd_update", id = %id, "updating");   // carries this call's TraceContext
    run(&id, &cwd, payload).await
}
```

```rust
// crates/sc-observability-log/src/lib.rs (additions)
pub use sc_observability_log_macros::instrument;

#[doc(hidden)]
pub mod __private {
    // from a-1/a-2: EventParts, enabled, emit, record_drop, Callsite, emit_callsite, record_dynamic_field,
    //               field dispatch, Level, Map, Value
    pub use crate::context::{current_trace, CallOutcome, CallSpan, Entered};
}
```

```rust
// crates/sc-observability-log/src/context.rs
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;
use sc_observability_types::{OutcomeLabel, TraceContext};
use crate::__private::{emit, record_drop, Callsite, EventParts, FieldRecord, Level, Map, Value};
use crate::DropCause;

/// Completion outcome; `label()` feeds `LogEvent.outcome`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallOutcome { Ok, Error, Panicked, Cancelled }
impl CallOutcome {
    pub const fn label(self) -> &'static str {
        match self { Self::Ok => "ok", Self::Error => "error", Self::Panicked => "panicked", Self::Cancelled => "cancelled" }
    }
}

static OUTCOME_LABELS: OnceLock<[Option<OutcomeLabel>; 4]> = OnceLock::new();

/// Converted once; the unit test asserts all four entries are `Some` (no `record_drop` path).
fn outcome_labels() -> &'static [Option<OutcomeLabel>; 4] {
    OUTCOME_LABELS.get_or_init(|| {
        [CallOutcome::Ok, CallOutcome::Error, CallOutcome::Panicked, CallOutcome::Cancelled]
            .map(|o| OutcomeLabel::new(o.label()).ok())
    })
}

/// Never panics: a label failing validation is counted and becomes `None`.
fn outcome_label(outcome: CallOutcome) -> Option<OutcomeLabel> {
    let [ok, error, panicked, cancelled] = outcome_labels();
    let label = match outcome {
        CallOutcome::Ok => ok,
        CallOutcome::Error => error,
        CallOutcome::Panicked => panicked,
        CallOutcome::Cancelled => cancelled,
    };
    if label.is_none() {
        record_drop(DropCause::InvalidEvent);
    }
    label.clone()
}

#[derive(Debug)]
struct StackEntry { key: u64, ctx: TraceContext }

thread_local! {
    static STACK: RefCell<Vec<StackEntry>> = const { RefCell::new(Vec::new()) };
}
static NEXT_KEY: AtomicU64 = AtomicU64::new(1);

/// Innermost entered context on this thread; `None` if none, or if the thread-local is unavailable.
pub fn current_trace() -> Option<TraceContext> {
    STACK
        .try_with(|s| s.try_borrow().ok().and_then(|v| v.last().map(|e| e.ctx.clone())))
        .ok()
        .flatten()
}

/// Send + Sync: owned by the instrumented call frame or its async body.
#[derive(Debug)]
pub struct CallSpan {
    callsite: &'static Callsite,
    level: Level,              // already resolved from `level`/`ret(level)`/`err(level)` by the expansion
    error_level: Level,
    key: u64,
    trace: Option<TraceContext>,
    fields: Map<String, Value>,
    started: Instant,
    panicked: AtomicBool,
    finished: bool,
}

impl CallSpan {
    /// New span id; parent and trace id from `current_trace()`, or a new trace id.
    pub fn new(callsite: &'static Callsite, level: Level, error_level: Level, fields: Map<String, Value>) -> CallSpan;

    /// Pushes this span's context; the guard pops only its own entry. No-op when `trace` is
    /// `None` or the thread-local is unavailable. The borrow ends before this returns.
    pub fn enter(&self) -> Entered<'_> {
        let pushed = match &self.trace {
            Some(ctx) => STACK
                .try_with(|s| match s.try_borrow_mut() {
                    Ok(mut v) => { v.push(StackEntry { key: self.key, ctx: ctx.clone() }); true }
                    Err(_) => false,
                })
                .unwrap_or(false),
            None => false,
        };
        Entered { span: self, pushed, _not_send: core::marker::PhantomData }
    }

    /// Consumes the span and emits the completion event once.
    pub fn finish_ok(mut self, ret: Option<FieldRecord>) { self.complete(CallOutcome::Ok, ret); }
    pub fn finish_err(mut self, error: FieldRecord) { self.complete(CallOutcome::Error, Some(error)); }

    fn complete(&mut self, outcome: CallOutcome, value: Option<FieldRecord>) {
        self.finished = true;
        let parts: EventParts = self.completion_parts(outcome, value); // takes `fields`, adds duration_ms,
                                                                       // return/error, outcome_label(outcome)
        let _g = self.enter();   // re-enter: `emit` attaches this call's TraceContext
        emit(parts);             // no RefCell borrow is held here
    }
}

/// Emits `Panicked` (flag set or thread panicking) or `Cancelled` when no `finish_*` ran.
impl Drop for CallSpan {
    fn drop(&mut self) {
        if !self.finished {
            let outcome = if self.panicked.load(Ordering::Relaxed) || std::thread::panicking() {
                CallOutcome::Panicked
            } else {
                CallOutcome::Cancelled
            };
            self.complete(outcome, None);
        }
    }
}

/// !Send + !Sync: valid for one synchronous section or one poll.
#[derive(Debug)]
pub struct Entered<'a> { span: &'a CallSpan, pushed: bool, _not_send: core::marker::PhantomData<*const ()> }

impl Drop for Entered<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.span.panicked.store(true, Ordering::Relaxed);
        }
        if self.pushed {
            let key = self.span.key;
            let _ = STACK.try_with(|s| {
                if let Ok(mut v) = s.try_borrow_mut() {
                    if let Some(pos) = v.iter().rposition(|e| e.key == key) {
                        v.remove(pos);
                    }
                }
            });
        }
    }
}
```

```rust
// Expansion of a sync fn with `ret, err` (body wrapped in a closure, as tracing-attributes does):
fn parse_count(s: &str) -> Result<u32, std::num::ParseIntError> {
    static __SC_CALLSITE: ::sc_observability_log::__private::Callsite =
        ::sc_observability_log::__private::Callsite::new(::core::module_path!(), ::core::option::Option::Some("parse_count"));
    let __sc_span = ::sc_observability_log::__private::CallSpan::new(&__SC_CALLSITE, /* level */, /* error level */, /* recorded args + fields(..) */);
    #[allow(clippy::redundant_closure_call)]
    let __sc_out = {
        let _sc_entered = __sc_span.enter();
        (move || {
            // original body, unchanged: `return` and `?` leave only this closure
            let n: u32 = s.parse()?;
            if n == 0 { return Ok(1); }
            Ok(n)
        })()
    }; // `_sc_entered` dropped here
    match &__sc_out {
        ::core::result::Result::Ok(x) => __sc_span.finish_ok(::core::option::Option::Some(::sc_observability_log::__private::debug_value(x))),
        ::core::result::Result::Err(e) => __sc_span.finish_err(::sc_observability_log::__private::display_value(e)),
    }
    __sc_out
}
// `err` only:  Ok(_) => __sc_span.finish_ok(None), Err(e) => __sc_span.finish_err(..)
// `ret` only:  __sc_span.finish_ok(Some(debug_value(&__sc_out)));   (whole return value)
// neither:     __sc_span.finish_ok(None);
// A panic in the body unwinds through `_sc_entered` and `__sc_span`, which record `panicked`.
```

```rust
// Expansion shape of an async fn (no unsafe, no Box, no Instrumented type):
async fn bd_update(id: String, cwd: String, payload: UpdatePayload) -> Result<Issue, String> {
    static __SC_CALLSITE: ::sc_observability_log::__private::Callsite =
        ::sc_observability_log::__private::Callsite::new(ISSUES, ::core::option::Option::Some("bd_update"));
    // Runs on first poll: a future dropped before its first poll emits nothing.
    let __sc_span = ::sc_observability_log::__private::CallSpan::new(&__SC_CALLSITE, /* level */, /* error level */, /* recorded args + fields(..) */);
    let __sc_out = {
        let mut __sc_body = ::core::pin::pin!(async move { /* original body */ });
        ::core::future::poll_fn(|cx| {
            let _sc_entered = __sc_span.enter();          // dropped before poll_fn returns: never held across .await
            ::core::future::Future::poll(__sc_body.as_mut(), cx)
        })
        .await
    };
    match &__sc_out {
        ::core::result::Result::Ok(_) => __sc_span.finish_ok(::core::option::Option::None),
        ::core::result::Result::Err(e) => __sc_span.finish_err(::sc_observability_log::__private::display_value(e)),
    }
    __sc_out
}
```

## Implementation Notes

Developer deviations from the plan text, each minimal and recorded here for QA and a-5:

1. **`CallSpan::new` signature.** Implemented as
   `CallSpan::new(callsite: &'static Callsite, levels: CallLevels, fields: impl FnOnce() -> Map<String, Value>)`
   instead of `new(callsite, level, error_level, fields: Map<String, Value>)`.
   The completion level precedence in Required Work needs three distinct
   levels (`ok` → `ret(level)` else `level`; `error` → `err(level)` else
   `ERROR`; `panicked`/`cancelled` → `level`), which two parameters cannot
   carry when `ret(level = ..)` is given. `CallLevels { level, ok, error }`
   (new `#[doc(hidden)] __private` item) carries them; without `err`,
   `error = level` (the `error` outcome cannot occur). The closure makes
   argument recording lazy (note 5).
2. **`CallSpan::enabled(outcome)`.** Added (`__private`) so the expansion formats
   `ret`/`err` values only when that outcome's completion event is enabled.
   When it is not, `finish_err` receives a placeholder
   `FieldRecord::Value(Value::Null)` that `complete` discards before use; the
   `finish_ok`/`finish_err` signatures are as specified.
3. **Where `emit` attaches the trace.** The plan originally listed `handle.rs`
   (corrected in Exact Targets at QA-1), but `__private::emit` is defined in
   `src/lib.rs`; the `current_trace()`
   assignment is made there (after `assemble_event`), so it covers bridge
   records, event macros and completion events alike. `handle.rs` is unchanged.
   `src/mapping.rs` changes only the `assemble_event` doc comment (it no longer
   claims to fill `trace`); its `trace: None` initializer and unit test are
   unchanged.
4. **Files beyond Exact Targets.**
   - `src/callsite.rs`: the a-2 label handling of `emit_callsite` is extracted into
     the crate-private `callsite_parts` (with an `outcome` parameter), so the
     completion event follows a-2 Deliverables 5 and 6 through the same code
     rather than a copy. `emit_callsite` behaves as before.
   - `sc-observability-log-macros/src/fields.rs`: the `FieldContext::Instrument`
     messages now match the rejected-forms table (the deferred-field message
     names the key; string-literal keys are rejected in instrument context), and
     `unraw` became `pub(crate)`; the a-2 unit test for instrument context was
     updated to the new messages. `src/event.rs`: `expand_field` became
     `pub(crate)` for reuse. No event-macro behavior changed.
   - `tests/ui.rs`: doc comment only (the glob already picks up the new cases).
5. **Laziness.** Arguments and `fields(..)` are recorded only when one of the
   completion levels is enabled when the call starts, and `complete` returns
   without emitting (and without counting) when the outcome's level is disabled,
   matching the event macros' `enabled` gate. The plan does not state a laziness
   rule for `#[instrument]`; the trace context is still created for every call.
   `tests/instrument_jsonl.rs` proves it with a `level = "trace"` call whose
   argument's `Debug` and `Serialize` impls panic.
6. **Fake return edge.** As tracing-attributes does (`expand.rs`), the rewritten
   sync closure and async block start with an unreachable
   `if false { let x: Ret = loop {}; return x; }` (`impl Trait` erased to `_`,
   omitted for `-> !`), so `?` in the body infers the fn's return type. The
   sample expansions omit it; without it `?` fails with E0282.
7. **Argument parsing strictness.** An unknown argument is a compile error
   (`` unknown `#[instrument]` argument `x` ``); tracing-attributes 0.1.31 only
   emits a deprecation warning. `name = ..`/`target = ..` accept any `const`
   path (tracing: a single identifier) and `level = ..` any path; both are
   supersets of the tracing grammar.
8. **Cross-worker test.** Besides the 4-worker tokio test (32 tasks hopping across
   `.await`), `tests/instrument_jsonl.rs` polls an instrumented future once on the
   test thread and resumes it on a spawned OS thread, which deterministically
   proves context restoration after resuming on a different thread (a tokio
   worker hop cannot be forced).
9. **QA-1 fix (RBP-F001): completion keys shadow, not silently overwrite.**
   `CallSpan::complete` originally inserted the reserved completion keys
   (`duration_ms`, `return`/`error`) with plain `Map::insert` after user
   fields/args, so a same-named user field (a parameter named `duration_ms` or
   `error`, or a `fields(duration_ms = ..)` entry) was silently dropped. The
   completion key stays authoritative (no compile error: `error` is a common
   tracing parameter name, and this is a `#[non_exhaustive]`-free frozen API),
   but the displaced value is now preserved: `context.rs`'s new
   `insert_completion_field` uses `Map::insert`'s returned `Option<Value>` to
   detect the collision and moves the old value to
   `fields["sc_observability_log.shadowed_fields"][key]`, mirroring
   `sc_observability_log.serialize_errors` in `callsite.rs`. No new
   `DropCause`. **a-5 review item:** confirm this precedence rule (completion
   key wins, user value preserved under `shadowed_fields`) is the one to keep
   long-term, versus e.g. renaming the colliding user field automatically.

## This Sprint Does Not Close

- tracing span macros (`span!`, `info_span!`, `Span::enter`) and a public span API.
- The rejected forms (`parent`, `follows_from`, deferred fields): valid in tracing, deliberately rejected.
- OTLP export of spans. `sc-observability-otlp` has no network exporter in 1.2.0.
- Any btit call-site adoption of `#[instrument]` (not in phase-a; btit uses the a-1 bridge only).
- Catching or converting user panics: they always propagate.

## Acceptance Criteria

1. Every argument-table row (including positional name, `name`/`target`/`level` given as consts, integer levels, `ret(level = ..)`/`err(level = ..)`, `fields(a.b = 1)`/`fields({ C } = 1)`/`fields(r#type = 1)`, the recorded `self`, bare `ret` = Debug and bare `err` = Display) and every completion-event field has a JSONL assertion in `tests/instrument_jsonl.rs`.
2. `tests/compat/instrument.rs` uses every argument-table row and compiles unchanged against `tracing::instrument`, against `sc_observability_log::instrument` (and passes its runtime assertions there), and inside `sc-observability-log-consumer-check`.
3. Nested and async-across-await tests prove each inner event carries the same `trace_id` with the correct `span_id`/`parent_span_id`, for a-2 macro events and a-1 bridge records alike; each completion event's `trace.span_id` equals its own call's `span_id` (not the parent's, and not `None`); and `docs/mapping.md` states the rule.
4. Each `CallOutcome` has a test: `Ok`; `Error` with the completion level and `fields.error`; `Ok` and `Error` for early `return` and for `?` in sync fns (returning ok and returning error); `Panicked` for a sync panic (still propagating to the test's `catch_unwind`) and for an async panic after `.await`; `Cancelled` for a future dropped by `tokio::time::timeout` after its first poll. A never-polled future emits nothing. Each call emits exactly one completion event.
5. Instrumented fns keep their exact signature, visibility, generics, `where` clauses, attributes and `async`-ness. The compatibility fixture covers a generic method with a `where` clause, including a `T: Debug` parameter given a `Serialize + Debug` value (recorded as its Debug string), and an instrumented `async fn` whose body is `Send` still produces a `Send` future.
6. Generated ids always pass `TraceId::new`/`SpanId::new` validation (property test over at least 10k generations), and all four `CallOutcome` labels convert to `OutcomeLabel` (unit test).
7. Every rejected-forms row has exactly the trybuild case named in the table, whose checked-in stderr contains the row's message, and `instrument_entered_across_await.rs` fails with the `!Send` error.
8. Both crates pass fmt, clippy `-D warnings` (including the a-1 no-panic lint set, with no `#[allow]` of those lints under `crates/*/src`) and the MSRV check; `context.rs` uses no `LocalKey::with`, `RefCell::borrow` or `RefCell::borrow_mut` (only the `try_` forms); the `crates` CI job passes on all three OSes.
9. API freeze and frozen graph: this sprint's diff does not touch `crates/sc-observability-log/tests/api_freeze.rs`, `crates/runtime-deps.txt` or any library `[dependencies]` table (`tokio` is a dev-dependency only); the runtime-dependency command exits 0; the consumer-check command exits 0; and when the branch contains a-4, `cargo check --locked --manifest-path src-tauri/Cargo.toml` passes without modifying `src-tauri/Cargo.lock`.
10. `tests/instrument_jsonl.rs` and `tests/compat_instrument.rs` each contain exactly one test fn that calls `init()` (the test-isolation command exits 0).

## Required Validation

Run from the repo root in bash. `<stack-parent>` is `feature/sprint-a-2-event-macros` while a-2 is unmerged and `origin/integrate/phase-a` after.

- `cargo fmt --check --all --manifest-path crates/Cargo.toml`
- `cargo clippy --locked --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings`
- `cargo test --locked --manifest-path crates/Cargo.toml --workspace`
- `cargo tree --locked --manifest-path crates/Cargo.toml -p sc-observability-log -e normal --target all --prefix none --format '{p}' | sed -E 's/ \(.*$//' | LC_ALL=C sort -u | diff - crates/runtime-deps.txt`
- `cargo tree --locked --manifest-path crates/Cargo.toml -p sc-observability-log-consumer-check -e normal --depth 1 --prefix none --format '{p}' | sed -E 's/ \(.*$//' | diff - <(printf 'sc-observability-log-consumer-check v0.1.0\nsc-observability-log v0.1.0\n')`
- `for f in crates/*/tests/*.rs; do if grep -qE '\binit\(' "$f"; then n=$(grep -cE '#\[([A-Za-z_]+::)*test\b' "$f"); [ "$n" -eq 1 ] || { echo "isolation violation: $f has $n test fns"; exit 1; }; fi; done`
- `! grep -rnE 'sanitiz' crates/sc-observability-log-macros/src`
- `! grep -nE '\.with\(|\.borrow\(\)|\.borrow_mut\(\)' crates/sc-observability-log/src/context.rs`
- `! grep -rnE 'pub struct [A-Za-z]*Error|allow\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)|expect\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)' crates/*/src`
- `git diff --exit-code <stack-parent>...HEAD -- crates/sc-observability-log/tests/api_freeze.rs crates/runtime-deps.txt`
- `cargo rustdoc --locked --manifest-path crates/Cargo.toml -p sc-observability-log -- -D missing-docs && cargo rustdoc --locked --manifest-path crates/Cargo.toml -p sc-observability-log-macros -- -D missing-docs`
- `cargo +1.94.1 check --locked --manifest-path crates/Cargo.toml --workspace --all-targets`
- `if grep -q sc-observability-log src-tauri/Cargo.toml; then cargo check --locked --manifest-path src-tauri/Cargo.toml; fi`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --manifest-path crates/Cargo.toml --target x86_64-pc-windows-msvc --workspace --all-targets`
- `git diff --check`
