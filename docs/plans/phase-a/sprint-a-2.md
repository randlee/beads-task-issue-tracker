---
id: a-2
title: sc-observability-log-macros — tracing-compatible event macros
status: complete
branch: feature/sprint-a-2-event-macros
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-a-2-event-macros
target: integrate/phase-a
recommended_model: higher-effort (proc-macro parsing of the tracing field grammar)
dependency_relations:
  - prerequisite: a-1
    dependent: a-2
    relation: must_follow
    rationale: "macros expand to a-1 __private::{EventParts, enabled, emit, record_drop, target_label, action_label, field_key_label} and are re-exported from a-1 crates/sc-observability-log/src/lib.rs; stack parent"
  - prerequisite: a-2
    dependent: a-3
    relation: must_follow
    rationale: "a-3 reuses the a-2 fields.rs EventSpec parser and the __private field-dispatch helpers"
  - prerequisite: none
    parallel_pair: [a-2, a-4]
    relation: parallel_safe
    rationale: "non-intersecting: a-2 touches only crates/ (sources, tests, docs, dev-dependencies, the consumer-check member) and its own sprint doc; a-4 owns src-tauri/, app/, tests/, CLAUDE.md, codebase-map, CHANGELOG; a-1 public API and runtime dependency graph frozen"
---

# Sprint a-2 — sc-observability-log-macros: tracing-compatible event macros

## Recommended Agent / Model

Recommended model: higher-effort (proc-macro parsing of the tracing field grammar).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Add the event proc-macros to `sc-observability-log-macros`.
- Re-export `trace!`, `debug!`, `info!`, `warn!`, `error!` and `event!` from `sc-observability-log`, accepting the `tracing` 0.1 event call syntax listed in the grammar table.
- Migration from tracing 0.1 is an import rename for every supported form. The rejected-forms table lists the tracing-valid forms that fail loudly at compile time.
- The events are real structured `LogEvent`s: fields become JSON values, not formatted text.

## Hard Dependencies

- a-1 pushed (stack parent): `__private::{EventParts, enabled, emit, record_drop}`, the a-1 `mapping.rs` sanitizer re-exported as `__private::{target_label, action_label, field_key_label, LabelError}`, the frozen `tests/api_freeze.rs`, the `crates/` workspace with its lints, and the `crates` CI job.

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA;
merge parent → child before every dev/fix round. PR-completion trigger: parent
PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- a-1 → a-2 — `must_follow` (a-2 follows a-1): macros expand to a-1 `__private::{EventParts, enabled, emit, record_drop}`, label through a-1 `__private::{target_label, action_label, field_key_label}`, and are re-exported from a-1 `crates/sc-observability-log/src/lib.rs`; stack parent.
- a-2 → a-3 — `must_follow` (a-3 follows a-2): a-3 reuses the a-2 `fields.rs` `EventSpec` parser and the `__private` field-dispatch helpers.
- a-2 ↔ a-4 — `parallel_safe`: a-2 touches only `crates/` and its own sprint doc; a-4 owns `src-tauri/`, `app/`, `tests/`, `CLAUDE.md`, `.claude/codebase-map.md`, `CHANGELOG.md`. a-2 does not change `tests/api_freeze.rs`, any `[dependencies]` table of the two library crates, or `crates/runtime-deps.txt`, so a-4's `src-tauri/Cargo.lock` stays valid (checked with `--locked` below).

Stack: `phase-a-core · layer 2`.

## Exact Targets

- `crates/Cargo.toml` (add member `sc-observability-log-consumer-check`; add dev-only `trybuild = "1"`, `tracing = "0.1"` to `[workspace.dependencies]`; no runtime dependency change)
- `crates/Cargo.lock`
- `crates/sc-observability-log-macros/src/lib.rs` (export the event macros)
- `crates/sc-observability-log-macros/src/fields.rs` (new; shared with a-3)
- `crates/sc-observability-log-macros/src/event.rs` (new)
- `crates/sc-observability-log-macros/docs/field-value-dispatch.md` (new; design record for Serialize/Debug dispatch)
- `crates/sc-observability-log/Cargo.toml` (`[dev-dependencies]` `trybuild`, `tracing` only)
- `crates/sc-observability-log/src/lib.rs` (re-exports, `Level`, `__private` additions)
- `crates/sc-observability-log/src/callsite.rs` (new; `Callsite` and `DynamicKey` with their runtime label caches, `emit_callsite`, `record_dynamic_field`, field dispatch; calls a-1 `__private::{target_label, action_label, field_key_label, record_drop}` without changing them)
- `crates/sc-observability-log/tests/macros_jsonl.rs` (new)
- `crates/sc-observability-log/tests/compat/events.rs` (new; shared fixture module, not a test binary)
- `crates/sc-observability-log/tests/compat_events.rs` (new)
- `crates/sc-observability-log/tests/ui.rs` (new; trybuild driver, no `init` call)
- `crates/sc-observability-log/tests/ui/*.rs` and `*.stderr` (new; trybuild compile-fail, one file per rejected-forms row)
- `crates/sc-observability-log/docs/compatibility.md` (new; events section)
- `crates/sc-observability-log-consumer-check/Cargo.toml` (new; `publish = false`; `[dependencies]` exactly `sc-observability-log`)
- `crates/sc-observability-log-consumer-check/src/lib.rs` (new; `include!`s `tests/compat/events.rs` from the log crate)
- `docs/plans/phase-a/sprint-a-2.md` (`status:` frontmatter only)

