---
id: a-5
title: Phase A critical review — a-4 btit adoption
status: findings_open
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
panel rendering, and application shutdown behavior. The underlying bridge
crates are reviewed here only where their lifecycle contract is exercised by
the btit integration.

## Verdict

**Not ready for a-5 closure or a-6 handoff.** One Blocking and one Important
finding remain open. The primary defect is the `clear_logs` / application-exit
race: the implementation cannot provide its documented one-time, bounded
shutdown guarantee when the guard is shared.

| id | reported by | severity | file:line (at `f6f69dc`) | finding | disposition |
| --- | --- | --- | --- | --- | --- |
| R-A4-001 | Codex | Blocking | `src-tauri/src/logging.rs:140-159` | If `clear_logs` has cloned `Arc<LogGuard>`, `on_run_event` takes the static owner, fails `Arc::try_unwrap`, and calls only `flush`. Once the command's clone is dropped, `LogGuard::Drop` invokes the bridge's separate flush-and-shutdown sequence. Exit therefore has a second lifecycle operation, its result is discarded, and it is not bounded by the handler's claimed single `LOG_IO_TIMEOUT`. This violates a-4 AC7 and can leave shutdown timing/error handling dependent on the concurrently executing command. | open — replace the shared-`Arc` handoff with coordinated exclusive shutdown ownership (or change the bridge lifecycle API), then add a deterministic clear-during-exit test proving one shutdown attempt, bounded completion, and a stopped logger. |
| R-A4-002 | Codex | Important | `src-tauri/src/logging.rs:230-243` | `remove_rotated_logs` uses `entries.flatten()`, silently discarding `ReadDir` errors. `clear_logs` can consequently return `Ok(())` even though it failed to inspect one or more directory entries and left rotated JSONL data behind. That breaks the command's stated clear semantics and hides an I/O failure from the frontend. | open — iterate over `ReadDir` results explicitly and map each entry error into the command's existing `Err(String)` path; add a fault-injection or injectable-directory-reader test for the error case. |

## Reviewed evidence

- The a-4 diff is `d4967c9..f6f69dc`; `git diff --check` is clean.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml` passed: 142 tests.
- `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets` passed with pre-existing warnings outside the a-4 diff. A stricter `-D warnings` run remains red on those pre-existing files and is not attributed to a-4.
- Frontend unit tests and `vue-tsc` were not runnable in this checkout because `node_modules` is absent (`vitest: command not found`); no dependency installation was performed during this review.
- Filing Beads issues for the two findings is currently blocked: `bd create` reports that the repository has a legacy Dolt workspace requiring an explicit migration and instructs callers to preserve `.beads` unchanged. The findings remain tracked in this document until that migration is completed.

## Required re-review evidence

1. A focused test drives `clear_logs` and `RunEvent::Exit` concurrently against an installed bridge, asserts the logger is shut down exactly once, and proves the exit path returns within its documented bound.
2. A focused test proves a per-entry directory enumeration error causes `clear_logs` to return `Err`, rather than silently retaining a rotated log.
3. Re-run the a-4 Rust, frontend, type-check, and manual quit validations after both fixes. Record the app-log path and final JSONL records in the fix PR.
