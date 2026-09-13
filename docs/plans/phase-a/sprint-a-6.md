---
id: a-6
title: Handoff into ../sc-observability for publish
status: planned
branch: feature/sprint-a-6-sc-handoff
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-a-6-sc-handoff
target: develop
recommended_model: higher-effort (cross-repo governance gates)
dependency_relations:
  - prerequisite: a-5
    dependent: a-6
    relation: must_follow
    rationale: "copies the crates after every Blocking/Important review finding is fixed and implements the a-5 type-placement decision; stack parent"
---

# Sprint a-6 — Handoff into ../sc-observability for publish

## Recommended Agent / Model

Recommended model: higher-effort (cross-repo governance gates).

Recommended agent: not set. The btit developer pool is pending the `arch-ctm` decision on PR #38.

Planning advice: team-lead assigns from the active pool.

## Goal

- Copy the reviewed crates into `../sc-observability` immediately after a-5, as workspace members, with every sc-observability CI gate green. They are then ready for sc-observability's release workflow to publish.
- Implement the a-5 type-placement decision there, so no type exists in both `sc-observability-types` and the new crates.
- Record, in btit, QA-inspectable evidence of the merged sc-observability PR.

## Hard Dependencies

- a-5 pushed (stack parent), with `review-a-5.md` showing `open_blocking: 0` and `open_important: 0`.
- The sc-observability branch model, per `../sc-observability/docs/git-workflows.md`.
- sc-observability team approval of the PR in `../sc-observability`.

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA; merge parent → child before every dev/fix round. PR-completion trigger: parent PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- a-5 → a-6 — `must_follow` (a-6 follows a-5): copies the reviewed, fixed crates and implements the recorded type-placement decision. a-5 is the stack parent.

Stack: `phase-a-core` · layer 5 (top).

## Exact Targets

**btit:**
- `docs/plans/phase-a/handoff-a-6.md` (new; the QA evidence artifact)

**`../sc-observability`** (branch `feature/sc-observability-log`, created from its integration branch):
- `crates/sc-observability-log/**`, `crates/sc-observability-log-macros/**` (copied from btit at the a-5 head)
- `crates/sc-observability-types/**` (only the items the a-5 type-placement decision moves)
- `Cargo.toml` (`[workspace].members`; `[workspace.dependencies]` additions)
- `release/publish-artifacts.toml` (two `[[crates]]` entries)
- `scripts/ci/validate_dependency_bans.sh` (exact allowed dependency baselines for the two crates)
- `docs/api-approvals/` (public API approval record for the two crates)
- `README.md`, `CONSUMING.md`, `docs/migration-guide.md`, `CHANGELOG.md` (`[Unreleased]`)
- `packages/sc-observability/skills/sc-observability-adopting/references/migrate-from-log.md` and `migrate-from-tracing.md` (point to the new crates)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Copy.** Both crates are copied into `../sc-observability/crates/` from the btit a-5 head commit (recorded in `handoff-a-6.md`) and switched to inherit the sc-observability `[workspace.package]` fields: version, edition, license, rust-version, repository, homepage. `publish = false` is removed.
2. **Type placement.** The a-5 decision is implemented. Items moved into `sc-observability-types` are removed from the new crates, which then import them from there.
3. **Publish order.** `release/publish-artifacts.toml` gets two entries: `sc-observability-log-macros` with `publish_order = 5`, then `sc-observability-log` with `publish_order = 6`. `wait_after_publish_seconds` follows the existing pattern: `30` except on the last entry, and `sc-observability-otlp` changes from `0` to `30`. `validate_publish_order.sh` passes.
4. **Dependency baselines.** `validate_dependency_bans.sh` pins exact runtime and test dependency sets for both crates. Every existing ban still holds.
5. **Public API governance.** The public API governance artifacts for both crates are added. `validate_public_api_diff.sh`, `validate_public_api_semver.py` and `validate_public_api_docs.sh` pass.
6. **Docs.** The sc-observability docs and adoption-skill references route `log` and `tracing` migrations to the new crates.
7. **Merged PR.** The PR in `../sc-observability` is merged, with team approval and every CI job green on ubuntu, macOS and Windows.
8. **btit evidence.** `docs/plans/phase-a/handoff-a-6.md` records:
   - the btit source commit
   - the sc-observability PR URL, merge commit and approving reviewer(s)
   - the CI run URL for the merge commit, with each job's result
   - the btit phase-b follow-up issue URL for "replace path dependency with crates.io `sc-observability-log` once published"
   - a statement that `../sc-observability` is now the source of truth for both crates