## Deliverables

Every deliverable must land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, split the sprint before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Event macros.** `sc-observability-log-macros` exports the function-like proc-macros `trace`, `debug`, `info`, `warn`, `error` and `event`, and `sc-observability-log` re-exports them at its crate root. The runtime dependency sets of both library crates are unchanged from a-1. Dev dependencies of `sc-observability-log`: `tempfile`, `trybuild`, `tracing`.
2. **Grammar.** Every supported form in the grammar table expands, inside an `if __private::enabled(level)` block, to `__private::emit_callsite`. Nothing in the field list or message is evaluated, formatted or allocated when the level is disabled.
3. **Rejected forms.** Each row of the rejected-forms table fails to compile with the message in that row: a `syn::Error` naming the form for the parser rows, rustc's own error for the span-macro and neither-trait rows.
4. **Field value dispatch.** Bare fields dispatch through the bound-checked kind mechanism in the design record. A field whose type implements neither `serde::Serialize` nor `core::fmt::Debug` fails with the crate's own `#[diagnostic::on_unimplemented]` message on `FieldDebug`, pointing at the field expression; the hidden method name is not the primary error.
5. **Runtime label sanitization with a per-call-site cache.** The proc-macro crate does **not** sanitize or validate labels. Each expansion declares one `static` `Callsite` holding the call's `target`/`name` as `&'static str` expressions (a literal, a `const`, `module_path!()`, `concat!(..)`; the default target is `module_path!()`). The first enabled event at that call site runs them through a-1 `__private::target_label` and `__private::action_label` and caches both `Result`s in the `Callsite`'s `OnceLock`; later events clone the cached labels. Event-level behavior on a label failure (the same in a-2 and a-3):
   - **`name` fails `action_label`** (an empty name after sanitizing, `LabelError::Empty`, or `LabelError::Rejected`): the event **still emits** with `action = None`, so the bridge default action applies, and each such event calls `__private::record_drop(DropCause::InvalidEvent)`.
   - **target fails `target_label`** (only `LabelError::Rejected`, unreachable because an empty target becomes `log`): a `LogEvent` cannot exist without a target, so that event is **not emitted** and is counted with `__private::record_drop(DropCause::InvalidEvent)`.
6. **Serialization failures and reserved keys.** A failed `serde_json::to_value` records `null` under the field key and the error text under the reserved key `sc_observability_log.serialize_errors` (an object keyed by field name). Literal and dotted keys that are empty or start with `sc_observability_log.` are rejected at compile time. A `{ KEY } = v` key (a constant `&'static str`, as tracing requires) gets its own `static` `DynamicKey`; the first use runs it through a-1 `__private::field_key_label` and caches the result. On `LabelError::Empty` or `LabelError::ReservedPrefix`, `record_dynamic_field` calls `__private::record_drop(DropCause::InvalidEvent)` for each event and omits only that field; the event still emits.
7. **Shared compatibility fixture.** `tests/compat/events.rs` is one source file using every supported form in the grammar table. It is compiled three ways: `tests/compat_events.rs` includes it with `use tracing::{trace, debug, info, warn, error, event, Level};` (compile-only, proves the syntax is genuine tracing syntax) and with `use sc_observability_log::{...}` (executed and asserted against the JSONL output); `sc-observability-log-consumer-check` `include!`s it with only `sc-observability-log` as a dependency. Rejected forms are never in the fixture; they live in `tests/ui/`.
8. **Single-dependency consumer proof.** `sc-observability-log-consumer-check` compiles in `cargo check --workspace` with `[dependencies]` exactly `sc-observability-log`. trybuild cannot prove this, because it copies the tested crate's `[dependencies]` and `[dev-dependencies]` into every UI-test project (`trybuild-1.0.121/src/dependencies.rs:16-31`, `src/run.rs:239`); a consumer package does, because only declared dependencies are in its extern prelude (a stray `::sc_observability` path fails with E0433, verified while planning).
9. **`Level`.** `sc_observability_log::Level` as shown in the code samples, with a unit test for the 1:1 conversion. Its associated consts make `const LVL: Level = Level::WARN; event!(LVL, ..)` work as in tracing.
10. **No panics.** All production code added in both crates passes the a-1 no-panic lint set; macro-input errors are `syn::Error` (`to_compile_error()`), never a panic in the proc-macro.
11. **Docs.** `docs/compatibility.md`, events section: the grammar table, the rejected-forms table (headed "valid in tracing, deliberately rejected" for the tracing-valid rows), the reserved-key rule, the runtime label/key rule from Deliverables 5 and 6, and a migration how-to that is only the import change. `docs/field-value-dispatch.md`: the design record below.

## Required Work

**Grammar table** (supported forms; tracing 0.1 event macro syntax, verified against `tracing-0.1.44/src/macros.rs` and by compiling every row against tracing 0.1.44 on 1.94.1 and 1.98.1 while planning):

