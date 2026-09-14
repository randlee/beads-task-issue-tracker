---
id: a-5
title: Phase A critical review — a-4 btit adoption
status: fixes_complete_pending_rereview
reviewed_branch: integrate/phase-a
reviewed_commit: f6f69dc
reviewed_range: d4967c9..f6f69dc
reviewed_at: 2026-09-13
reviewer: Codex
---

# a-5 critical review record

## Scope

This review covers the a-4 adoption merged by PR #48: replacement of
`tauri-plugin-log` with `sc-observability-log`, JSONL command handling, debug
panel rendering, and application shutdown behavior. It also covers the
`sc-observability-log` bridge contract exercised by that adoption: lifecycle,
non-panicking/non-reentrant emission, error reporting, and consumer-operable
health data. The review is therefore a release gate for both BTIT a-5 and a
future migration of the bridge into `sc-observability`.

## Verdict

**Not ready for a-5 closure or a-6 handoff.** Three Blocking and two Important
findings were raised. All five (R-A4-001..005) have fix commits and await
re-review (see the dispositions below and "Fix summary"). The primary integration defect is the `clear_logs` /
application-exit race: the implementation cannot provide its documented
one-time, bounded shutdown guarantee when the guard is shared. Independently,
the bridge's enabled `log` path can panic before its advertised containment and
reentrancy protection starts.

