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
findings were raised. R-A4-001..004 have fix commits and await re-review (see
the dispositions below and "Fix summary"); R-A4-005 is open. The primary integration defect is the `clear_logs` /
application-exit race: the implementation cannot provide its documented
one-time, bounded shutdown guarantee when the guard is shared. Independently,
the bridge's enabled `log` path can panic before its advertised containment and
reentrancy protection starts.

| id | reported by | severity | file:line (at `f6f69dc`) | finding | disposition |
| --- | --- | --- | --- | --- | --- |
| R-A4-001 | Codex | Blocking | `src-tauri/src/logging.rs:140-159` | If `clear_logs` has cloned `Arc<LogGuard>`, `on_run_event` takes the static owner, fails `Arc::try_unwrap`, and calls only `flush`. Once the command's clone is dropped, `LogGuard::Drop` invokes the bridge's separate flush-and-shutdown sequence. Exit therefore has a second lifecycle operation, its result is discarded, and it is not bounded by the handler's claimed single `LOG_IO_TIMEOUT`. This violates a-4 AC7 and can leave shutdown timing/error handling dependent on the concurrently executing command. | fixed (trailer `Review-Finding: R-A4-001`) — the static `LogLifecycle<LogGuard>` (`src-tauri/src/logging/lifecycle.rs`) is the single guard owner. `clear_logs` flushes through the new non-owning `LogHandle` and claims a clear-write slot; exit marks `ShuttingDown`, waits (bounded) only for a truncating clear, and makes the one final shutdown within 1x `LOG_IO_TIMEOUT` total; its `ExitOutcome` is reported. A clear after exit begins returns `ClearError::ShutdownStarted`. Bridge support: `LogHandle::flush`, `FlushError::ShutDown`. Test: `exit_during_a_held_clear_flush_shuts_down_once_and_stops_the_logger` plus five fake-guard lifecycle tests. |
| R-A4-002 | Codex | Important | `src-tauri/src/logging.rs:230-243` | `remove_rotated_logs` uses `entries.flatten()`, silently discarding `ReadDir` errors. `clear_logs` can consequently return `Ok(())` even though it failed to inspect one or more directory entries and left rotated JSONL data behind. That breaks the command's stated clear semantics and hides an I/O failure from the frontend. | fixed (trailer `Review-Finding: R-A4-002`) — `remove_rotated_logs` checks every entry result and maps an entry error to `ClearError::ReadDirEntry` in the command's `Err(String)`; the directory reader is injectable. Tests: `clear_log_files_reports_a_directory_entry_error`, `clear_log_files_reports_a_listing_error`. |
| R-A4-003 | Codex | Blocking | `crates/sc-observability-log/src/bridge.rs:26-43`, `crates/sc-observability-log/src/mapping.rs:197-234`, `crates/sc-observability-log/src/lib.rs:396-425` | `Bridge::log` obtains `BridgeOptions` and calls `record_to_parts` *before* `__private::emit` enters `handle::submit_guarded`. Formatting `record.args()` (`to_string`) and a non-primitive key-value (`value.to_string`) can call user `Display` code and panic. Such a panic unwinds out through `log!`, violating the documented guarantee that the emit path never panics. A formatter that itself logs also runs before `EmitScope` is active, so its nested record is not classified as `ReentrantEmit`. | fixed (trailer `Review-Finding: R-A4-003`) — `Bridge::log` enters `submit_guarded` once and runs the slot read, options lookup, `record_to_parts`, assembly, redaction and `try_log` inside it; `__private::emit` and the bridge share the unguarded cores `handle::submit_installed` / `handle::submit_to`. Test: `tests/bridge_guard.rs` (`bridge_guard_contains_user_formatting`: panicking message, panicking kv value, logging `Display`). |
| R-A4-004 | Codex | Important design gap | `crates/sc-observability-log/src/lib.rs:220-228`, `crates/sc-observability-log/src/handle.rs:64-68` | `LogGuard` exposes cumulative dropped-event counts but no read-only health/diagnostic snapshot from the underlying logger. A consumer can see `WriterDegraded` or a timeout count but cannot inspect writer state, last writer error, queue pressure, active file path, or sink health to determine remediation. This weakens the observability bridge precisely when it is degraded and complicates a common frontend/backend status interface. | fixed (trailer `Review-Finding: R-A4-004`) — `LogGuard::health()` / `LogHandle::health()` return the bridge-owned, serde-serializable `BridgeHealth` (schema version 1): lifecycle, state, writer state, queue depth/capacity/high-water mark, last writer error and last error with code and remediation, file-sink status and path, console-sink status, dropped-event counters; defined after shutdown. Tests: `health::tests::*`, `tests/health_snapshot.rs`, `tests/api_freeze.rs::a5_health_api_is_frozen`. |
| R-A4-005 | Codex | Blocking for crate acceptance | `crates/sc-observability-log/src/lib.rs:186-229`, `src-tauri/src/logging.rs:104-221` | The bridge has no settled public lifecycle/control boundary for consumers outside BTIT. The sole-owner `LogGuard` is forced into `Arc` sharing for clear/export work, while bindings would have no supported structured-submit, health, or read-only control API. If BTIT completes around the current shape, `sc-observability` would have to make a later public-API redesign to support its own frontend and Python consumers. | open — BTIT must implement and document the public contract below before its bridge design is accepted. This is a design gate for the crate, not a request to add Tauri/Python dependencies or frontend policy to BTIT. |

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

All four findings were fixed on `feature/sprint-a-5-sc-review` and await
re-review; each fix commit carries its `Review-Finding:` trailer.

- **Public API change (intentional, recorded in `crates/sc-observability-log/tests/api_freeze.rs`).**
  Added `LogGuard::health`, `LogGuard::handle`, `LogHandle` (`flush`, `health`),
  `BridgeHealth` and its component types, `BRIDGE_HEALTH_SCHEMA_VERSION`, the
  `Timestamp` re-export, `Serialize`/`Deserialize` on `DroppedEvents`, and
  `FlushError::ShutDown` with code `SC_OBSERVABILITY_LOG_FLUSH_AFTER_SHUTDOWN`.
  A flush through a live `LogGuard` cannot observe `ShutDown`.
- **Runtime dependency graph unchanged.** The workspace `serde` dependency gained
  the `derive` feature; `serde_derive` was already in the graph, so
  `crates/runtime-deps.txt` and both lockfiles are unchanged.
- **sprint-a-4 AC7 superseded.** The shared-`Arc` exit design is replaced by the
  single lifecycle owner described under R-A4-001.