| Form | Mapping |
|---|---|
| `info!("fmt {}", a)` | `message = format!(..)`; `target = module_path!()`, sanitized at runtime; `action = None` (the bridge default action) |
| `info!(target: "t", ...)`, `info!(target: TGT, ...)`, `info!(target: module_path!(), ...)`, `info!(target: concat!("a", ".b"), ...)` | `target` = the expression's value, stored in the `static` `Callsite` and sanitized at runtime on first use. The expression must be a constant `&'static str`, as in tracing (a non-constant value fails with rustc E0015 in both). |
| `info!(name: "n", ...)`, `info!(name: NAME, ...)`, `info!(name: concat!(..), ...)` | `action` = the value, sanitized at runtime on first use, same rules as `target:` |
| `info!(name: "n", target: "t", ...)` | both; **`name:` must precede `target:`**, as in tracing (`target:` then `name:` does not compile against tracing 0.1.44) |
| `info!(k = v, ...)` / `info!(a.b = v, ...)` | `fields["k"]` / `fields["a.b"]` from the bare-field dispatch (Serialize JSON, else Debug string) |
| `info!("literal key" = v, ...)` | `fields["literal key"]`, same dispatch (`$k:literal = ..`, `macros.rs:2895-2924`) |
| `info!(r#type = v, ...)` | `fields["type"]`: the `r#` prefix is stripped, as tracing does (`tracing-0.1.44/src/lib.rs:1072-1105`) |
| `info!({ KEY } = v, ...)` / `info!({ KEY } = ?v, ...)` / `info!({ KEY } = %v, ...)` | `record_dynamic_field(&mut fields, &__SC_KEY_n, ..)` with `static __SC_KEY_n: DynamicKey = DynamicKey::new(KEY)`: `fields[field_key_label(KEY)]` with the bare/`?`/`%` rule of the other rows; an empty or reserved key is omitted and counted (Deliverable 6). `KEY` must be a constant `&'static str`, as in tracing |
| `info!(?v)` / `info!(k = ?v)` | `fields["v"|"k"] = format!("{:?}", v)` |
| `info!(%v)` / `info!(k = %v)` | `fields["v"|"k"] = format!("{}", v)` |
| `info!(v)` / `info!(a.b)` (shorthand, dotted allowed) | `fields["v"|"a.b"]` as for `k = v` |
| brace field set: `info!({ k = 1, ?v }, "fmt {}", a)`, `info!({ k = 1 })`, `info!(name: "n", { k = 1 }, "m")`, `info!(target: "t", { k = 1 }, "m")`, `info!(name: "n", target: "t", { k = 1 }, "m")`, `event!(Level::INFO, { k = 1 }, "m")` | `fields.rs` unwraps the braces and parses their contents as the field list, with every field row above; a brace group followed by `=` is a `{ KEY } = v` key instead |
| fields followed by a message: `info!(k = v, "fmt {}", a)` | fields plus message |
| `event!(Level::INFO, ...)` / `event!(target: "t", Level::WARN, ...)` / `event!(name: "n", Level::INFO, ...)` / `event!(name: "n", target: "t", Level::ERROR, ...)` / `event!(LVL, ...)` with `const LVL: Level` | level from the `Level` expression, converted with `From<Level> for sc_observability_types::Level` |

**Rejected forms** (one trybuild case each under `tests/ui/`, whose checked-in stderr contains the message):

| Form | Valid in tracing 0.1.44? | trybuild case | Message |
|---|---|---|---|
| `parent: ...` | yes — deliberately rejected | `ui/event_parent.rs` | `` `parent:` is not supported (no span parents) `` |
| deferred field `k = tracing::field::Empty` (any value path ending in `field::Empty`) | yes — deliberately rejected | `ui/event_field_empty.rs` | `` deferred fields (`field::Empty`) are not supported `` |
| empty string-literal key `"" = v` | yes — deliberately rejected | `ui/event_empty_key.rs` | `` empty field key is not supported `` |
| reserved string key `"sc_observability_log.x" = v` (also inside braces) | yes — deliberately rejected | `ui/event_reserved_key_string.rs` | `` field keys starting with `sc_observability_log.` are reserved `` |
| reserved dotted key `sc_observability_log.x = v` (also inside braces) | yes — deliberately rejected | `ui/event_reserved_key_dotted.rs` | `` field keys starting with `sc_observability_log.` are reserved `` |
| span macros (`span!`, `trace_span!` … `error_span!`, `info_span!` …; not exported) | yes — deliberately rejected | `ui/event_span_macro.rs` (uses `sc_observability_log::info_span!`) | rustc's unresolved-path error naming `info_span` |
| `target: .., name: ..` | no (tracing-invalid order) | `ui/event_target_before_name.rs` | `` `name:` must come before `target:` `` |
| a bare field or shorthand whose value implements neither `Serialize` nor `Debug` | no (tracing needs `Value` or `Debug`) | `ui/field_not_serialize_or_debug.rs` | the `FieldDebug` `on_unimplemented` message (Deliverable 4) |

**Field value dispatch (design record — `crates/sc-observability-log-macros/docs/field-value-dispatch.md`):**
- **Kind selection by autoref.** A bare field `k = v` / shorthand `v` expands to `{ let __v = &v; (&&FieldValue(__v)).__sc_field_kind().record(__v) }` with `SerializeKindTag` and `DebugKindTag` in scope. `SerializeKindTag` is implemented for `&FieldValue<'_, T>` only `where T: Serialize` and resolves at the first probe step; `DebugKindTag` is implemented for `FieldValue<'_, T>` for **every** `T` and resolves one autoderef later. A type implementing both takes the Serialize path.
- **Static-type dispatch.** The kind is chosen at the expansion site from the expression's *static* type. In generic code bounded only by `T: Debug`, `T: Serialize` is not provable there, so a value whose concrete type also implements `Serialize` records its **Debug string**, not JSON. Add `Serialize` to the generic bound to get JSON. Verified while planning: `fn generic_debug<T: Debug>(v: T)` called with a `Serialize + Debug` struct records `"Both { a: 4 }"`, while `fn generic_both<T: Debug + Serialize>(v: T)` records `{"a":3}`. a-3's generic-method fixture covers this.
- **Bound-checked call.** `#[diagnostic::on_unimplemented]` (stable since 1.78) only fires on an unmet trait bound, not on rustc's "no method found". The fallback kind therefore has no bound at selection time; the bound is checked by the *call* `DebugKind::record<T: ?Sized + FieldDebug>(self, v: &T)`, where `FieldDebug` carries the diagnostic and has the blanket impl `impl<T: ?Sized + Debug> FieldDebug for T`. The proc-macro emits that call with `quote_spanned!` on the field expression, so the error points at the field.
- **Verified while planning** (1.94.1 and 1.98.1) with exactly the `FieldRecord`, `record_field`, `SerializeKind::record` and `DebugKind::record` code in Explicit Code Samples: a Serialize+Debug type records its JSON (`{"a":1}`), a Debug-only type records its Debug string, `5_u32` records `5`, `f64::NAN` records `null`, `HashMap<(i32, i32), i32>` records `null` plus `serialize_errors["hm"] = "key must be a string"`, a custom `Serialize` returning `Err` records `null` plus its message, nothing panics, and a type with neither trait fails with `error[E0277]: field value `Neither` implements neither `serde::Serialize` nor `core::fmt::Debug`` plus the label and note from the attribute, with the primary span on the field expression.
- **Stability:** relies only on documented method-call autoref/autoderef probing and a stable diagnostic attribute; no `specialization`. CI checks it on 1.94.1 (MSRV step) and 1.98.1.
- **Size:** field JSON is inserted as produced by `serde_json::to_value`, with no size or depth cap (matching `LogEvent.fields: serde_json::Map<String, Value>`).
- **Serialization failure:** `SerializeKind::record` returns `FieldRecord::SerializeFailed(serde_json::Error)`; `record_field` stores `null` under the key and the error's `Display` text under `fields["sc_observability_log.serialize_errors"][key]`. Non-finite floats are not errors (`to_value(f64::NAN)` is `Value::Null`, serde_json `impl From<f64> for Value`). No `unwrap`/`expect` appears on this path.

**Error inventory** (authoritative for a-2, by enum and variant):

| Enum::Variant | Where | Cause | Recovery |
|---|---|---|---|
| `syn::Error` `` `parent:` is not supported (no span parents) `` | `fields.rs` parse (compile time) | `parent:` in an event macro | remove `parent:`; span parents need `tracing` |
| `syn::Error` `` deferred fields (`field::Empty`) are not supported `` | `fields.rs` parse | a field value path ending in `field::Empty` | record the value directly |
| `syn::Error` `` empty field key is not supported `` | `fields.rs` parse | `"" = v` | give the field a name |
| `syn::Error` `` field keys starting with `sc_observability_log.` are reserved `` | `fields.rs` parse | literal or dotted key with the reserved prefix | rename the field |
| `syn::Error` `` `name:` must come before `target:` `` | `fields.rs` parse | `target:` given before `name:` | swap them (tracing requires the same order) |
| `syn::Error` (other parse errors, for example `` expected `=` after field key ``) | `fields.rs` parse | malformed macro input | fix the call; the message and span come from syn |
| rustc E0433 unresolved `info_span` (and the other span macros) | user code (compile time) | span macros are not exported | keep spans on `tracing` or use `#[instrument]` (a-3) |
| rustc E0277 `FieldDebug` `on_unimplemented` | expanded `DebugKind::record` call | the field value implements neither `Serialize` nor `Debug` | record with `?`/`%`, or implement `Serialize` |
| `FieldRecord::SerializeFailed(serde_json::Error)` (recorded, not returned) | `SerializeKind::record` → `record_field` | the value's `Serialize` impl returned `Err` (for example non-string map keys) | none at runtime: `null` plus `serialize_errors[key]`; fix the impl or record with `?` |
| `LabelError::Empty { kind: Action }` / `LabelError::Rejected { kind: Action, .. }` (from a-1 `action_label`, cached, never returned) → `DropCause::InvalidEvent` via `__private::record_drop` | `Callsite` label cache → `emit_callsite` | `name:` empty after sanitizing, or rejected by `ActionName::new` | the event emits with `action = None` (bridge default action); counted per event; fix the name |
| `LabelError::Rejected { kind: Target, .. }` (from a-1 `target_label`) → `DropCause::InvalidEvent` | `Callsite` label cache → `emit_callsite` | sanitized target rejected by `TargetCategory::new` (unreachable: empty becomes `log`) | the event is not emitted; counted per event |
| `LabelError::Empty { kind: FieldKey }` / `LabelError::ReservedPrefix { kind: FieldKey }` (from a-1 `field_key_label`) → `DropCause::InvalidEvent` | `DynamicKey` cache → `record_dynamic_field` | a `{ KEY }` key empty after sanitizing, or starting with `sc_observability_log.` after sanitizing (`::` becomes `.`) | only that field is omitted and the event emits; counted per event; rename the key |

**Implementation constraints:**
- `fields.rs` owns parsing of `name:`, `target:`, the level argument, the brace field set, the field list and the format tail into one `EventSpec`, so a-3 can reuse it. The parser takes a `FieldContext::{Event, Instrument}` argument (a-3 uses `Instrument`, where a value-less field is a deferred field and is rejected). Parse errors are `syn::Result`.
- **Brace unwrap rule:** after the optional `name:`/`target:` prefixes (and the level for `event!`), a brace group that is **not** followed by `=` is the whole field set: its contents are parsed with the field-list parser and a comma or end of input must follow. A brace group followed by `=` is a `{ KEY } = v` key. Proven feasible while planning with syn 2.0.119 (`ParseStream::fork` plus `braced!`), including the reserved-key and `field::Empty` checks inside braces.
- `event.rs` expands `EventSpec`. `target:`/`name:` expressions are emitted unchanged into `Callsite::new(..)`; no string processing happens in the proc-macro. Expanded code references only `::sc_observability_log::__private::*` and `::core`/`::std` paths, never `sc_observability`, so users need a single dependency.
- Label and dynamic-key failures use only the a-1 `target_label`/`action_label`/`field_key_label` results and `__private::record_drop(DropCause::InvalidEvent)`, exactly as Deliverables 5 and 6 state; no panic and no `unwrap`/`expect`.

**Tests:**
- `tests/macros_jsonl.rs` initializes the a-1 guard and asserts each grammar row against the JSONL output: exact `target`, `action`, `message` and `fields` JSON types. This includes `target: TGT`, `target: module_path!()`, `name: concat!(..)`, every brace-form row, `{ KEY } = v`/`?v`/`%v`, `event!(name: "n", Level::INFO, ..)` and `event!(LVL, ..)`.
- Runtime label and key checks in `tests/macros_jsonl.rs`: with `const EMPTY: &str = ""`, `const RESERVED: &str = "sc_observability_log.x"`, `const RESERVED_PATH: &str = "sc_observability_log::y"` and `const SPACED: &str = "a b"`, one event `info!({ EMPTY } = 1, { RESERVED } = 2, { RESERVED_PATH } = 3, { SPACED } = 4, ok = 5, "m")` is present with `fields == {"a_b": 4, "ok": 5}` and `guard.dropped_events().get(DropCause::InvalidEvent)` increased by exactly 3; `info!(name: "", "m")` is present with the bridge default action and increases the count by exactly 1; `info!(name: "bad name", target: "bad target::x", "m")` records `target = "bad_target.x"`, `action = "bad_name"` and no count.
- Dispatch tests in `tests/macros_jsonl.rs`: both-traits → Serialize JSON; Debug-only → Debug string; `HashMap<(i32, i32), i32>` and a custom `Serialize` returning `Err` → `null` plus an entry under `sc_observability_log.serialize_errors`; `f64::NAN` → `null`.
- Disabled level: a call below `LoggerConfig.level` whose argument's `Debug` and `Serialize` impls panic is not evaluated.
- `callsite.rs` unit tests (no `init`, no counter assertions, per the a-1 rule that only `emit_core_counts_panics_and_reentry` asserts counter deltas): `Callsite::new("", Some("bad name"))` caches `Ok(TargetCategory("log"))` and `Some(Ok(ActionName("bad_name")))`; `Callsite::new("t", Some(""))` caches `Some(Err(LabelError::Empty { kind: LabelKind::Action }))`; `DynamicKey::new("").key()` is `Err(LabelError::Empty { kind: LabelKind::FieldKey })`, `DynamicKey::new("sc_observability_log.x").key()` and `DynamicKey::new("sc_observability_log::y").key()` are `Err(LabelError::ReservedPrefix { kind: LabelKind::FieldKey })`, and `record_dynamic_field` with `DynamicKey::new("a b")` inserts `"a_b"`. These unit tests never pass an invalid label to `emit_callsite` or `record_dynamic_field`, so they never call `record_drop`; the counted paths are asserted only in `tests/macros_jsonl.rs`.
- `tests/ui.rs` drives trybuild over `tests/ui/*.rs`: exactly the cases named in the rejected-forms table, expected stderr checked in.
- Test isolation: follows the a-1 test-isolation contract — `tests/macros_jsonl.rs` and `tests/compat_events.rs` are separate binaries, each with exactly one test fn that calls `init()`; `tests/ui.rs` does not call `init()`.

## Explicit Code Samples

```rust
// Migration is an import swap:
// before: use tracing::{info, warn};
use sc_observability_log::{info, warn};

const SYNC: &str = "sync";

fn sync_project(project: &str, issues: usize, err: &std::io::Error) {
    info!(name: SYNC, target: "btit.sync", project, issues, "synced {} issues", issues);
    warn!(name: SYNC, { error = %err, retry = true }, "sync failed");
}
```

```rust
// crates/sc-observability-log/src/lib.rs (additions)
pub use sc_observability_log_macros::{debug, error, event, info, trace, warn};

/// tracing-compatible level type (mirrors the `tracing::Level` constants).
/// No `PartialOrd`/`Ord`: tracing orders by verbosity, which would surprise here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Level(LevelInner);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LevelInner { Trace, Debug, Info, Warn, Error }
impl Level {
    pub const TRACE: Level = Level(LevelInner::Trace);
    pub const DEBUG: Level = Level(LevelInner::Debug);
    pub const INFO: Level = Level(LevelInner::Info);
    pub const WARN: Level = Level(LevelInner::Warn);
    pub const ERROR: Level = Level(LevelInner::Error);
}
impl From<Level> for sc_observability_types::Level { /* exhaustive match, 1:1 */ }

#[doc(hidden)]
pub mod __private {
    // from a-1: EventParts, enabled, emit, record_drop, target_label, action_label, field_key_label, LabelError,
    //         LabelKind, RESERVED_FIELD_PREFIX, Map, Value; DropCause is at the crate root
    pub use sc_observability_types::Level;
    pub use crate::callsite::{emit_callsite, record_dynamic_field, record_field, Callsite, DynamicKey /* , dispatch items below */};
}
```

```rust
// crates/sc-observability-log/src/callsite.rs
use std::borrow::Cow;
use std::sync::OnceLock;
use sc_observability_types::{ActionName, TargetCategory};
use crate::__private::{
    action_label, emit, field_key_label, record_drop, target_label, EventParts, LabelError, Level, Map, Value,
};
use crate::DropCause;

/// One per expansion site; `new` is const so it can initialize a `static`.
#[derive(Debug)]
pub struct Callsite {
    target: &'static str,          // literal, const, module_path!() or concat!(..): unsanitized
    action: Option<&'static str>,  // `name:`; unsanitized
    labels: OnceLock<CallsiteLabels>,
}

#[derive(Debug)]
struct CallsiteLabels {
    target: Result<TargetCategory, LabelError>,
    action: Option<Result<ActionName, LabelError>>, // None: no `name:`
}

impl Callsite {
    pub const fn new(target: &'static str, action: Option<&'static str>) -> Callsite {
        Callsite { target, action, labels: OnceLock::new() }
    }

    /// Runs the a-1 label functions once per call site, on first use.
    fn labels(&self) -> &CallsiteLabels {
        self.labels.get_or_init(|| CallsiteLabels {
            target: target_label(self.target),
            action: self.action.map(action_label),
        })
    }
}

/// Builds EventParts from the cached labels and calls `emit`; never panics.
pub fn emit_callsite(callsite: &'static Callsite, level: Level, message: Option<String>, fields: Map<String, Value>) {
    let labels = callsite.labels();
    let Ok(target) = labels.target.clone() else {
        record_drop(DropCause::InvalidEvent); // no target: the event is not emitted (unreachable)
        return;
    };
    let action = match &labels.action {
        None => None,
        Some(Ok(name)) => Some(name.clone()),
        Some(Err(_)) => {
            record_drop(DropCause::InvalidEvent); // invalid name: default action, event still emits
            None
        }
    };
    emit(EventParts { level, target, action, message, outcome: None, fields });
}

/// One per `{ KEY } = v` expansion; `KEY` is a constant `&'static str`, as tracing requires.
#[derive(Debug)]
pub struct DynamicKey {
    raw: &'static str,
    key: OnceLock<Result<String, LabelError>>,
}
impl DynamicKey {
    pub const fn new(raw: &'static str) -> DynamicKey {
        DynamicKey { raw, key: OnceLock::new() }
    }
    /// Runs a-1 `field_key_label` once and caches the result.
    pub(crate) fn key(&self) -> &Result<String, LabelError> {
        self.key.get_or_init(|| field_key_label(self.raw).map(Cow::into_owned))
    }
}

/// Inserts under the cached `field_key_label(KEY)`; an empty or reserved key omits the field and is counted.
pub fn record_dynamic_field(fields: &mut Map<String, Value>, key: &'static DynamicKey, record: FieldRecord) {
    match key.key() {
        Ok(clean) => insert_record(fields, clean.clone(), record),
        Err(_) => record_drop(DropCause::InvalidEvent),
    }
}

/// Outcome of recording one field value.
#[derive(Debug)]
pub enum FieldRecord {
    Value(Value),
    SerializeFailed(serde_json::Error),
}
/// Inserts `record` under `key`; failures go to `null` + `sc_observability_log.serialize_errors`.
pub fn record_field(fields: &mut Map<String, Value>, key: &'static str, record: FieldRecord) {
    insert_record(fields, key.to_owned(), record);
}
fn insert_record(fields: &mut Map<String, Value>, key: String, record: FieldRecord) {
    match record {
        FieldRecord::Value(value) => { fields.insert(key, value); }
        FieldRecord::SerializeFailed(err) => {
            let errors = fields
                .entry("sc_observability_log.serialize_errors")
                .or_insert_with(|| Value::Object(Map::new()));
            if let Value::Object(errors) = errors {
                errors.insert(key.clone(), Value::String(err.to_string()));
            }
            fields.insert(key, Value::Null);
        }
    }
}
pub fn debug_value<T: ?Sized + core::fmt::Debug>(v: &T) -> FieldRecord;     // `?v`
pub fn display_value<T: ?Sized + core::fmt::Display>(v: &T) -> FieldRecord; // `%v`

pub struct FieldValue<'a, T: ?Sized>(pub &'a T);
pub struct SerializeKind;
pub struct DebugKind;