| id | reported by | severity | file:line (at `f6f69dc`) | finding | disposition |
| --- | --- | --- | --- | --- | --- |
| R-A4-001 | Codex | Blocking | `src-tauri/src/logging.rs:140-159` | If `clear_logs` has cloned `Arc<LogGuard>`, `on_run_event` takes the static owner, fails `Arc::try_unwrap`, and calls only `flush`. Once the command's clone is dropped, `LogGuard::Drop` invokes the bridge's separate flush-and-shutdown sequence. Exit therefore has a second lifecycle operation, its result is discarded, and it is not bounded by the handler's claimed single `LOG_IO_TIMEOUT`. This violates a-4 AC7 and can leave shutdown timing/error handling dependent on the concurrently executing command. | fixed (trailer `Review-Finding: R-A4-001`) — the static `LogLifecycle<LogGuard>` (`src-tauri/src/logging/lifecycle.rs`) is the single guard owner. `clear_logs` flushes through the non-owning `LogControl` (round 1 `LogHandle`, renamed by R-A4-005) and claims a clear-write slot (wait bounded by `LOG_IO_TIMEOUT` since QA-1 RSH-002); exit marks `ShuttingDown`, waits (bounded) only for a truncating clear, and makes the one final shutdown within 1x `LOG_IO_TIMEOUT` total; its `ExitOutcome` is reported. A clear after exit begins returns `ClearError::ShutdownStarted`. Bridge support: `LogControl::flush`, `FlushError::ShutDown`. QA-1 RSH-001 adds the degraded `ExitOutcome::ShutDownWhileClearWriting` when a clear is still writing at the exit deadline. Test: `exit_during_a_held_clear_flush_shuts_down_once_and_stops_the_logger` plus five fake-guard lifecycle tests. |
| R-A4-002 | Codex | Important | `src-tauri/src/logging.rs:230-243` | `remove_rotated_logs` uses `entries.flatten()`, silently discarding `ReadDir` errors. `clear_logs` can consequently return `Ok(())` even though it failed to inspect one or more directory entries and left rotated JSONL data behind. That breaks the command's stated clear semantics and hides an I/O failure from the frontend. | fixed (trailer `Review-Finding: R-A4-002`) — `remove_rotated_logs` checks every entry result and maps an entry error to `ClearError::ReadDirEntry` in the command's `Err(String)`; the directory reader is injectable. Tests: `clear_log_files_reports_a_directory_entry_error`, `clear_log_files_reports_a_listing_error`. |
| R-A4-003 | Codex | Blocking | `crates/sc-observability-log/src/bridge.rs:26-43`, `crates/sc-observability-log/src/mapping.rs:197-234`, `crates/sc-observability-log/src/lib.rs:396-425` | `Bridge::log` obtains `BridgeOptions` and calls `record_to_parts` *before* `__private::emit` enters `handle::submit_guarded`. Formatting `record.args()` (`to_string`) and a non-primitive key-value (`value.to_string`) can call user `Display` code and panic. Such a panic unwinds out through `log!`, violating the documented guarantee that the emit path never panics. A formatter that itself logs also runs before `EmitScope` is active, so its nested record is not classified as `ReentrantEmit`. | fixed (trailer `Review-Finding: R-A4-003`) — `Bridge::log` enters `submit_guarded` once and runs the slot read, options lookup, `record_to_parts`, assembly, redaction and `try_log` inside it; `__private::emit` and the bridge share the unguarded cores `handle::submit_installed` / `handle::submit_to`. Test: `tests/bridge_guard.rs` (`bridge_guard_contains_user_formatting`: panicking message, panicking kv value, logging `Display`). |
| R-A4-004 | Codex | Important design gap | `crates/sc-observability-log/src/lib.rs:220-228`, `crates/sc-observability-log/src/handle.rs:64-68` | `LogGuard` exposes cumulative dropped-event counts but no read-only health/diagnostic snapshot from the underlying logger. A consumer can see `WriterDegraded` or a timeout count but cannot inspect writer state, last writer error, queue pressure, active file path, or sink health to determine remediation. This weakens the observability bridge precisely when it is degraded and complicates a common frontend/backend status interface. | fixed (trailer `Review-Finding: R-A4-004`) — `LogGuard::health()` / `LogControl::health()` return the bridge-owned, serde-serializable `BridgeHealthReport` (schema version 1; round 1 `BridgeHealth`, renamed by R-A4-005): lifecycle, state, writer state, queue depth/capacity/high-water mark, last writer error and last error with code and remediation, file-sink status and path, console-sink status, dropped-event counters; defined after shutdown. Tests: `health::tests::*`, `tests/health_snapshot.rs`, `tests/api_freeze.rs::a5_health_api_is_frozen`. |
| R-A4-005 | Codex | Blocking for crate acceptance | `crates/sc-observability-log/src/lib.rs:186-229`, `src-tauri/src/logging.rs:104-221` | The bridge has no settled public lifecycle/control boundary for consumers outside BTIT. The sole-owner `LogGuard` is forced into `Arc` sharing for clear/export work, while bindings would have no supported structured-submit, health, or read-only control API. If BTIT completes around the current shape, `sc-observability` would have to make a later public-API redesign to support its own frontend and Python consumers. | fixed (trailer `Review-Finding: R-A4-005`) — contract: non-`Clone` `LogGuard` is the sole owner (`shutdown` once, `Drop` fallback); `LogGuard::control()` returns `Clone + Send + Sync` `LogControl` (`flush`, `health`, `active_log_path() -> Option<PathBuf>`, `submit`) with no shutdown, no `LogGuard` conversion and no public constructor; `LogControl::submit(StructuredRecord) -> Result<SubmitOutcome, SubmitError>` runs the same `submit_guarded` core and `Logger` as the facade and macros, which discard the result only after it counted exactly one `DropCause`; the bridge owns version, timestamp, service, identity, trace, redaction and routing (`StructuredRecord` is `deny_unknown_fields`); `SubmitError` distinguishes `queue_full`, `invalid_input(reason)`, `stopped{lifecycle}`, `reentrant`, `writer_degraded`, `backend_shutdown_timed_out`, `contained_panic` with `SC_OBSERVABILITY_LOG_SUBMIT_*` codes, remediation and `drop_cause()`; `BridgeHealthReport` (schema 1) and `FailureReport` (`report()` on every error, `CONTROL_SCHEMA_VERSION` 1) are tagged serde data; `BridgeLifecycle::ShutdownTimedOut` separates a timed-out shutdown from the final `Stopped`, which the detached helper publishes on late completion; unified field-key rules (facade `kv` reserved/empty keys omitted and counted, `code.*` authoritative with `shadowed_fields`) in `docs/mapping.md` "Field keys". Evidence: (1) `tests/api_freeze.rs::a5_control_api_is_frozen` + trybuild `tests/ui/log_guard_not_clone.rs`, `log_control_not_owner.rs`, `log_control_not_constructible.rs`; (2) `tests/one_writer.rs::facade_macro_and_submit_share_one_core_and_one_writer`; (3) `sc-observability-log-consumer-check` `status::read_status` + `tests/control_consumer.rs::consumer_reads_health_and_flushes_through_control_only`; (4) `tests/reinstall_subprocess.rs::bridge_cannot_be_reinstalled_or_replaced_after_shutdown` (child process) and `tests/shutdown_timeout.rs::timed_out_shutdown_completes_late_and_is_observable`. No Tauri/Python dependency or frontend policy was added. |

