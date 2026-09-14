# sc-observability-log: record mapping, sanitizer, emit semantics and errors

This document records how `sc-observability-log` turns a `log::Record` (and a
`StructuredRecord`) into a `sc_observability_types::LogEvent`, the rules of the
single label sanitizer and of field keys, the semantics of the submission path,
the public lifecycle/control contract, and the error inventory. The mapping,
sanitizer and emit sections started from the sprint a-1 plan
(`docs/plans/phase-a/sprint-a-1.md`); the lifecycle, control, structured
submission, field-key and report sections record the a-5 R-A4-005 contract
(`docs/plans/phase-a/review-a-5.md`).

## Lifecycle and control contract

| Rule | Contract |
|---|---|
| Install once, process-global | `init` succeeds at most once per process (`INSTALLED` compare-exchange). The `log` facade has no uninstall API: after shutdown the stopped bridge stays installed, a second `init` returns `InitError::AlreadyInitialized`, and `log::set_boxed_logger` from any other crate fails. Proven in a fresh child process by `tests/reinstall_subprocess.rs`. |
| One lifecycle owner | `LogGuard` (not `Clone`; `tests/ui/log_guard_not_clone.rs`) is the only value that can stop the logger: `LogGuard::shutdown(timeout)` once, or `Drop for LogGuard` as a fallback that discards its result. |
| Control is not ownership | `LogGuard::control()` returns `LogControl` (`Clone`, `Send + Sync`, holds no logger reference): `flush(timeout)`, `health()`, `active_log_path() -> Option<PathBuf>`, `submit(StructuredRecord)`. It has no shutdown, cannot be converted into a `LogGuard` and cannot be constructed outside the crate (`tests/ui/log_control_not_owner.rs`, `tests/ui/log_control_not_constructible.rs`). Every method is defined after shutdown. |
| One core, one writer | The `log` facade (`Bridge::log`), the event macros / `#[instrument]` (`__private::emit`) and `LogControl::submit` enter the same `handle::submit_guarded` once per record and reach the same `Installed.logger`. The first two keep their unit return and discard the result after `submit_guarded` has counted the rejection; `submit` returns it. Proven by `tests/one_writer.rs`. |
| Identity ownership | `LogEvent.version`, `timestamp`, `service`, `identity` (resolved once at `init`), `trace` (ambient `#[instrument]` context), redaction (`LoggerConfig.redaction`, applied by `try_log`) and sink routing are bridge-owned for every producer. `StructuredRecord` has no field for any of them and rejects unknown keys when deserialized. |
| Timeout versus final stop | `BridgeLifecycle`: `Running` → `ShuttingDown` (shutdown started) → `Stopped` (final) — or `ShutdownTimedOut` when `shutdown` returned `ShutdownError::TimedOut` while its detached helper still waits for the logger. The helper has no deadline of its own; when it completes it stores the final health report and publishes `Stopped`, which is how late completion is observed. Proven by `tests/shutdown_timeout.rs`. |
| Versioned data contracts | `BridgeHealthReport` (`BRIDGE_HEALTH_SCHEMA_VERSION` = 1) and `StructuredRecord`, `SubmitOutcome`, `SubmitError`, `FailureReport` (`CONTROL_SCHEMA_VERSION` = 1) are serde data: `snake_case` tagged discriminants, stable `ErrorCode` strings, `Remediation` objects, no ownership handles, nothing to parse from a display string. |

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
| `key_values()` (the `log` `kv` feature) | `fields[key] = serde_json::Value` (numbers, bools and strings typed; others `to_string()`); the key is stored as written. A key that `field_key_label` rejects (empty, or `sc_observability_log.` after sanitizing) is omitted and counted as `DropCause::InvalidEvent`; the record still emits |
| `module_path()`, `file()`, `line()` | `fields["code.module"]`, `fields["code.file"]`, `fields["code.line"]` (omitted when `None`); authoritative: a `kv` value under the same key moves to `fields["sc_observability_log.shadowed_fields"][key]` |
| none | `service` = `LoggerConfig.service_name`; `identity` = the value `init` resolved (resolved once from `LoggerConfig.process_identity`); `version` = envelope version; `timestamp` = `Timestamp::now_utc()` |
| none | `trace` = the ambient `#[instrument]` context at emit time: `__private::emit` reads `current_trace()`, the innermost `TraceContext` entered on the emitting thread, for every event (`log` bridge records, event macros and `#[instrument]` completion events alike); `None` outside any instrumented call, or when the thread-local context stack is unavailable. Events inside an instrumented call carry that call's `trace_id` and `span_id` (and its parent's `span_id` as `parent_span_id`); the completion event carries the call's own `span_id`. See `compatibility.md`, "Context rules". |
| none | `request_id`, `correlation_id`, `outcome`, `diagnostic`, `state_transition` = `None` |
| `log::Log::flush()` | no-op; nothing is flushed (use `LogControl::flush(timeout)` or `LogGuard::flush(timeout)`) |