/// Autoref level 0: selected only when `T: Serialize`.
pub trait SerializeKindTag {
    fn __sc_field_kind(&self) -> SerializeKind { SerializeKind }
}
impl<T: ?Sized + serde::Serialize> SerializeKindTag for &FieldValue<'_, T> {}

/// Autoref level 1: selected for every `T`; the bound is checked by `DebugKind::record`.
pub trait DebugKindTag {
    fn __sc_field_kind(&self) -> DebugKind { DebugKind }
}
impl<T: ?Sized> DebugKindTag for FieldValue<'_, T> {}

#[diagnostic::on_unimplemented(
    message = "field value `{Self}` implements neither `serde::Serialize` nor `core::fmt::Debug`",
    label = "this field value cannot be recorded",
    note = "record it with `?value` (Debug) or `%value` (Display), or implement `serde::Serialize`"
)]
pub trait FieldDebug {
    fn field_debug(&self) -> String;
}
impl<T: ?Sized + core::fmt::Debug> FieldDebug for T {
    fn field_debug(&self) -> String { format!("{self:?}") }
}

impl SerializeKind {
    pub fn record<T: ?Sized + serde::Serialize>(self, v: &T) -> FieldRecord {
        match serde_json::to_value(v) {
            Ok(value) => FieldRecord::Value(value),
            Err(err) => FieldRecord::SerializeFailed(err),
        }
    }
}
impl DebugKind {
    pub fn record<T: ?Sized + FieldDebug>(self, v: &T) -> FieldRecord {
        FieldRecord::Value(Value::String(v.field_debug()))
    }
}
```

```rust
// Expansion of: info!(name: SYNC, target: "btit.sync", { issues, { KEY } = %project }, "synced {} issues", issues);
{
    static __SC_CALLSITE: ::sc_observability_log::__private::Callsite =
        ::sc_observability_log::__private::Callsite::new("btit.sync", ::core::option::Option::Some(SYNC)); // unsanitized
    let __sc_level = ::sc_observability_log::__private::Level::Info;
    if ::sc_observability_log::__private::enabled(__sc_level) {
        #[allow(unused_imports)]
        use ::sc_observability_log::__private::{DebugKindTag as _, SerializeKindTag as _};
        let mut __sc_fields = ::sc_observability_log::__private::Map::new();
        ::sc_observability_log::__private::record_field(&mut __sc_fields, "issues", {
            let __v = &issues;
            (&&::sc_observability_log::__private::FieldValue(__v)).__sc_field_kind().record(__v) // quote_spanned! on `issues`
        });
        {
            static __SC_KEY_0: ::sc_observability_log::__private::DynamicKey =
                ::sc_observability_log::__private::DynamicKey::new(KEY);
            ::sc_observability_log::__private::record_dynamic_field(
                &mut __sc_fields, &__SC_KEY_0, ::sc_observability_log::__private::display_value(&project),
            );
        }
        ::sc_observability_log::__private::emit_callsite(
            &__SC_CALLSITE, __sc_level, ::core::option::Option::Some(::std::format!("synced {} issues", issues)), __sc_fields,
        );
    }
}
```

```toml
# crates/sc-observability-log-consumer-check/Cargo.toml
[package]
name = "sc-observability-log-consumer-check"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Compile-only proof that the event macros need only the sc-observability-log dependency."
publish = false

