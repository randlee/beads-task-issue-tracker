# tracing / log API compatibility

`sc-observability-log` matches the call syntax of the commonly used logging
crates, so migrating is a dependency change plus an import rename.

- **`log` 0.4:** `log::{trace,debug,info,warn,error}!` call sites, including
  `target:` and `key = value; "msg"`, keep working unchanged through the bridge
  installed by `sc_observability_log::init` (see `mapping.md`).
- **`tracing` 0.1 events:** this document, section "Events" (sprint a-2).
- **`tracing::instrument`:** added in sprint a-3.

## Events

`sc_observability_log::{trace, debug, info, warn, error, event}` and
`sc_observability_log::Level` accept the `tracing` 0.1 event syntax (verified
against `tracing` 0.1.44). Each call builds a structured `LogEvent`: fields are
JSON values, not formatted text.

### Migration how-to

Change the import:

```rust
// before: use tracing::{info, warn, event, Level};
use sc_observability_log::{info, warn, event, Level};
```

Nothing else changes for any form in the grammar table. The shared fixture
`tests/compat/events.rs` proves it: the same source compiles against `tracing`
0.1 and against `sc_observability_log`, and inside
`sc-observability-log-consumer-check`, whose only dependency is
`sc-observability-log`.

### Grammar table (supported forms)

