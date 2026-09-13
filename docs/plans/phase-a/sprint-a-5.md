---
id: a-5
title: sc-observability team critical review (btit-side closure)
status: planned
branch: feature/sprint-a-5-sc-review
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-a-5-sc-review
target: develop
recommended_model: higher-effort (review triage, API-level fixes across both crates)
dependency_relations:
  - prerequisite: a-3
    dependent: a-5
    relation: must_follow
    rationale: "the review covers the complete crate API (bridge, event macros, #[instrument]); stack parent"
  - prerequisite: a-4
    dependent: a-5
    relation: must_follow
    rationale: "the review covers the crates as adopted by a real consumer; a-4 PR merged and the stack rebased onto develop before a-5 development starts"
  - prerequisite: a-5
    dependent: a-6
    relation: must_follow
    rationale: "a-6 copies the reviewed and fixed crates and implements the recorded type-placement decision"
---

# Sprint a-5 — sc-observability team critical review (btit-side closure)

## Recommended Agent / Model

Recommended model: higher-effort (review triage, API-level fixes across both crates).

Recommended agent: not set. The btit developer pool is pending the `arch-ctm` decision on PR #38.

Planning advice; team-lead assigns from the active pool.

## Goal

- Get a critical review of `sc-observability-log` and `sc-observability-log-macros` from the sc-observability team.
- Close every Blocking and Important finding inside btit `crates/`.
- Agree with the team on which items belong in the shared low-level `sc-observability-types` crate and which stay in the new crates, and record that decision. After a-6, no type may be defined twice.
- Every closure gate in this sprint is a btit artifact. None depends on merging a change in another repository.

## Hard Dependencies

- a-3 is pushed (it is the stack parent). The a-4 PR is merged to `develop`, and the `phase-a-core` layers have been rebased onto that `develop` before development starts.
- The sc-observability team reviews over ATM, per `../sc-observability/docs/team-protocol.md`, using its review agents in `../sc-observability/.claude/agents/`: `rust-architect`, `rust-code-reviewer` and `arch-qa`.

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA;
merge parent → child before every dev/fix round. PR-completion trigger: parent
PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- a-3 → a-5 — `must_follow` (a-5 follows a-3): the review covers the complete crate API; stack parent.
- a-4 → a-5 — `must_follow` (a-5 follows a-4): the review covers the crates as adopted by btit. The a-4 PR merges, and the stack is rebased onto `develop`, before a-5 development starts.
- a-5 → a-6 — `must_follow` (a-6 follows a-5): a-6 copies the reviewed, fixed crates.

Stack: `phase-a-core` · layer 4.

## Exact Targets

- `crates/sc-observability-log/**`, `crates/sc-observability-log-macros/**` (review fixes only)
- `docs/plans/phase-a/review-a-5.md` (new; findings, dispositions, type-placement decision)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

1. **Review request.** Sent to the sc-observability team for the reviewed commit, which is the a-5 branch head after rebasing onto `develop`. It covers the crate paths, `docs/mapping.md`, `docs/compatibility.md`, and btit's adoption diff from a-4.
2. **`review-a-5.md`.** Lists every finding with id, reviewer, severity (Blocking / Important / Minor), file:line, disposition (`fixed <commit>` or `rejected — reviewer agreed <message ref>`), and reviewed commit.
3. **Fixes.** Every Blocking and Important finding is fixed in `crates/`, and the full `crates` gate set is re-run after the last fix. Each Minor finding is either fixed, or recorded in `review-a-5.md` as a follow-up for a-6 or the sc-observability backlog.
4. **Type-placement decision.** `review-a-5.md` records the team's decision and target crate (`sc-observability-types` or the new crates) for each of these item groups:
   - tracing-style `Level` constants
   - `TraceId`/`SpanId` generation
   - `TargetCategory`/`ActionName` sanitizers
   - `LevelFilter` conversions
5. **Re-review confirmation.** The sc-observability team confirms that no Blocking or Important finding remains open, and the message reference is recorded in `review-a-5.md`.

## Required Work

- If a fix needs a runtime dependency change, record it in `review-a-5.md` and regenerate `crates/runtime-deps.txt` in the same commit. That is allowed here because a-4 has already merged and no `parallel_safe` sprint is open.
- Re-run the btit gates, because btit consumes the crates through a path dependency after a-4.

## Explicit Code Samples

```markdown
<!-- docs/plans/phase-a/review-a-5.md -->
# a-5 review record
reviewed_commit: <sha>
reviewers: rust-architect, rust-code-reviewer, arch-qa (sc-observability team)

| id | reviewer | severity | file:line | finding | disposition |
|----|----------|----------|-----------|---------|-------------|
| R-001 | rust-code-reviewer | Important | crates/sc-observability-log/src/bridge.rs:42 | ... | fixed abc1234 |

## Type placement
| item | decision | target crate | decided by (message ref) |
|------|----------|--------------|--------------------------|
| Level constants | ... | sc-observability-types / sc-observability-log | ... |

## Re-review
confirmation: <message ref>   open_blocking: 0   open_important: 0
```

## This Sprint Does Not Close

- Copying the crates into `../sc-observability`, including its CI gates, publish manifest and PR (a-6).
- Moving any type into `sc-observability-types` (a-6, in `../sc-observability`).
- Publishing to crates.io.

## Acceptance Criteria

1. `review-a-5.md` exists, with the reviewed commit, every finding in the table format above, and `open_blocking: 0`, `open_important: 0`.
2. Every `fixed` disposition references a commit on `feature/sprint-a-5-sc-review` that exists in the branch history.
3. The type-placement table has a decision for all four item groups.
4. The re-review confirmation message reference is recorded.
5. All `crates` gates and btit gates pass at the branch head.

## Required Validation

- `cargo fmt --check --all --manifest-path crates/Cargo.toml`
- `cargo clippy --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings`
- `cargo test --manifest-path crates/Cargo.toml --workspace`
- `cargo +1.94.1 check --manifest-path crates/Cargo.toml --workspace --all-targets`
- `diff <(cargo tree --manifest-path crates/Cargo.toml -p sc-observability-log -e normal --prefix none | sort -u) crates/runtime-deps.txt`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `pnpm test`