[dependencies]
sc-observability-log = { path = "../sc-observability-log" }

[lints]
workspace = true
```

## This Sprint Does Not Close

- `#[instrument]`, spans and trace context (a-3).
- A `log`-style `key = value; "msg"` syntax in these macros. `log` call sites stay on `log::` macros through the a-1 bridge.
- The rejected forms (`parent:`, deferred `field::Empty` fields, empty and reserved keys, span macros) and `Level` ordering: deliberately unsupported, documented in `docs/compatibility.md`.
- Any btit `src-tauri/` or `app/` change (a-4).

## Acceptance Criteria

1. Every grammar-table row has a JSONL assertion in `tests/macros_jsonl.rs`, with the exact JSON type (number, bool, string, object) for `Serialize` values and strings for `?`/`%`, including the non-literal `target:`/`name:`, brace-form, `{ KEY } = v`, `event!(name:, Level)` and const-level rows.
2. `tests/compat/events.rs` uses every grammar-table row and compiles unchanged against `tracing` 0.1, against `sc_observability_log` (and passes its runtime assertions there), and inside `sc-observability-log-consumer-check`.
3. `cargo tree --locked --manifest-path crates/Cargo.toml -p sc-observability-log-consumer-check -e normal --depth 1 --prefix none --format '{p}'` lists only the package itself and `sc-observability-log` (path suffixes aside).
4. Every rejected-forms row has exactly the trybuild case named in the table, whose checked-in stderr contains the row's message. The neither-trait stderr's primary line is the `FieldDebug` `on_unimplemented` message and its span is the field expression.
5. `docs/field-value-dispatch.md` matches the design record, including the literal trait and method signatures in Explicit Code Samples and the static-type dispatch rule. Serialization failures record `null` plus an entry under `sc_observability_log.serialize_errors` and never panic; `f64::NAN` records `null`; both dispatch tests pass on the MSRV check and the pinned toolchain.
6. With the configured level above the call's level, field values and message arguments are not evaluated, formatted or allocated (a test uses an argument whose `Debug`/`Serialize` impls panic).
7. The runtime label and key test passes: empty and reserved `{ KEY }` keys are omitted while the event still emits, an empty `name:` emits with the default action, `dropped_events().get(DropCause::InvalidEvent)` rises by exactly the number of omitted keys plus invalid names, and the proc-macro crate contains no sanitization code (`! grep -rnE 'sanitiz' crates/sc-observability-log-macros/src`).
8. `tests/macros_jsonl.rs` and `tests/compat_events.rs` each contain exactly one test fn that calls `init()` (the test-isolation command exits 0).
9. Both crates meet the fmt, clippy `-D warnings` (including the a-1 no-panic lint set, with no `#[allow]` of those lints under `crates/*/src`) and MSRV gates, and the `crates` CI job passes on all three OSes.
10. API freeze and frozen graph: this sprint's diff does not touch `crates/sc-observability-log/tests/api_freeze.rs`, `crates/runtime-deps.txt`, or the `[dependencies]` tables of `sc-observability-log` and `sc-observability-log-macros`; the runtime-dependency command exits 0; and when the branch contains a-4 (after a rebase onto `develop`), `cargo check --locked --manifest-path src-tauri/Cargo.toml` passes without modifying `src-tauri/Cargo.lock`.

