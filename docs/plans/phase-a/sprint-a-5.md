---
id: a-5
title: sc-observability team critical review and handoff into ../sc-observability
status: planned
branch: feature/sprint-a-5-sc-observability-handoff
target: develop
recommended_model: higher-effort (cross-repo governance gates, review resolution)
dependency_relations:
  - prerequisite: a-4
    dependent: a-5
    relation: must_follow
    rationale: The review must cover the crates as adopted by a real consumer (btit on the bridge); a-5 copies the post-a-4 crate sources. Merge a-4 forward before every dev/fix round; a-4 PR merges first.
---

# Sprint a-5 — sc-observability team critical review and handoff

## Goal

- Get a critical review of `sc-observability-log` and `sc-observability-log-macros` from the sc-observability team and resolve every finding.
- Copy the reviewed crates into `../sc-observability` right away as workspace members, with every sc-observability CI gate passing. They are then ready for sc-observability's release workflow to publish.
- Record which items belong in `sc-observability-types`, the shared low-level types crate, versus the new crates, so shared types are not duplicated.

## Hard Dependencies

- a-4 merged: btit `develop` runs on the bridge.
- sc-observability team availability for review, following `../sc-observability/docs/team-protocol.md`.
- sc-observability `develop`/`main` branch model, per `../sc-observability/docs/git-workflows.md`.

## Exact Targets

**btit:**
- `crates/sc-observability-log/**`, `crates/sc-observability-log-macros/**` (review fixes only)
- `docs/plans/phase-a/review-a-5.md` (new; review findings and dispositions)

**`../sc-observability` (branch `feature/sc-observability-log` from its integration branch):**
- `crates/sc-observability-log/**`, `crates/sc-observability-log-macros/**` (copied)
- `Cargo.toml` (`[workspace].members`; `[workspace.dependencies]` additions)
- `release/publish-artifacts.toml` (two `[[crates]]` entries with `publish_order`)
- `scripts/ci/validate_dependency_bans.sh` (allowed dependency baselines for the two crates)
- `docs/api-approvals/` (public API approval record for the two crates)
- `README.md`, `CONSUMING.md`, `docs/migration-guide.md`, `CHANGELOG.md` (`[Unreleased]`)
- `packages/sc-observability/skills/sc-observability-adopting/references/migrate-from-log.md` and `migrate-from-tracing.md` (point to the new crates)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Critical review.** The sc-observability team reviews both crates at the a-4 merge commit, using its review agents (`rust-architect`, `rust-code-reviewer`, `arch-qa` in `../sc-observability/.claude/agents/`). Each finding goes into `docs/plans/phase-a/review-a-5.md` with severity (Blocking / Important / Minor), a disposition (fixed with commit, or rejected with the reviewer's agreement), and the reviewed commit.
2. **Blocking and Important findings fixed in btit first.** Every Blocking and Important finding is fixed in `crates/` in btit, and btit gates are re-run, before the copy. Minor findings are fixed or recorded as issues in `../sc-observability`.
3. **Type-placement decision.** `review-a-5.md` records, with the team's decision, where each of these belongs: `sc-observability-types` or the new crates.
   - the tracing-style `Level` constants
   - `TraceId`/`SpanId` generation
   - `TargetCategory`/`ActionName` sanitizers
   - `LevelFilter` conversions

   Anything moved into `sc-observability-types` is implemented in the sc-observability PR, and the new crates depend on it from there. No type is defined twice.
4. **Copy.** Both crates are copied into `../sc-observability/crates/` and converted to inherit the sc-observability `[workspace.package]` (version, edition, license, rust-version, repository, homepage). `publish = false` is removed.
5. **Publish order.** The two `release/publish-artifacts.toml` entries have `publish_order`: `sc-observability-log-macros` before `sc-observability-log`, after all existing entries (orders 5 and 6); `sc-observability-otlp`'s `wait_after_publish_seconds` becomes 30 since it is no longer last. `validate_publish_order.sh` passes.
6. **Dependency baselines.** `validate_dependency_bans.sh` pins exact allowed runtime and test dependency sets for both crates. Every existing ban still holds (no `log` or `tracing` in the core crates, no OTLP outside `sc-observability-otlp`).
7. **Public API governance.** The public API governance artifacts for both crates are added. `validate_public_api_diff.sh`, `validate_public_api_semver.py` and `validate_public_api_docs.sh` pass.
8. **Documentation.** The sc-observability docs and adoption skill references point `log` and `tracing` migrations to the new crates.
9. **PR merged.** A PR in `../sc-observability` with all CI jobs green on ubuntu, macOS and Windows is merged, with sc-observability team approval.

## Required Work

- **Before copying:** run sc-observability's own gates locally against the copied tree:
  - `cargo fmt --check --all`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --workspace`
  - every `scripts/ci/validate_*.sh` / `.py` script
- **btit follow-up issue:** open one btit GitHub issue for phase-b to "replace path dependency with crates.io `sc-observability-log` once published". Link it in `review-a-5.md`.
- **After merge:** btit `crates/` remains the working copy only until phase-b switches to the published crates. Record in `review-a-5.md` that `../sc-observability` is now the source of truth, and that later changes go there first.

## Explicit Code Samples

```toml
# ../sc-observability/release/publish-artifacts.toml (additions; schema_version = 1 format)
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
wait_after_publish_seconds = 0   # last entry; sc-observability-otlp (order 4) changes to 30

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

## This Sprint Does Not Close

- **Publishing to crates.io.** sc-observability's release workflow and `publisher` agent own it, per `../sc-observability/docs/publishing.md`.
- **btit on the published crates.** Switching btit to the published crates and deleting btit `crates/sc-observability-log*` is phase-b.
- **OTLP.** Span export over OTLP is not part of this work.

## Acceptance Criteria

1. `docs/plans/phase-a/review-a-5.md` lists every review finding with severity, disposition and reviewed commit. No Blocking or Important finding is open.
2. `review-a-5.md` records the type-placement decision for all four item groups, and no type exists in both `sc-observability-types` and the new crates.
3. The sc-observability PR is merged with team approval, and every CI job is green on all three OSes (fmt, clippy, docs-consistency, dependency-bans, version-literals, public-api-governance, manifest-validation, tests).
4. Both crates inherit the sc-observability workspace version, and `publish = false` is gone.
5. `validate_publish_order.sh` passes with the new entries. `sc-observability-log-macros` publishes before `sc-observability-log`.
6. The dependency-ban baselines for the new crates are exact sets, and the pre-existing bans are unchanged.
7. The btit phase-b follow-up issue exists and is linked from `review-a-5.md`.

## Required Validation

**In btit** (after review fixes):
- `cargo fmt --check --all --manifest-path crates/Cargo.toml`
- `cargo clippy --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings`
- `cargo test --manifest-path crates/Cargo.toml --workspace`
- `cargo test --manifest-path src-tauri/Cargo.toml`

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