## Required Work

- Before opening the PR, run sc-observability's full gate set locally on the copied tree.
- After merge, open the btit phase-b follow-up issue and link it in `handoff-a-6.md`.

## Explicit Code Samples

```toml
# ../sc-observability/release/publish-artifacts.toml (additions; schema_version = 1)
[[crates]]
artifact = "sc-observability-log-macros"
package = "sc-observability-log-macros"
cargo_toml = "crates/sc-observability-log-macros/Cargo.toml"
publish_order = 5
wait_after_publish_seconds = 30

[[crates]]
artifact = "sc-observability-log"
package = "sc-observability-log"
cargo_toml = "crates/sc-observability-log/Cargo.toml"
publish_order = 6
wait_after_publish_seconds = 0

# ../sc-observability/crates/sc-observability-log/Cargo.toml (after copy)
[package]
name = "sc-observability-log"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
repository.workspace = true
homepage.workspace = true
```

```markdown
<!-- docs/plans/phase-a/handoff-a-6.md (QA evidence artifact) -->
btit_source_commit: <sha>
sc_observability_pr: https://github.com/randlee/sc-observability/pull/<n>
merge_commit: <sha>
approved_by: <reviewer(s)>
ci_run: <url>   jobs: fmt ✅ clippy ✅ docs-consistency ✅ dependency-bans ✅ version-literals ✅ public-api-governance ✅ manifest-validation ✅ tests(ubuntu/macos/windows) ✅
phase_b_followup_issue: https://github.com/randlee/beads-task-issue-tracker/issues/<n>
source_of_truth: ../sc-observability (later changes to these crates land there first)
```

## This Sprint Does Not Close

- **Publishing to crates.io.** The sc-observability release workflow and its `publisher` agent own this, per `../sc-observability/docs/publishing.md`.
- **Switching btit to the published crates and deleting btit `crates/sc-observability-log*`.** This is phase-b.
- **Span export over OTLP.**

## Acceptance Criteria

1. `handoff-a-6.md` exists with every field shown in the code sample populated. QA verifies closure from this file alone.
2. The recorded PR URL shows it merged, and the recorded CI run shows every listed job green on all three OSes.
3. Both crates inherit the sc-observability workspace version, and neither sets `publish = false`.
4. `sc-observability-log-macros` has `publish_order = 5` and `sc-observability-log` has `publish_order = 6`, and `validate_publish_order.sh` passes.
5. No type exists in both `sc-observability-types` and the new crates, as the a-5 decision requires.
6. The phase-b follow-up issue exists and is linked.

## Required Validation

**In `../sc-observability`:**
- `cargo fmt --check --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `bash scripts/ci/validate_dependency_bans.sh`
- `bash scripts/ci/validate_publish_order.sh`
- `bash scripts/ci/validate_docs_consistency.sh`
- `python3 scripts/ci/validate_version_literals.py`
- `bash scripts/ci/validate_repo_boundaries.sh`
- `bash scripts/ci/validate_public_api_diff.sh`
- `python3 scripts/ci/validate_public_api_semver.py`
- `bash scripts/ci/validate_public_api_docs.sh`

**In btit:**
- `test -f docs/plans/phase-a/handoff-a-6.md`