| Form | Mapping |
|---|---|
| `info!("fmt {}", a)` | `message = format!(..)`; `target = module_path!()`, sanitized at runtime; `action = None` (the bridge default action) |
| `info!(target: "t", ...)`, `info!(target: TGT, ...)`, `info!(target: module_path!(), ...)`, `info!(target: concat!("a", ".b"), ...)` | `target` = the expression's value, stored in the `static` `Callsite` and sanitized at runtime on first use. The expression must be a constant `&'static str`, as in tracing (a non-constant value fails with rustc E0015 in both). |
| `info!(name: "n", ...)`, `info!(name: NAME, ...)`, `info!(name: concat!(..), ...)` | `action` = the value, sanitized at runtime on first use, same rules as `target:` |
| `info!(name: "n", target: "t", ...)` | both; **`name:` must precede `target:`**, as in tracing |
| `info!(k = v, ...)` / `info!(a.b = v, ...)` | `fields["k"]` / `fields["a.b"]` from the bare-field dispatch (Serialize JSON, else Debug string) |
| `info!("literal key" = v, ...)` | `fields["literal key"]`, same dispatch |
| `info!(r#type = v, ...)` | `fields["type"]`: the `r#` prefix is stripped, as tracing does |
| `info!({ KEY } = v, ...)` / `info!({ KEY } = ?v, ...)` / `info!({ KEY } = %v, ...)` | `fields[field_key_label(KEY)]` with the bare/`?`/`%` rule of the other rows; an empty or reserved key is omitted and counted (see "Runtime labels and keys"). `KEY` must be a constant `&'static str`, as in tracing |
| `info!(?v)` / `info!(k = ?v)` | `fields["v"\|"k"] = format!("{:?}", v)` |
| `info!(%v)` / `info!(k = %v)` | `fields["v"\|"k"] = format!("{}", v)` |
| `info!(v)` / `info!(a.b)` (shorthand, dotted allowed) | `fields["v"\|"a.b"]` as for `k = v` |
| brace field set: `info!({ k = 1, ?v }, "fmt {}", a)`, `info!({ k = 1 })`, `info!(name: "n", { k = 1 }, "m")`, `info!(target: "t", { k = 1 }, "m")`, `info!(name: "n", target: "t", { k = 1 }, "m")`, `event!(Level::INFO, { k = 1 }, "m")` | the braces are unwrapped and their contents parsed as the field list, with every field row above; a brace group followed by `=` is a `{ KEY } = v` key instead |
| fields followed by a message: `info!(k = v, "fmt {}", a)` | fields plus message |
| `event!(Level::INFO, ...)` / `event!(target: "t", Level::WARN, ...)` / `event!(name: "n", Level::INFO, ...)` / `event!(name: "n", target: "t", Level::ERROR, ...)` / `event!(LVL, ...)` with `const LVL: Level` | level from the `Level` expression, converted with `From<Level> for sc_observability_types::Level` |

A string literal not followed by `=`, a macro call (`concat!(..)`), or any other
token that cannot start a field begins the format message, as in tracing.

**Laziness.** Every form expands inside an `if` on the configured level
(`LoggerConfig.level`). When the level is disabled, no field value or message
argument is evaluated, formatted or allocated.

**`Level`.** `sc_observability_log::Level` has the associated consts `TRACE`,
`DEBUG`, `INFO`, `WARN` and `ERROR`. It deliberately has no `PartialOrd`/`Ord`:
tracing orders levels by verbosity, which would surprise here.

### Rejected forms

#### Valid in tracing, deliberately rejected

Each fails to compile with the message shown; the trybuild case under
`tests/ui/` checks in the expected stderr.

| Form | trybuild case | Message |
|---|---|---|
| `parent: ...` | `ui/event_parent.rs` | `` `parent:` is not supported (no span parents) `` |
| deferred field `k = tracing::field::Empty` (any value path ending in `field::Empty`) | `ui/event_field_empty.rs` | `` deferred fields (`field::Empty`) are not supported `` |
| empty string-literal key `"" = v` | `ui/event_empty_key.rs` | `` empty field key is not supported `` |
| reserved string key `"sc_observability_log.x" = v` (also inside braces) | `ui/event_reserved_key_string.rs` | `` field keys starting with `sc_observability_log.` are reserved `` |
| reserved dotted key `sc_observability_log.x = v` (also inside braces) | `ui/event_reserved_key_dotted.rs` | `` field keys starting with `sc_observability_log.` are reserved `` |
| span macros (`span!`, `trace_span!` … `error_span!`; not exported) | `ui/event_span_macro.rs` | rustc's unresolved-path error naming `info_span` |

Keep span parents, deferred fields and spans on `tracing`, or use
`#[instrument]` (sprint a-3).

#### Not valid in tracing either

| Form | trybuild case | Message |
|---|---|---|
| `target: .., name: ..` | `ui/event_target_before_name.rs` | `` `name:` must come before `target:` `` |
| a bare field or shorthand whose value implements neither `Serialize` nor `Debug` | `ui/field_not_serialize_or_debug.rs` | `` field value `T` implements neither `serde::Serialize` nor `core::fmt::Debug` `` (the `FieldDebug` `on_unimplemented` message, pointing at the field expression) |

Other malformed input fails with a `syn` parse error such as
`` expected `=` after field key ``.

### Reserved keys

Field keys starting with `sc_observability_log.` are reserved for the crate
itself (for example `sc_observability_log.serialize_errors`). Literal
(`"sc_observability_log.x" = v`) and dotted (`sc_observability_log.x = v`) keys
with that prefix, and the empty literal key, are rejected at compile time.
Literal and dotted keys are otherwise stored exactly as written (after removing
`r#`).

### Runtime labels and keys

The proc-macro crate never sanitizes or validates labels. The single sanitizer
is `mapping.rs` (`::` becomes `.`, every char outside `[A-Za-z0-9._-]` becomes
`_`), applied at runtime:

- **Per-call-site cache.** Each expansion declares one `static` `Callsite`
  holding the unsanitized `target` (default `module_path!()`) and `name`. The
  first enabled event at that call site runs them through `target_label` and
  `action_label` and caches both results; later events clone the cached labels.
- **`name` fails `action_label`** (empty after sanitizing, or rejected by
  `ActionName::new`): the event **still emits** with the bridge default action,
  and each such event is counted as `DropCause::InvalidEvent`.
- **target fails `target_label`** (unreachable: an empty target becomes `log`):
  the event is **not emitted** and is counted as `DropCause::InvalidEvent`.
- **`{ KEY } = v`.** Each such field gets its own `static` `DynamicKey`; the
  first use runs `KEY` through `field_key_label` and caches the result. A key
  that is empty after sanitizing, or starts with `sc_observability_log.` after
  sanitizing (`sc_observability_log::y` included), omits **only that field**; the
  event still emits and each omission is counted as `DropCause::InvalidEvent`.

Example: `info!(name: "bad name", target: "bad target::x", "m")` records
`target = "bad_target.x"` and `action = "bad_name"` and counts nothing.

### Serialization failures

A bare field whose `Serialize` impl fails records `null` under its key and the
error text under `fields["sc_observability_log.serialize_errors"][key]`.
Non-finite floats are not failures: they record `null`. See
`sc-observability-log-macros/docs/field-value-dispatch.md`.

### Field value size

Field values are inserted into events as produced by `serde_json::to_value`
(bare and `{ key } =` fields), or as the `Debug` (`?v`) or `Display` (`%v`)
string. There is no size or depth cap on individual field values or on the
formatted message. The a-1 writer queue is bounded by event count
(`queue_capacity`), not by bytes, so callers must not log unbounded collections
or untrusted large payloads as field values without truncating them first.

### Evaluation cost

When the event's level passes the `LoggerConfig.level` threshold, field values
and the message are serialized or formatted synchronously on the calling thread
inside the macro expansion, as with `tracing`. When the level is disabled, the
field and message expressions are not evaluated. In async code, keep field
`Serialize`, `Debug` and `Display` implementations cheap: the work runs on the
executor thread that called the macro.