## Required Validation

Run from the repo root in bash. `<stack-parent>` is `feature/sprint-a-1-log-bridge` before the a-1 PR merges and `origin/integrate/phase-a` after.

- `cargo fmt --check --all --manifest-path crates/Cargo.toml`
- `cargo clippy --locked --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings`
- `cargo test --locked --manifest-path crates/Cargo.toml --workspace`
- `cargo tree --locked --manifest-path crates/Cargo.toml -p sc-observability-log -e normal --target all --prefix none --format '{p}' | sed -E 's/ \(.*$//' | LC_ALL=C sort -u | diff - crates/runtime-deps.txt`
- `cargo tree --locked --manifest-path crates/Cargo.toml -p sc-observability-log-consumer-check -e normal --depth 1 --prefix none --format '{p}' | sed -E 's/ \(.*$//' | diff - <(printf 'sc-observability-log-consumer-check v0.1.0\nsc-observability-log v0.1.0\n')`
- `for f in crates/*/tests/*.rs; do if grep -qE '\binit\(' "$f"; then n=$(grep -cE '#\[([A-Za-z_]+::)*test\b' "$f"); [ "$n" -eq 1 ] || { echo "isolation violation: $f has $n test fns"; exit 1; }; fi; done`
- `! grep -rnE 'sanitiz' crates/sc-observability-log-macros/src`
- `! grep -rnE 'pub struct [A-Za-z]*Error|allow\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)|expect\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)' crates/*/src`
- `git diff --exit-code <stack-parent>...HEAD -- crates/sc-observability-log/tests/api_freeze.rs crates/runtime-deps.txt`
- `cargo rustdoc --locked --manifest-path crates/Cargo.toml -p sc-observability-log -- -D missing-docs && cargo rustdoc --locked --manifest-path crates/Cargo.toml -p sc-observability-log-macros -- -D missing-docs`
- `cargo +1.94.1 check --locked --manifest-path crates/Cargo.toml --workspace --all-targets`
- `if grep -q sc-observability-log src-tauri/Cargo.toml; then cargo check --locked --manifest-path src-tauri/Cargo.toml; fi`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --manifest-path crates/Cargo.toml --target x86_64-pc-windows-msvc --workspace --all-targets`
- `git diff --check`

## QA-1 Resolutions

- **ATM-QA-001:** Fixed `info!()` example in line 162 to use correct `name:` before `target:` order.
- **RSH-A2-001:** Added "Field value size" subsection to `crates/sc-observability-log/docs/compatibility.md` documenting unbounded serialization and queue bounds.
- **RSH-A2-002:** Added "Evaluation cost" subsection to `crates/sc-observability-log/docs/compatibility.md` documenting synchronous evaluation and async implications.
