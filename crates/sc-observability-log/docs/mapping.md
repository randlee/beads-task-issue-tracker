# sc-observability-log: record mapping, sanitizer, emit semantics and errors

This document records how `sc-observability-log` turns a `log::Record` into a
`sc_observability_types::LogEvent`, the rules of the single label sanitizer, the
semantics of the emit path, and the error inventory. The sections below are
taken verbatim from the sprint a-1 plan (`docs/plans/phase-a/sprint-a-1.md`).

## Implementation notes (approved deviations)

`record_to_parts` returns `Result<EventParts, LabelError>` rather than a bare
`EventParts`: the target sanitizer can reject a value after sanitizing
(`LabelError::Rejected`), and the caller must be able to drop that record and
count `DropCause::InvalidEvent` instead of receiving a placeholder. See
`docs/plans/phase-a/sprint-a-1.md` (Implementation Notes) for the full record
of this and the `tests/api_freeze.rs` lint-allow deviation.

## Record mapping

Implemented in `src/mapping.rs`.

| `log::Record` | `LogEvent` |
|---|---|
| `level()` Error/Warn/Info/Debug/Trace | `Level::Error/Warn/Info/Debug/Trace` |
| `target()` | `target = target_label(target())`: `::` → `.`; each char outside `[A-Za-z0-9._-]` → `_`; empty → `log` |
| leading `[tag]` of `args()` when `options.parse_bracket_action` and `tag` matches `[A-Za-z0-9._-]+` (`sanitize_label(tag)` borrows and is non-empty) | `action = action_label(tag)`; the tag and one following space are removed from `message` |
| otherwise | `action = options.default_action` |
| formatted `args()` | `message = Some(..)` |
| `key_values()` (the `log` `kv` feature) | `fields[key] = serde_json::Value` (numbers, bools and strings typed; others `to_string()`) |
| `module_path()`, `file()`, `line()` | `fields["code.module"]`, `fields["code.file"]`, `fields["code.line"]` (omitted when `None`) |
| none | `service` = `LoggerConfig.service_name`; `identity` = the value `init` resolved (resolved once from `LoggerConfig.process_identity`); `version` = envelope version; `timestamp` = `Timestamp::now_utc()` |
| none | `trace` = the ambient `#[instrument]` context at emit time: `__private::emit` reads `current_trace()`, the innermost `TraceContext` entered on the emitting thread, for every event (`log` bridge records, event macros and `#[instrument]` completion events alike); `None` outside any instrumented call, or when the thread-local context stack is unavailable. Events inside an instrumented call carry that call's `trace_id` and `span_id` (and its parent's `span_id` as `parent_span_id`); the completion event carries the call's own `span_id`. See `compatibility.md`, "Context rules". |
| none | `request_id`, `correlation_id`, `outcome`, `diagnostic`, `state_transition` = `None` |
| `log::Log::flush()` | no-op; nothing is flushed (use `LogGuard::flush(timeout)`) |

Implementation notes:

- `kv` fields are inserted before the `code.*` fields, so a `kv` key named
  `code.module`, `code.file` or `code.line` is overwritten by the record location.
- A target that the sanitizer cannot label (`LabelError::Rejected`, not expected
  for sanitized input) drops the record and counts `DropCause::InvalidEvent`.
- Neither `record_to_parts` nor sc-observability 1.2.0 caps the size of a
  message or a `kv` field: `LoggerConfig.rotation_max_bytes` bounds the active
  JSONL file, and `queue_capacity` bounds the emit queue by event count, but
  no setting bounds the byte size of one event. Unusually large messages or
  fields are copied and emitted as-is (see the README `No message or field
  size cap` section).

## Label sanitizer

**Mapping and the single label sanitizer:** the crate-private pure functions `record_to_parts` and `assemble_event`, specified by the mapping table above, and the label sanitizer in `mapping.rs`, which is the only sanitizer in phase-a:

- `sanitize_label(raw: &str) -> Cow<'_, str>` rewrites `::` to `.` and every char outside `[A-Za-z0-9._-]` to `_`. It borrows when `raw` is already valid and never fails.
- `target_label(raw) -> Result<TargetCategory, LabelError>` sanitizes and maps an empty result to `log`.
- `action_label(raw) -> Result<ActionName, LabelError>` sanitizes and rejects an empty result with `LabelError::Empty`.
- `field_key_label(raw) -> Result<Cow<'_, str>, LabelError>` sanitizes, rejects an empty result with `LabelError::Empty`, and rejects a result starting with `RESERVED_FIELD_PREFIX` (`"sc_observability_log."`) with `LabelError::ReservedPrefix`.
- A `TargetCategory::new`/`ActionName::new` error after sanitizing is `LabelError::Rejected { source }`. It cannot occur for sanitized input, but it is returned, not unwrapped.
- The bridge labels `Record::target()` with `target_label`. a-2 and a-3 call the same functions through `__private` at runtime, the first time a callsite is used, and cache the result in the `Callsite` `OnceLock`. This covers literal and non-literal `target`/`name` (const, `module_path!()`, `concat!`) and every `{ expr } = v` field key. The proc-macro crate does not reimplement sanitization. A label that fails is counted with `record_drop(DropCause::InvalidEvent)`; a failing field key omits only that field and the event still emits.