## Required changes and acceptance tests

### R-A4-001 — serialize lifecycle ownership

The application must nominate one lifecycle owner for the non-cloneable
`LogGuard`. A clear operation may request a bounded flush, but it must not
retain an `Arc<LogGuard>` that can outlive or race the exit owner. The selected
design must serialize `clear_logs`, exit, and final shutdown, reject or
no-op clearly once shutdown begins, and surface a final-shutdown error where a
caller can record it. `Drop` remains a fallback, not an unobserved second
lifecycle path.

Required evidence:

1. A deterministic test starts `clear_logs`, delivers `RunEvent::Exit` while
   the clear-side flush is held, and proves exactly one final shutdown is
   requested.
2. The exit handler returns within `LOG_IO_TIMEOUT` (including contention),
   the bridge has stopped accepting records, and the final outcome is observed
   rather than discarded.
3. A follow-up record is absent from the JSONL output after shutdown.

### R-A4-002 — preserve clear failure semantics

Replace `ReadDir::flatten` with explicit handling of every entry result. A
directory enumeration failure must produce the existing command error instead
of a success response. Test the entry-error path with an injectable reader or
an equivalent deterministic fault-injection seam, and verify that the frontend
receives a failed clear rather than falsely reporting success.

### R-A4-003 — guard all user-controlled formatting

The guard must become the outermost boundary for bridge emission. It must
cover target/action validation, `fmt::Arguments` rendering, key-value
conversion, event construction, redaction, and `Logger::try_log`. It must map
a caught panic to exactly one `DropCause::LoggerPanicked`; a nested `log!`
issued by a formatter must map to exactly one `DropCause::ReentrantEmit` while
allowing the outer record to complete when its formatter returns normally.

Required tests, with an enabled log level:

1. `log::info!` with a custom `Display` implementation that panics: a caller's
   `catch_unwind` sees no panic and `LoggerPanicked` increases by one.
2. A `log` key-value value whose formatting panics: the same no-unwind and
   exactly-one-counter behavior holds.
3. A custom `Display` implementation that invokes `log!`: the nested record is
   not written and `ReentrantEmit` increases by one; the outer record is
   written once.

### R-A4-004 — make degradation actionable

Add a `LogGuard` read-only health method returning a stable bridge-owned
snapshot rather than the mutable underlying `Logger`. At minimum, include
writer state, queue depth/high-water mark where available, last writer error
with stable diagnostic code/remediation, file-sink availability and active
path, and the existing dropped-event counters. Define the result after
shutdown, and ensure it can be represented without lossy string parsing by
future TypeScript and Python bindings.

### R-A4-005 — lock the transferable public bridge contract

BTIT is building the crate that `sc-observability` will copy; its public API
must therefore be correct before BTIT declares the initial design complete.
The resulting crate must have these boundaries:

1. `LogGuard` is non-cloneable and is the sole lifecycle owner. It retains
   explicit final shutdown; no clone, control handle, or binding can shut the
   logger down implicitly.
2. `LogGuard::control()` returns a cloneable `LogControl` (final naming is a
   design choice). `LogControl` exposes bounded flush, an owned active-path
   value, and the serializable bridge-health snapshot. It cannot expose a
   mutable `Logger`, acquire final ownership, or trigger a second shutdown.
3. `LogControl` exposes a nonblocking structured-submit result against the
   *same* installed writer. Its final request type must let the bridge—not a
   frontend—own envelope version, timestamp, service identity, redaction, and
   writer routing. Its result/error types must distinguish accepted, filtered
   when applicable, queue-full, invalid input, stopped lifecycle, reentrancy,
   and contained backend/formatter failure with stable codes and remediation.
4. Facade `log::Log` and macro APIs retain their compatibility unit-return
   behavior. They call the same guarded submission core and may deliberately
   discard its result only after exactly-once drop accounting. They must never
   create a second logger or a second writer path.
