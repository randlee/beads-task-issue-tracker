---
id: SC-OBS-B.P3-source
status: planned
qa_status: not_dispatched
merge_status: unmerged
branch: feature/sc-obs-B-P3-runtime-bridge
worktree: /Users/randlee/github/beads-task-issue-tracker-worktrees/feature/sc-obs-B-P3-runtime-bridge
target: develop
---

# B.P3 — Implement the reviewed sc-observability bridge contract in BTIT

This source task implements sc-observability B.P3 before mechanical B.1 copy.
It does not replace BTIT's separate crate-extraction Phase B plan.
Baseline BTIT commit: `8d8e82ae9f8501dbbf78c76df24d943918d7a6e4`.

## Authority and dependencies

The authoritative target and runtime behavior are the immutable
[target bridge contract](https://github.com/randlee/sc-observability/blob/84b32e9d6718418371ffd25a3de52346278725ca/docs/plans/phase-b/target-bridge-api.md),
[runtime contract](https://github.com/randlee/sc-observability/blob/84b32e9d6718418371ffd25a3de52346278725ca/docs/plans/phase-b/runtime-level-contract.md), and
[B.P3 sprint](https://github.com/randlee/sc-observability/blob/84b32e9d6718418371ffd25a3de52346278725ca/docs/plans/phase-b/sprint-b-p3-runtime-btit.md).
Their complete inventories and required fixtures govern this task; the list below
summarizes rather than narrows them.

Phase lead aobs accepts the target design at that revision for BTIT implementation,
based on [independent contract QA2 PASS](https://github.com/randlee/sc-observability/pull/111#issuecomment-5708115438).
QA later disclosed missing mandatory service-hardening coverage; source work
requires the registered `phase-b-bp3-service-addendum` result before starting.
The design decision is conditional on that coverage passing or a documented
reviewer scope-skip. This is not acceptance of BTIT source. Runtime public-API acceptance remains
owner-deferred to phase completion; publication remains deferred to B.7.

Consume the final immutable B.P2 staged 1.3.0 package set from the completed
`phase-b-bp2-qa1-corrections` handoff. Record its actual source, inventory, archive
hashes and platform qualification. Distinguish the published 1.2.0 baseline.
Use portable, verified archive inputs in local and CI builds; never rely on
ambient sibling-worktree patches, fabricated registry availability or live publishing.

## Deliverables and acceptance

1. Reconcile every observed public export, impl/derive and hidden macro-support
   family against the target matrix. Implement every target revision before copy,
   including BridgeEvent, direct admission/query/control operations, native typed
   errors and traits, health fields, and retained shutdown results. Update BTIT
   callsites affected by the intentional unpublished API changes.
2. Replace independent THRESHOLD state/helpers with shared core filtering and one
   LogGuard-owned LevelOwner. LogControl remains non-owning and cannot mutate.
   Preserve the conservative Trace facade ceiling; reject unsupported static
   levels before mutation and unsupported startup baselines before installation.
3. Implement the exact lifecycle, bounded flush, shutdown, wait_stopped, field-key,
   diagnostic, identity, panic/reentrancy and exactly-once accounting contracts.
   Failed/unconfirmed completion must never claim the core writer stopped.
4. Add every target error/Serde/trait/ownership fixture and the runtime transition,
   release-level, capped-build, concurrency and fault matrix. Use deterministic
   coordination for races. Keep macro grammar and external consumer coverage.
5. Retain reproducible source-side export/boundary checks, resolved Cargo features,
   and passing macOS/Linux/Windows bridge qualification at the final source SHA.
   Source builds must prove the exact staged packages were consumed.
6. Write `docs/plans/sc-observability-runtime/handoff-b-p3.md`: source SHA, target
   revision, staged package provenance, exact removed symbols, export dispositions,
   commands/results and raw evidence paths/hashes. Independent critical review
   and accepted re-review are required before sc-observability accepts source.

## Validation and completion

Run complete bridge/macros/API/UI consumer checks, affected application checks,
core-sharing integration, deterministic races/fault injection, release logging
and capped builds. Run formatting, clippy and the applicable repository gates.
Record exact commands and outcomes, including failed or pending checks.

Maintain a worktree-local checklist for all deliverables and contract fixtures:
first implement every item, then inspect every actual file and its evidence.
Fix and recheck failures before handoff. Lead completeness review precedes QA.

`status: complete` means source implementation complete only; keep `qa_status`
and `merge_status` separate. No B.1 destination copy, live publication, binding
implementation or invented source acceptance belongs in this task.

The installed bd tool currently refuses this legacy Dolt workspace. Preserve
`.beads` unchanged; record work through the registered ATM task and report this
tracking limitation rather than attempting an unrelated data migration.