## Emit semantics

**Non-blocking, non-panicking, non-reentrant emit:**

- **Slot:** the global slot is a `RwLock<Option<Arc<Installed>>>`. `emit` holds the read lock only to clone the `Arc` (no I/O and no queue wait while it is held), then calls `Logger::try_log`. Every lock acquisition recovers from poisoning with `PoisonError::into_inner` (the slot holds only an `Option<Arc<_>>`, which has no invariant a panic can break).
- **Drop accounting:** every dropped event is counted under exactly one `DropCause` variant through `record_drop`. The four `TryLogError` variants map 1:1, and an empty slot is `NotInstalled`.
- **Panics:** sc-observability's `try_log` contains `expect` calls (`runtime.rs:128`, and `record_last_error` at `runtime.rs:392-398`), and a `log` record's `args()` and key-values run user `Display`/`Debug` code when rendered. `submit_guarded` calls its closure as `std::panic::catch_unwind(AssertUnwindSafe(submit))` and counts a caught panic as `LoggerPanicked`.
- **One guard per record (R-A4-003):** `submit_guarded` is the outermost boundary of every emission and is entered exactly once per record. Inside it, only the unguarded cores `handle::submit_installed` (slot read) and `handle::submit_to` (`assemble_event`, ambient trace, redaction and `try_log`) reach the logger. `Bridge::log` runs the slot read, `BridgeOptions` lookup, `record_to_parts` (target/action labelling, rendering `fmt::Arguments` and every key-value), event assembly and `try_log` inside that one guard, so a panicking formatter is counted once as `LoggerPanicked` and a formatter that logs is counted once as `ReentrantEmit` while the outer record still completes. `__private::emit` (event macros and `#[instrument]` completion events) wraps `submit_installed` in the same single guard; the macro expansions render their field values before `emit_callsite`, as before. The guarded `__private::emit` is never called from inside another guard, which would misclassify every record as `ReentrantEmit`. Only the lock-free level check (`Log::enabled`) runs before the guard; it runs no user code.
- **Why `AssertUnwindSafe`:** it is required because `Logger` holds `dyn LogSink`, `dyn Redactor` and `dyn ProcessIdentityResolver`, which are not `RefUnwindSafe`. Without the wrapper the closure fails with E0277 (compiled on 1.94.1). The assertion is sound: after a caught panic the bridge reads no logger state except by calling `try_log` again, and sc-observability reports its own poisoned mutexes by panicking again, which is caught and counted the same way.
- **Reentrancy:** `submit_guarded` first enters an `EmitScope`, a guard over `thread_local! { static IN_EMIT: Cell<bool> = const { Cell::new(false) } }`. The flag is read and written only with `LocalKey::try_with`, never `with`, and it is cleared in `Drop`, so it is also cleared while a panic unwinds. A call to `emit` while the flag is set on the same thread returns at once and is counted as `ReentrantEmit`. Examples are a panic hook that logs while `try_log` panics, or a sink or redactor that logs through `log`. A `try_with` `AccessError` is also treated as reentrant. It cannot occur for a `const` `Cell<bool>`, which has no destructor, but it is handled without panicking.

## Error inventory

Codes live in `src/error_codes.rs` with an `ALL` slice.

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
| `DropCause::LoggerPanicked` | emit path (counted) | — | a panic inside the emit guard, caught by `catch_unwind(AssertUnwindSafe(..))`: in sc-observability `try_log`, or in a `log` record's `Display`/`Debug` formatting (message arguments or a key-value) | a formatting panic: fix the value's `Display`/`Debug` impl; a `try_log` panic: report upstream and shut the guard down |
| `DropCause::ReentrantEmit` | emit path (counted) | — | a record was emitted on a thread whose emit guard is already active (`IN_EMIT` set): a formatter of the record being emitted, a panic hook, sink or redactor that logs through `log` or the macros | do not log from panic hooks, sinks or redactors that run inside the logger |
| `BoundedError::{TimedOut, Spawn { source: std::io::Error }, WorkerLost}` (crate-private) | `run_bounded` | — | mapped 1:1 into the `FlushError`/`ShutdownError` variants above | — |
| `ShutdownStep::{StillShared, FinalFlush { source: sc_observability_types::FlushError }}` (crate-private) | shutdown helper | — | `StillShared` → `ShutdownError::TimedOut`; `FinalFlush` → `ShutdownError::FinalFlush` | — |
| `LabelError::{Empty { kind: LabelKind }, ReservedPrefix { kind: LabelKind }, Rejected { kind: LabelKind, source: sc_observability_types::ValueValidationError }}` (`#[doc(hidden)]` via `__private`) | `target_label`, `action_label`, `field_key_label` | — | an empty label or key, a reserved-prefix field key, or a validation error after sanitizing | counted as `DropCause::InvalidEvent` by the caller; never returned to users |