5. `BridgeHealthReport`, control results, and errors are serializable,
   versioned Rust data contracts. They contain tagged discriminants and stable
   diagnostic code/remediation fields; they do not require a future consumer
   to parse display strings or hold Rust ownership handles.
6. The contract explicitly documents install-once process-global behavior,
   timeout versus final-stopped semantics, identity ownership, and unified
   field-key/collision rules.

Required design evidence:

1. Public rustdoc and compile-time API checks pin each exported signature and
   demonstrate that neither `LogGuard` nor `LogControl` can be cloned into a
   second shutdown owner.
2. An integration test proves facade, macro, and direct structured submission
   all admit to one writer and maintain exactly-once drop accounting.
3. A consumer fixture accesses health and bounded flush through `LogControl`
   without importing private support modules or a mutable logger.
4. A subprocess fixture proves the installed bridge cannot be replaced after
   shutdown, and a timeout fixture makes late completion observable through
   lifecycle/health state.

## Reviewed evidence

- The a-4 diff is `d4967c9..f6f69dc`; `git diff --check` is clean.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml` passed: 142 tests.
- `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets` passed with pre-existing warnings outside the a-4 diff. A stricter `-D warnings` run remains red on those pre-existing files and is not attributed to a-4.
- Frontend unit tests and `vue-tsc` were not runnable in this checkout because `node_modules` is absent (`vitest: command not found`); no dependency installation was performed during this review.
- Filing Beads issues for the two findings is currently blocked: `bd create` reports that the repository has a legacy Dolt workspace requiring an explicit migration and instructs callers to preserve `.beads` unchanged. The findings remain tracked in this document until that migration is completed.

## Required re-review evidence

1. Close R-A4-001 through R-A4-005 and attach the focused tests specified above.
2. Re-run the a-4 Rust, frontend, type-check, and manual quit validations after the fixes. Record the app-log path, final JSONL records, health snapshot, and shutdown result in the fix PR.
3. Obtain an independent `sc-observability` design review before the bridge is migrated or exposed through generated language bindings.

## Fix summary

All five findings were fixed on `feature/sprint-a-5-sc-review` and await
re-review; each fix commit carries its `Review-Finding:` trailer.

- **Public API change (intentional, recorded in `crates/sc-observability-log/tests/api_freeze.rs`).**
  Round 1 (R-A4-001/004): `LogGuard::health`, `BRIDGE_HEALTH_SCHEMA_VERSION`,
  the `Timestamp` re-export, `Serialize`/`Deserialize` on `DroppedEvents`, and
  `FlushError::ShutDown` with code `SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN`.
  A flush through a live `LogGuard` cannot observe `ShutDown`.
  Round 2 (R-A4-005): the round-1 `LogGuard::handle` / `LogHandle` are replaced
  by `LogGuard::control` / `LogControl` (`flush`, `health`, `active_log_path`,
  `submit`); `BridgeHealth` is renamed `BridgeHealthReport`;
  `BridgeLifecycle::ShutdownTimedOut`; new `StructuredRecord`, `SubmitOutcome`,
  `SubmitError`, `InvalidInputReason`, `JsonMap`, `JsonValue`, `FailureReport`,
  `Failure`, `InitFailure`, `FlushFailure`, `ShutdownFailure`,
  `CONTROL_SCHEMA_VERSION`, `report()` on `InitError`/`FlushError`/
  `ShutdownError`/`SubmitError`, `Serialize`/`Deserialize` on `Level`, and seven
  `SC_OBSERVABILITY_LOG_SUBMIT_*` codes. The name `FailureReport` (not
  `ErrorReport`) keeps the sprint's `pub struct *Error*` wrapper-struct gate green.
- **Behavior changes in round 2.** A timed-out shutdown now leaves the detached
  helper waiting for sole ownership without its own deadline and publishes
  `Stopped` on late completion (previously the lifecycle became `Stopped` at
  once and the helper gave up at the deadline). `log` facade key-values with an
  empty or reserved key are omitted and counted `InvalidEvent`, and a `kv` value
  displaced by `code.module`/`code.file`/`code.line` moves to
  `sc_observability_log.shadowed_fields` (previously overwritten silently).
- **Runtime dependency graph unchanged.** The workspace `serde` dependency gained
  the `derive` feature; `serde_derive` was already in the graph, so
  `crates/runtime-deps.txt` and both lockfiles are unchanged.
- **sprint-a-4 AC7 superseded.** The shared-`Arc` exit design is replaced by the
  single lifecycle owner described under R-A4-001.

## QA-1 findings on the round-1 fixes (`dbedd1a`)

Verified by the coordinator and fixed in round 2 (commit titles `fix(a-5): QA-1 …`).

| id | reviewer | severity | finding | disposition |
| --- | --- | --- | --- | --- |
| RBP-F001 | rust-best-practices-agent | Important | `src-tauri` `ClearError` had no stable `code()` / `remediation()`, and `clear_logs` returned an uncoded string. | fixed — `BTIT_LOG_CLEAR_*` codes and a remediation per variant (including the new `WriteSlotTimedOut` and `TaskFailed`); the command's `Err(String)` is `"<CODE>: <message>"` and the failure is logged with code and remediation. `app/` callers (`DebugPanel.vue`, `DebugDialog.vue`) only catch and log, so they are unaffected. Test: `clear_error_codes_are_stable_and_remediations_non_empty`. |
| RSH-001 | rust-service-hardening-agent | Important | Once the exit deadline passed with a clear still writing, exit wrote the at-exit records and shut down while the clear could still truncate the same file. | fixed — exit stays bounded, skips the at-exit records and returns `ExitOutcome::ShutDownWhileClearWriting`; `report_exit` writes `BTIT_LOG_EXIT_CLEAR_STILL_WRITING` to stderr. Residual risk (the clear may truncate after the final flush, losing the last records) is documented in `src-tauri/src/logging/lifecycle.rs`. Tests: `exit_reports_a_degraded_shutdown_when_a_clear_is_still_truncating_at_its_deadline`, `exit_waits_for_a_truncating_clear_that_finishes_within_its_budget`, `exit_report_names_the_degraded_clear_case`. |
| RSH-002 | rust-service-hardening-agent | Minor | `claim_clear_write` waited on the condvar without a timeout, so a second clear could hang behind stuck file I/O. | fixed — `wait_timeout` against the clear's `LOG_IO_TIMEOUT`, returning `ClearError::WriteSlotTimedOut` (`BTIT_LOG_CLEAR_WRITE_SLOT_TIMED_OUT`). Test: `clear_times_out_waiting_for_a_held_write_slot`. |
| RSH-003 | rust-service-hardening-agent | — | `log_frontend` message size cap. | deferred to issue #49 under the accepted no-size-cap policy. |
| ATM-QA-002 | req-qa | Minor | The always-on `[logging] health at exit` record was undocumented. | fixed — documented below and on `OwnedGuard::report_before_shutdown` in `src-tauri/src/logging/lifecycle.rs`; the real-bridge lifecycle test pins its shape (`assert_health_at_exit_record`). |

### The `[logging] health at exit` record (ATM-QA-002)

Written by BTIT's lifecycle owner immediately before the one final shutdown on
`RunEvent::Exit` (every exit that owns the guard, except the degraded
`ShutDownWhileClearWriting` case). It uses the plain `log` macros, so it is
written even when the debug panel's `LOGGING_ENABLED` toggle is off.

- **Optional warn record** when any event was dropped: level `Warn`, action
  `logging`, message `dropped events: <Cause>=<n>, …` listing only causes with a
  non-zero count (`DropCause` names, for example `QueueFull=3`).
- **Health record**: level `Info`, target `app_lib.logging.lifecycle`, action
  `logging`, message `health at exit`, and `fields.health` containing the
  `BridgeHealthReport` serialized as a JSON **string** (`schema_version` 1;
  consumers parse it a second time). The snapshot is taken while the bridge is
  still running (`lifecycle: "running"`), so it carries the session's final queue
  high-water mark, writer and sink errors, and drop counters. If serialization
  fails, a `Warn` record `health at exit could not be serialized: <error>` is
  written instead.
- **Purpose**: post-mortem evidence in the JSONL file (and the debug panel) of
  whether the session lost records or ran degraded, since no status channel
  remains after shutdown.