Implementation notes:

- `kv` fields are inserted before the `code.*` fields. A `kv` key named
  `code.module`, `code.file` or `code.line` is replaced by the record location,
  and the displaced value is kept under
  `fields["sc_observability_log.shadowed_fields"][key]` (see "Field keys").
- A target that the sanitizer cannot label (`LabelError::Rejected`, not expected
  for sanitized input) drops the record and counts `DropCause::InvalidEvent`.
- Neither `record_to_parts` nor sc-observability 1.2.0 caps the size of a
  message or a `kv` field: `LoggerConfig.rotation_max_bytes` bounds the active
  JSONL file, and `queue_capacity` bounds the emit queue by event count, but
  no setting bounds the byte size of one event. Unusually large messages or
  fields are copied and emitted as-is (see the README `No message or field
  size cap` section).

## Structured submission

Implemented in `src/control.rs` (`LogControl::submit`) and
`mapping::structured_to_parts`.

| `StructuredRecord` | `LogEvent` |
|---|---|
| `level: Level` (serialized `"Trace"` … `"Error"`, the `LogEvent.level` spelling) | `level`; below `LoggerConfig.level` → `Ok(SubmitOutcome::Filtered)`, nothing written or counted |
| `target: String` | `target_label(target)` (`::` → `.`, invalid chars → `_`, empty → `log`) |
| `action: Option<String>` | `action_label(action)`; `None` → `BridgeOptions.default_action`; empty after sanitizing → `SubmitError::InvalidInput(EmptyAction)` |
| `message: Option<String>` | `message`, as given |
| `fields: JsonMap` | `fields`, keys stored as written; an empty or reserved key → `SubmitError::InvalidInput(EmptyFieldKey / ReservedFieldKey { key })` |
| none | `version`, `timestamp`, `service`, `identity`, `trace` as for every other producer; `request_id`, `correlation_id`, `outcome`, `diagnostic`, `state_transition` = `None` |

Order of checks: level threshold (a record below it is `Filtered` only while
the lifecycle is `Running`), lifecycle (`SubmitError::Stopped { lifecycle }`),
then — inside the guarded core — slot read, request validation, assembly,
redaction and `try_log`. A request is never partially written.

| Outcome | Counted as |
|---|---|
| `Ok(SubmitOutcome::Accepted)` | — (admitted to the writer queue) |
| `Ok(SubmitOutcome::Filtered)` | — |
| `Err(SubmitError::QueueFull)` | `QueueFull` |
| `Err(SubmitError::InvalidInput(reason))` | `InvalidEvent` |
| `Err(SubmitError::Stopped { lifecycle })` | `NotInstalled` |
| `Err(SubmitError::Reentrant)` | `ReentrantEmit` |
| `Err(SubmitError::WriterDegraded)` | `WriterDegraded` |
| `Err(SubmitError::BackendShutdownTimedOut)` | `ShutdownTimedOut` |
| `Err(SubmitError::ContainedPanic)` | `LoggerPanicked` |

`SubmitError::drop_cause()` returns that one cause; the counter is incremented
before the error is returned.

## Field keys

One sanitizer (`mapping::field_key_label`) and one reserved prefix
(`RESERVED_FIELD_PREFIX` = `sc_observability_log.`) apply to every producer.
The per-producer difference is only *when* a violation is detected and what
the producer can report back:

| Producer | Key source | Reserved or empty key | Duplicate user key | Collision with a crate-owned key |
|---|---|---|---|---|
| Event macros, literal / dotted / `r#` key | compile time, stored as written | compile error (`ui/event_reserved_key_*.rs`, `ui/event_empty_key.rs`) | the later field wins | — (no crate-owned keys) |
| Event macros and `#[instrument] fields(..)`, `{ KEY } = v` | runtime, stored sanitized | field omitted, event emitted, counted `InvalidEvent` | the later field wins | — |
| `#[instrument]` completion event | argument names and `fields(..)` | as the two rows above | a `fields(..)` entry replaces the argument of the same name | `duration_ms`, `return`, `error` win; the user value moves to `fields["sc_observability_log.shadowed_fields"][key]` |
| `log` facade `kv` | runtime, stored as written | field omitted, record emitted, counted `InvalidEvent` | the later pair wins | `code.module`, `code.file`, `code.line` win; the user value moves to `fields["sc_observability_log.shadowed_fields"][key]` |
| `LogControl::submit` | runtime `JsonMap`, stored as written | whole request rejected: `SubmitError::InvalidInput`, nothing written, counted `InvalidEvent` | impossible (a JSON object has unique keys) | — (no crate-owned keys are added) |

Rules common to all rows:

- A key is reserved when its *sanitized* form starts with the prefix, so
  `sc_observability_log::x` is reserved too.
