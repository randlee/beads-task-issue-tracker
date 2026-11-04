# B.P3 source re-review and lead acceptance

Reviewed commit: 396a9d9f77ca1950eeb92d4f88c0eecadb5ef00b
Verdict: accepted

Phase lead aobs accepts this exact BTIT source for the B.1 mechanical copy, based on the independent QA2 PASS below and the lead verification of the full export inventory, target disposition, staged package identity, and retained execution evidence. Production files match implementation `51fb22c6873b12c541b20b7f909110abb457c240`.

Target: `randlee/sc-observability`, `docs/plans/phase-b/target-bridge-api.md` at `84b32e9d6718418371ffd25a3de52346278725ca`. Runtime public-API acceptance remains owner-deferred; publication remains deferred to B.7. This record authorizes mechanical source import, not merge or publication. The documented shutdown residual risk remains unchanged.

Recorded at: 2026-09-17T07:15:27.371703+00:00

The following is the unmodified quality-mgr report from [PR90 QA2](https://github.com/randlee/beads-task-issue-tracker/pull/90#issuecomment-5710484169). Its statements that source acceptance was pending describe the state when QA reported; the lead acceptance above is the subsequent decision.

---

# Final Quality Report

Generated: 2026-09-17T07:10:34Z
QA Pass: 2
Sprint/Task: SC-OBS-B.P3-source / phase-b-bp3-source-qa-2
Branch: `fix/sc-obs-B-P3-source-qa1`
Commit: `396a9d9f77ca1950eeb92d4f88c0eecadb5ef00b`
PR: #90
Final Verdict: **PASS**

## Machine Status (JSON)
```json
{
  "sprint": "SC-OBS-B.P3-source",
  "task": "phase-b-bp3-source-qa-2",
  "branch": "fix/sc-obs-B-P3-source-qa1",
  "commit": "396a9d9f77ca1950eeb92d4f88c0eecadb5ef00b",
  "pr": 90,
  "verdict": "PASS",
  "findings": {
    "blocking": 0,
    "important": 0,
    "minor": 0
  },
  "blocking_ids": [],
  "merge_readiness": "ready",
  "merge_reason": "All 10 carried-forward QA1 findings independently confirmed fixed by all 3 required reviewers (req-qa, arch-qa boundary check, rust-qa-agent execution) plus direct quality-mgr re-verification; deliverable completion 16/16 (100%); zero unresolved findings in round scope; PR#90 is active with 13/13 CI checks green.",
  "next_action": "none",
  "owner": "none",
  "recommendation": "Merge PR#90 pending human review sign-off. This QA2 round resolves fix-correctness for the 10 carried-forward findings only; it does not resolve or grant sc-observability's own B.1/publication source-acceptance gate, nor the owner-deferred runtime-level-contract.md public API status -- both remain pending and must be handled by their respective owners."
}
```

## Validated Scope
### Reviewers dispatched (scoped, per QA2 fix-verification protocol)
- **req-qa**: full re-check of all 10 carried-forward IDs against `docs/plans/sc-observability-runtime/b-p3.md`, `handoff-b-p3.md`, `bp3-implementation-inventory.md`, and the authoritative external contract (`target-bridge-api.md`@84b32e9). Result: PASS, 16/16 deliverables (100%), zero findings.
- **arch-qa boundary review** (routed through a general-purpose agent per the established cross-repo pattern, since the built-in `arch-qa` persona is hard-wired to sc-observability's own RULE-001..008 and correctly refuses foreign-repo charters): re-checked the compile-fail mutation-authority fixture, the redesigned `ShutdownState::Complete(Result<...>)`/`wait_error_from_snapshot` construction confinement, crate dependency-direction/coupling, and `WaitError::Unavailable` public-API presence. Result: boundary intact on all 4 items, no regressions.
- **rust-qa-agent**: real execution of the retained CI command list (fmt, clippy -D warnings, full test suite, release runtime_level_bridge test, static_level_cap_test, test_hooks fixtures, MSRV, rustdoc -D missing-docs, cargo-tree diff, test-isolation loop) plus 3x back-to-back re-runs of `shutdown_snapshot_unavailable`, `shutdown_timeout`, and `reinstall_subprocess` for residual flakiness, plus independent inventory regeneration diff. Result: all 10 IDs fixed with execution evidence, zero regressions.

### Independent re-verification performed directly (quality-mgr, not just trusting reviewer citations)
- Recomputed `ubuntu-crates.log`'s SHA-256 directly: `a695f5e4b66d8927e83d5b7de0769fbcb792a66562fac9927baa154ff9f1ceec`, matches `handoff-b-p3.md`'s documented hash exactly (closing req-qa's disclosed tooling-limitation gap).
- Ran `git diff --stat 51fb22c6..396a9d9f -- crates/` directly: empty, confirming `crates/` is byte-identical between `implementation_sha` and the review head (closing req-qa's other disclosed gap).
- Spot-verified arch-qa's four boundary claims directly: empty `Cargo.toml`/`Cargo.lock` diff between QA1 head and review head; `WaitError` enum shape and crate-root re-export; `wait_error_from_snapshot` confirmed private (non-`pub`).
- Confirmed PR#90 CI is 13/13 SUCCESS (`gh pr checks 90`) and re-confirmed stack88's structure via `gh api repos/randlee/beads-task-issue-tracker/stacks/88` (never ran `gh stack checkout` against the frozen worktree, per this task's own explicit "never checkout/rewrite frozen worktrees" instruction, which takes precedence over step j's generic fallback text).
- Personally read and traced every one of the 10 fix commits (`crates/sc-observability-log/src/handle.rs`, `health.rs`, `error.rs`, `crates/btit-app/src/logging.rs`, all new/changed test files) before any reviewer was dispatched, and cross-checked every reviewer's file:line citations against that independent reading.

### Per-ID disposition (all 10 carried-forward findings)
| ID | Severity (QA1) | Disposition | Evidence |
|---|---|---|---|
| ATM-QA-001 | blocking | **fixed** | `tests/ui/log_control_cannot_elevate_or_reset.rs`+`.stderr`, genuine E0599 trybuild compile-fail, wired into `tests/ui.rs`'s existing glob, executed in CI |
| ATM-QA-002 | important | **fixed** | same fixture confirmed as the actual required proof, not a substitute |
| RSH-001 | important | **fixed** (documented residual risk, no code change) | `handoff-b-p3.md` "Handoff boundary"/"Shutdown residual risk" section; `take_sole` (handle.rs) confirmed unchanged, still no deadline |
| RSH-002 | important | **fixed** | `bounded_frontend_log_input` (btit-app/src/logging.rs) bounds both level and message, UTF-8 char-boundary safe, truncation marker appended, unit-tested |
| FTQ-001 | important | **fixed** | `shutdown_timeout.rs` now uses `control.wait_stopped(...)`; sleep-poll loop fully removed |
| FTQ-002 | important | **fixed** | same test's `recv_timeout`/bounded channel joins replace bare `recv()`/`join().unwrap()` |
| RBQA-F004 | minor | **fixed** | `handoff-b-p3.md` documents the stray evidence log with hash and explicit historical/superseded disposition; hash independently recomputed and matches |
| ATM-QA-003 | minor | **fixed** | `FlushError` variant order now matches `target-bridge-api.md`'s declared order |
| RBP-F001 | important (corrected scope) | **fixed** | `save_shutdown`/`wait_error_from_snapshot` retain a terminal `WaitError::Unavailable`, notify all waiters, `wait_stopped` returns the identical retained diagnostic on repeat calls; `WaitError::Unavailable` remains in the public API; deterministic `test_hooks`-gated test exercises both prompt notification and diagnostic equality, executed 3x with no flakiness |
| FTQ-003 | minor | **fixed** | `reinstall_subprocess.rs` now drains child stdout concurrently via a dedicated reader thread |

**Deliverable completion: 16/16 (100%)**, reconciled against the original QA1 checklist. Both previously partial items (D4/AC3 compile-fail fixture; evidence-hash verification) are now fully closed.

## Findings Summary (Final)
- Blocking: 0
- Important: 0
- Minor: 0

## Residual Risks
- **RSH-001 (accepted, unchanged from QA1's permitted disposition)**: a permanently-blocked arbitrary user callback/I/O operation holding the sole-ownership reference can still leave `LogGuard::shutdown`'s wait blocked forever; the bridge correctly never invents a terminal Stopped/Failed result in that case. Bounded-caller contract (`wait_stopped` timeout, honest `Stopping` lifecycle) is unchanged and independently confirmed unchanged in code.
- **Debt note (non-blocking, pre-existing, out of this round's target scope)**: rust-qa-agent's real-execution review incidentally found a hardcoded `/tmp/native-health.jsonl` literal in `crates/sc-observability-log/src/health.rs` (lines 194, 208), a serde round-trip test fixture value that is never touched on the filesystem. Independently confirmed via `git diff` that this literal already existed at the QA1 head (`0df6123c`), unchanged by any commit in this fix round -- it predates and is unrelated to all 10 carried-forward findings. Not counted against this round's merge gate; recorded here as a debt note for a future round.
- **Standing, unresolved by this or any QA round**: `docs/plans/sc-observability-runtime/b-p3.md`'s `source_acceptance: pending_independent_qa` remains pending -- this QA2 round verifies fix-correctness only and does not itself grant sc-observability's own separate B.1/publication source-acceptance. `docs/plans/phase-b/runtime-level-contract.md`'s owner-deferred runtime public-API status also remains untouched and pending; no QA round may resolve or infer acceptance of it.

## Merge Readiness
- Status: **ready**
- Reason: All 10 carried-forward QA1 findings independently confirmed fixed by all 3 required reviewers (req-qa, arch-qa boundary check, rust-qa-agent execution) plus direct quality-mgr re-verification; deliverable completion 16/16 (100%); zero unresolved findings in round scope; PR#90 is active with 13/13 CI checks green.

## Recommendation
Merge PR#90 pending human review sign-off. This QA2 round resolves fix-correctness for the 10 carried-forward findings only; it does not resolve or grant sc-observability's own B.1/publication source-acceptance gate, nor the owner-deferred runtime-level-contract.md public API status -- both remain pending and must be handled by their respective owners.