- Only the crate writes reserved keys: `sc_observability_log.serialize_errors`
  (a field value's `Serialize` failed) and `sc_observability_log.shadowed_fields`
  (a user value displaced by a crate-owned key).
- A producer without a result channel (facade, macros) repairs what it can and
  counts each omission once; a producer with one (`submit`) rejects instead.

## Label sanitizer

**Mapping and the single label sanitizer:** the crate-private pure functions `record_to_parts` and `assemble_event`, specified by the mapping table above, and the label sanitizer in `mapping.rs`, which is the only sanitizer in phase-a:

- `sanitize_label(raw: &str) -> Cow<'_, str>` rewrites `::` to `.` and every char outside `[A-Za-z0-9._-]` to `_`. It borrows when `raw` is already valid and never fails.
- `target_label(raw) -> Result<TargetCategory, LabelError>` sanitizes and maps an empty result to `log`.
- `action_label(raw) -> Result<ActionName, LabelError>` sanitizes and rejects an empty result with `LabelError::Empty`.
- `field_key_label(raw) -> Result<Cow<'_, str>, LabelError>` sanitizes, rejects an empty result with `LabelError::Empty`, and rejects a result starting with `RESERVED_FIELD_PREFIX` (`"sc_observability_log."`) with `LabelError::ReservedPrefix`.
- A `TargetCategory::new`/`ActionName::new` error after sanitizing is `LabelError::Rejected { source }`. It cannot occur for sanitized input, but it is returned, not unwrapped.
- The bridge labels `Record::target()` with `target_label`. a-2 and a-3 call the same functions through `__private` at runtime, the first time a callsite is used, and cache the result in the `Callsite` `OnceLock`. This covers literal and non-literal `target`/`name` (const, `module_path!()`, `concat!`) and every `{ expr } = v` field key. The proc-macro crate does not reimplement sanitization. A label that fails is counted with `record_drop(DropCause::InvalidEvent)`; a failing field key omits only that field and the event still emits.

## Emit semantics

**Non-blocking, non-panicking, non-reentrant submission** (shared by the facade, the macros and `LogControl::submit`):

- **Slot:** the global slot is a `RwLock<Option<Arc<Installed>>>`. `emit` holds the read lock only to clone the `Arc` (no I/O and no queue wait while it is held), then calls `Logger::try_log`. Every lock acquisition recovers from poisoning with `PoisonError::into_inner` (the slot holds only an `Option<Arc<_>>`, which has no invariant a panic can break).
- **Drop accounting:** every dropped event is counted under exactly one `DropCause` variant through `record_drop`. The four `TryLogError` variants map 1:1, and an empty slot is `NotInstalled`. `submit_guarded` is generic over the rejection type (`DropCause` for the facade and macros, `SubmitError` for `submit`) and counts `rejection.drop_cause()` before returning, so a caller that discards the result cannot skip or duplicate the count.
- **Panics:** sc-observability's `try_log` contains `expect` calls (`runtime.rs:128`, and `record_last_error` at `runtime.rs:392-398`), and a `log` record's `args()` and key-values run user `Display`/`Debug` code when rendered. `submit_guarded` calls its closure as `std::panic::catch_unwind(AssertUnwindSafe(submit))` and counts a caught panic as `LoggerPanicked`.
- **One guard per record (R-A4-003):** `submit_guarded` is the outermost boundary of every emission and is entered exactly once per record. Inside it, only the unguarded cores `handle::submit_installed` (slot read) and `handle::submit_to` (`assemble_event`, ambient trace, redaction and `try_log`) reach the logger. `Bridge::log` runs the slot read, `BridgeOptions` lookup, `record_to_parts` (target/action labelling, rendering `fmt::Arguments` and every key-value), event assembly and `try_log` inside that one guard, so a panicking formatter is counted once as `LoggerPanicked` and a formatter that logs is counted once as `ReentrantEmit` while the outer record still completes. `__private::emit` (event macros and `#[instrument]` completion events) wraps `submit_installed` in the same single guard; the macro expansions render their field values before `emit_callsite`, as before. The guarded `__private::emit` is never called from inside another guard, which would misclassify every record as `ReentrantEmit`. Only the lock-free level check (`Log::enabled`) runs before the guard; it runs no user code.
- **Why `AssertUnwindSafe`:** it is required because `Logger` holds `dyn LogSink`, `dyn Redactor` and `dyn ProcessIdentityResolver`, which are not `RefUnwindSafe`. Without the wrapper the closure fails with E0277 (compiled on 1.94.1). The assertion is sound: after a caught panic the bridge reads no logger state except by calling `try_log` again, and sc-observability reports its own poisoned mutexes by panicking again, which is caught and counted the same way.
- **Reentrancy:** `submit_guarded` first enters an `EmitScope`, a guard over `thread_local! { static IN_EMIT: Cell<bool> = const { Cell::new(false) } }`. The flag is read and written only with `LocalKey::try_with`, never `with`, and it is cleared in `Drop`, so it is also cleared while a panic unwinds. A call to `emit` while the flag is set on the same thread returns at once and is counted as `ReentrantEmit`. Examples are a panic hook that logs while `try_log` panics, or a sink or redactor that logs through `log`. A `try_with` `AccessError` is also treated as reentrant. It cannot occur for a `const` `Cell<bool>`, which has no destructor, but it is handled without panicking.

## Health

Implemented in `src/health.rs`; returned by `LogControl::health()` and
`LogGuard::health()` (R-A4-004, renamed `BridgeHealthReport` by R-A4-005). It is
a bridge-owned projection: it never exposes `sc_observability::Logger`.

| `BridgeHealthReport` field | Source |
|---|---|
| `schema_version` | `BRIDGE_HEALTH_SCHEMA_VERSION` (1) |
| `lifecycle` | bridge lifecycle: `Running` after `init`; `ShuttingDown` from the first step of `shutdown`/`Drop`; when that sequence returns, `Stopped` (final) for every result except `ShutdownError::TimedOut`, which leaves `ShutdownTimedOut` until the detached helper completes and publishes `Stopped` |
| `state` | `LoggingHealthReport.state` (`Healthy` → `Healthy`, `DegradedDropping` → `Degraded`, `Unavailable` → `Unavailable`) while `Running`; `Unavailable` otherwise, or when no report could be read |
| `logger` | `None` when `Logger::health()` panicked (sc-observability 1.2.0 `expect`s on its mutexes; the panic is caught) or when shutdown did not reach `Logger::shutdown`; otherwise `writer_state` (`WriterState`), `queue` (`queue_depth`, `queue_capacity`, `queue_high_water_mark`, `queue_full_drops_total`), `last_writer_error`, `last_error`, `dropped_events_total`, `flush_errors_total` |
| `file_sink` | `status`: `Disabled` when `enable_file_sink` is false, else the `jsonl-file` sink state while `Running` and `Unavailable` afterwards; `active_log_path` as captured at `init` (`None` when disabled); the sink's `last_error` |
| `console_sink` | the same for the `console` sink, without a path |
| `dropped_events` | the bridge's `DroppedEvents` counters (as `LogGuard::dropped_events()`) |

- **Timeout versus final stop:** during `ShutdownTimedOut` the slot is empty
  and no final report exists yet, so `logger` is `None`; the late completion
  sets `lifecycle: stopped` and `logger.writer_state: stopped` together.
- **Reading:** a snapshot clones the installed `Arc` under the slot read lock
  and calls `Logger::health()` inside `catch_unwind`. After the slot has been
  emptied by shutdown it returns the report read from the stopped logger right
  after `Logger::shutdown`, stored once by the shutdown helper.
- **Serialization:** every type derives `Serialize` and `Deserialize`. Enums
  are unit variants serialized as `snake_case` strings; `ErrorCode` is its
  stable string; `Remediation` keeps sc-observability's own representation, an object
  internally tagged by `"kind"` (`"recoverable"` with its ordered `steps`, or
  `"not_recoverable"` with a `justification`); `Timestamp` is
  RFC 3339 UTC; `active_log_path` is a string (a non-UTF-8 path fails to
  serialize instead of being altered). Structs are `#[non_exhaustive]`.
- **Remediation:** `DiagnosticSummary` carries no remediation, so
  `HealthDiagnostic.remediation` is chosen by code:

| code | remediation |
|---|---|
| `SC_OBSERVABILITY_LOGGER_QUEUE_FULL` | recoverable: reduce logging pressure or increase `LoggerConfig.queue_capacity`; inspect `queue.depth` and `high_water_mark` |
| `SC_OBSERVABILITY_LOGGER_WRITER_DEGRADED` | recoverable: restart the process (the bridge cannot reinstall the logger in-process); inspect `logger.last_writer_error` |
| `SC_OBSERVABILITY_LOGGER_FLUSH_FAILED` | recoverable: inspect the writer-thread flush failure; retry the flush after the writer recovers |
| `SC_OBSERVABILITY_LOGGER_SINK_WRITE_FAILED` | recoverable: check that the log directory exists, is writable and has free space |
| `SC_OBSERVABILITY_LOGGER_SHUTDOWN_TIMED_OUT` | not recoverable: still-queued events may be lost |
| `SC_OBSERVABILITY_LOGGER_SHUTDOWN` | not recoverable: the logger has shut down |
| `SC_OBSERVABILITY_LOGGER_MAINTENANCE_FAILED`, `..._MAINTENANCE_JOIN_TIMEOUT`, `..._MAINTENANCE_WORKER_FAILED` | not recoverable: handled by the logger runtime |
| no code, or any other code | not recoverable: report the diagnostic upstream |

## Error inventory

Codes live in `src/error_codes.rs` with an `ALL` slice.

| Enum::Variant (typed fields) | Returned by | `code()` | Cause | `remediation()` |
|---|---|---|---|---|
| `InitError::AlreadyInitialized` | `init` | `SC_OBSERVABILITY_LOG_ALREADY_INITIALIZED` | `INSTALLED.compare_exchange` failed: an own `init` succeeded earlier (guard alive or shut down), is running concurrently, or earlier returned `ForeignLoggerInstalled` | not recoverable: the `log` facade logger cannot be replaced; keep the first `LogGuard` |
| `InitError::ForeignLoggerInstalled { source: log::SetLoggerError }` | `init` | `SC_OBSERVABILITY_LOG_FOREIGN_LOGGER_INSTALLED` | another `log::Log` (for example `tauri-plugin-log`) is installed; the built `Logger` is shut down and `INSTALLED` stays set | remove the other logger, or call `init` before it is installed |
| `InitError::IdentityResolution { source: sc_observability_types::IdentityError }` | `init` | `SC_OBSERVABILITY_LOG_IDENTITY_RESOLUTION_FAILED` | `ProcessIdentityPolicy::Resolver` failed; `INSTALLED` is reset | fix the resolver, or use `Auto`/`Fixed`, then call `init` again |
| `InitError::Logger { source: sc_observability_types::InitError }` | `init` | the source's code (for example `SC_OBSERVABILITY_LOGGER_INIT_FAILED`) | `Logger::new` failed (log root not creatable, invalid config); `INSTALLED` is reset | the source's remediation, then call `init` again |
| `FlushError::TimedOut { timeout: Duration }` | `LogGuard::flush`, `LogControl::flush` | `SC_OBSERVABILITY_LOG_FLUSH_TIMED_OUT` | the writer did not acknowledge within `timeout`; the helper is detached and keeps its `Arc<Installed>` clone until sc-observability's flush returns | retry later or raise `timeout`; a `shutdown` before that flush returns reports `ShutdownError::TimedOut` |
| `FlushError::Logger { source: sc_observability_types::FlushError }` | `LogGuard::flush` | the source's code (`SC_OBSERVABILITY_LOGGER_FLUSH_FAILED`, `SC_OBSERVABILITY_LOGGER_WRITER_DEGRADED`) | a sink flush failed or the writer disconnected | the source's remediation |
| `FlushError::HelperSpawn { source: std::io::Error }` | `LogGuard::flush` | `SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED` | `std::thread::Builder::spawn` failed | retry; check process thread limits |
| `FlushError::HelperLost` | `LogGuard::flush` | `SC_OBSERVABILITY_LOG_HELPER_LOST` | the helper thread ended without a result (a panic inside sc-observability) | shut the guard down; the logger may be degraded |
| `FlushError::ShutDown` | `LogControl::flush` (unreachable through a live `LogGuard`) | `SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN` | the slot is empty: shutdown has started or finished, so nothing is flushed. A flush whose helper took its `Arc<Installed>` clone before shutdown emptied the slot completes normally and is awaited by the shutdown's `take_sole`, within the shutdown timeout | not recoverable: the lifecycle owner's final shutdown flushed what was queued |
| `ShutdownError::TimedOut { timeout: Duration }` | `LogGuard::shutdown` | `SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT` | sole ownership, flush and writer join did not finish within `timeout`, most often because another `Arc<Installed>` clone is still alive (the helper of an earlier timed-out `flush`, or a submission blocked in a sink or redactor). The helper is detached and keeps waiting without a deadline; lifecycle `ShutdownTimedOut` until it completes and publishes `Stopped` | none needed at process exit; still-queued events may be lost |
| `ShutdownError::FinalFlush { source: sc_observability_types::FlushError }` | `LogGuard::shutdown` | the source's code | the final flush failed; the logger was still shut down | the source's remediation |
| `ShutdownError::HelperSpawn { source: std::io::Error }` | `LogGuard::shutdown` | `SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED` | helper thread could not start; the slot is already empty | none needed at process exit |
| `ShutdownError::HelperLost` | `LogGuard::shutdown` | `SC_OBSERVABILITY_LOG_HELPER_LOST` | the helper ended without a result | none needed at process exit |
| `SubmitError::QueueFull` | `LogControl::submit` | `SC_OBSERVABILITY_LOG_SUBMIT_QUEUE_FULL` | `TryLogError::QueueFull`; counted `QueueFull` | recoverable: retry later, reduce pressure or raise `queue_capacity` |
| `SubmitError::InvalidInput(InvalidInputReason)` | `LogControl::submit` | `SC_OBSERVABILITY_LOG_SUBMIT_INVALID_INPUT` | `EmptyAction`, `RejectedAction`, `RejectedTarget`, `EmptyFieldKey`, `ReservedFieldKey { key }`, or `RejectedByLogger` (`TryLogError::InvalidEvent`); nothing written; counted `InvalidEvent` | recoverable, per reason: fix the label or key and resubmit |
| `SubmitError::Stopped { lifecycle: BridgeLifecycle }` | `LogControl::submit` | `SC_OBSERVABILITY_LOG_SUBMIT_STOPPED` | lifecycle is `ShuttingDown`, `ShutdownTimedOut` or `Stopped`, or the slot is empty; counted `NotInstalled` | not recoverable: the bridge cannot be reinstalled in this process |
| `SubmitError::Reentrant` | `LogControl::submit` | `SC_OBSERVABILITY_LOG_SUBMIT_REENTRANT` | called inside another submission on the same thread; counted `ReentrantEmit` | recoverable: do not submit from sinks, redactors or panic hooks |
| `SubmitError::WriterDegraded` | `LogControl::submit` | `SC_OBSERVABILITY_LOG_SUBMIT_WRITER_DEGRADED` | `TryLogError::WriterDegraded`; counted `WriterDegraded` | recoverable: restart the process |
| `SubmitError::BackendShutdownTimedOut` | `LogControl::submit` | `SC_OBSERVABILITY_LOG_SUBMIT_BACKEND_SHUTDOWN_TIMED_OUT` | `TryLogError::ShutdownTimedOut`; counted `ShutdownTimedOut` | not recoverable |
| `SubmitError::ContainedPanic` | `LogControl::submit` | `SC_OBSERVABILITY_LOG_SUBMIT_CONTAINED_PANIC` | a panic in the logger, a sink or a redactor was caught; counted `LoggerPanicked` | recoverable: inspect custom sinks and redactors |
| `DropCause::{QueueFull, WriterDegraded, ShutdownTimedOut}` | emit path (counted through `record_drop`, not returned) | — | the matching `TryLogError` variant | raise `queue_capacity`; inspect sc-observability health |
| `DropCause::InvalidEvent` | emit path and a-2/a-3 emission (counted through `record_drop`) | — | two causes: (1) `TryLogError::InvalidEvent` from `try_log`, the whole event dropped; (2) a runtime label (target, name or field key) failing `target_label`/`action_label`/`field_key_label` validation or the reserved-prefix check in a-2/a-3 emission, a failing field key omitting only that field | (1) inspect the event contents; (2) fix the label or the key |
| `DropCause::NotInstalled` | emit path (counted) | — | record arrived before `init` or after shutdown | keep the guard alive for the process lifetime |
| `DropCause::LoggerPanicked` | emit path (counted) | — | a panic inside the emit guard, caught by `catch_unwind(AssertUnwindSafe(..))`: in sc-observability `try_log`, or in a `log` record's `Display`/`Debug` formatting (message arguments or a key-value) | a formatting panic: fix the value's `Display`/`Debug` impl; a `try_log` panic: report upstream and shut the guard down |
| `DropCause::ReentrantEmit` | emit path (counted) | — | a record was emitted on a thread whose emit guard is already active (`IN_EMIT` set): a formatter of the record being emitted, a panic hook, sink or redactor that logs through `log` or the macros | do not log from panic hooks, sinks or redactors that run inside the logger |
| `BoundedError::{TimedOut, Spawn { source: std::io::Error }, WorkerLost}` (crate-private) | `run_bounded` | — | mapped 1:1 into the `FlushError`/`ShutdownError` variants above | — |
| `ShutdownStep::{StillShared, FinalFlush { source: sc_observability_types::FlushError }}` (crate-private) | shutdown helper | — | `StillShared` → `ShutdownError::TimedOut`; `FinalFlush` → `ShutdownError::FinalFlush` | — |
| `LabelError::{Empty { kind: LabelKind }, ReservedPrefix { kind: LabelKind }, Rejected { kind: LabelKind, source: sc_observability_types::ValueValidationError }}` (`#[doc(hidden)]` via `__private`) | `target_label`, `action_label`, `field_key_label` | — | an empty label or key, a reserved-prefix field key, or a validation error after sanitizing | counted as `DropCause::InvalidEvent` by the caller; never returned to users |


## Failure reports

`InitError::report()`, `FlushError::report()`, `ShutdownError::report()` and
`SubmitError::report()` return a `FailureReport` (`src/report.rs`), the
serializable projection a binding or a frontend command channel receives:

| Field | Value |
|---|---|
| `schema_version` | `CONTROL_SCHEMA_VERSION` (1) |
| `failure` | `Failure`, internally tagged by `operation` (`init` / `flush` / `shutdown` / `submit`) and then by `kind` (`InitFailure`, `FlushFailure`, `ShutdownFailure`, or `SubmitError` itself, whose `InvalidInput` adds a `reason` tag). Timeouts carry `timeout_ms`. |
| `code` | the error's `code()` (a wrapped sc-observability error keeps its own code) |
| `message` | the `Display` text; informational only |
| `remediation` | the error's `remediation()` |

Example: `{"schema_version":1,"failure":{"operation":"flush","kind":"timed_out","timeout_ms":2000},"code":"SC_OBSERVABILITY_LOG_FLUSH_TIMED_OUT","message":"flush did not complete within 2s","remediation":{"kind":"recoverable","steps":["retry the flush later or raise the timeout","..."]}}`.
