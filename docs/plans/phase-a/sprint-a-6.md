---
id: a-6
title: Handoff into ../sc-observability for publish
status: planned
branch: feature/sprint-a-6-sc-handoff
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-a-6-sc-handoff
target: integrate/phase-a
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

Recommended agent: not set. The btit developer pane is still `tbd` in `.atm.toml`.

Planning advice: team-lead assigns from the active pool.

## Goal

- Copy the reviewed crates into `../sc-observability` immediately after a-5, as workspace members, with every sc-observability CI gate green and every sc-observability governance list extended to cover them. They are then ready for sc-observability's release workflow to publish.
- Implement the a-5 type-placement decision there.
- Record, in btit, QA-inspectable evidence of the merged sc-observability PR.

## Hard Dependencies

- a-5 pushed (stack parent), with `review-a-5.md` showing `open_blocking: 0`, `open_important: 0` and an `allowed_duplicate_names` list.
- The sc-observability branch model, per `../sc-observability/docs/git-workflows.md` (feature branch from `develop`, PR to `develop`).
- sc-observability team approval of the PR in `../sc-observability`.

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA; merge parent → child before every dev/fix round. PR-completion trigger: parent PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- a-5 → a-6 — `must_follow` (a-6 follows a-5): copies the reviewed, fixed crates and implements the recorded type-placement decision. a-5 is the stack parent.

Stack: `phase-a-core` · layer 5 (top).

## Exact Targets

**btit:**
- `docs/plans/phase-a/handoff-a-6.md` (new; the QA evidence artifact)
- `docs/plans/phase-a/sprint-a-6.md` (`status:` frontmatter only)

**`../sc-observability`** (branch `feature/sc-observability-log`, created from `develop`):
- `crates/sc-observability-log/**`, `crates/sc-observability-log-macros/**`, `crates/sc-observability-log-consumer-check/**` (copied from the btit source tag)
- `crates/sc-observability-types/**` (only the items the a-5 type-placement decision moves)
- `Cargo.toml` (`[workspace].members`; `[workspace.dependencies]` additions, including the `=` pin for `sc-observability-log-macros`)
- `release/publish-artifacts.toml` (two `[[crates]]` entries)
- `scripts/ci/validate_dependency_bans.sh` (`required_members` at lines 28-33; exact runtime and test dependency baselines for the three new packages)
- `scripts/ci/validate_repo_boundaries.sh` (`required_members` at lines 30-35; `shared_crate_roots` at lines 71-76; layering checks for the new crates)
- `scripts/ci/validate_docs_consistency.sh` (the `for crate in …` rustdoc `-Dmissing-docs` list at line 46)
- `scripts/ci/validate_version_literals.py` (`workspace_dep_version_pattern` at lines 24-26 also matches an exact `"=X.Y.Z"` pin)
- `scripts/ci/validate_public_api_diff.sh` (per-package loop at lines 20-39: a package not yet on crates.io is reported as a new crate instead of aborting the script)
- `docs/requirements.md` (new layering rules after LAY-007, section 2, lines 53-59)
- `docs/architecture.md` (section 6 crate boundary table, lines 692-699; a new overview paragraph after the section 1 diagram, leaving the diagram text unchanged)
- `docs/api-design.md` (section 3.1, lines 80-101: the crate-count statements at lines 82-87, 91-92 and 96-97, and the pair's position next to the layering statement at lines 99-100; the line-100 needle stays byte-identical)
- `docs/api-approvals/` (public API approval record for the two published crates)
- `README.md`, `CONSUMING.md`, `docs/migration-guide.md`, `CHANGELOG.md` (`[Unreleased]`)
- `packages/sc-observability/skills/sc-observability-adopting/references/migrate-from-log.md` and `migrate-from-tracing.md` (point to the new crates)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Source anchor.** btit `team-lead` pushes the annotated tag `phase-a/handoff-a-6-source` on the btit commit being copied (the a-5 head after all fixes). `handoff-a-6.md` records the tag and `git rev-parse phase-a/handoff-a-6-source:crates` (tree hash), so the evidence survives the rebase cascade and the a-5/a-6 merges.
2. **Copy.** The three packages are copied into `../sc-observability/crates/` from that tag and switched to inherit the sc-observability `[workspace.package]` fields: version, edition, license, rust-version, repository, homepage. `publish = false` is removed from the two library crates; `sc-observability-log-consumer-check` keeps `publish = false`.
3. **Lockstep pin.** `../sc-observability/Cargo.toml` `[workspace.dependencies]` gets `sc-observability-log-macros = { version = "=<workspace version>", path = "crates/sc-observability-log-macros" }` and `sc-observability-log = { version = "<workspace version>", path = "crates/sc-observability-log" }`, following `serde` → `serde_core = "=1.0.228"`. `validate_version_literals.py` is extended so its workspace-dependency pattern also captures `"=X.Y.Z"` (verified while planning: the current pattern does not match an `=` pin, so the pin would silently escape the version-literal check). The README states that `__private` is outside semver and the two crates always release together at the same version.
4. **Lint parity without weakening.** sc-observability's `[workspace.lints]` has only `missing_debug_implementations`, `unsafe_op_in_unsafe_fn` and clippy `pedantic`, and Cargo does not allow `[lints] workspace = true` to be combined with extra entries. The three packages therefore replace `[lints] workspace = true` with an explicit per-package `[lints.rust]`/`[lints.clippy]` table: sc-observability's workspace set plus the no-panic deny set from a-1 (`pedantic` with `priority = -1`). Their `clippy.toml` files are copied unchanged. Existing sc-observability crates are not changed.
5. **Type placement.** The a-5 decision is implemented. Items moved into `sc-observability-types` are removed from the new crates, which then import them from there.
6. **Publish order.** `release/publish-artifacts.toml` gets two entries: `sc-observability-log-macros` with `publish_order = 5`, then `sc-observability-log` with `publish_order = 6`. `wait_after_publish_seconds` follows the existing pattern: `30` except on the last entry, and `sc-observability-otlp` changes from `0` to `30`. `validate_publish_order.sh` passes.
7. **Governance lists cover the new crates.** Every hardcoded crate list in sc-observability CI names the new packages, so they cannot be silently ungoverned while CI stays green:
   - `validate_dependency_bans.sh`: `required_members` includes the three packages; exact runtime baselines — `sc-observability-log` = {`sc-observability`, `sc-observability-types`, `log`, `serde`, `serde_json`, `thiserror`, `sc-observability-log-macros`}, `sc-observability-log-macros` = {`syn`, `quote`, `proc-macro2`}, `sc-observability-log-consumer-check` = {`sc-observability-log`}; test baseline for `sc-observability-log` ⊆ {`tempfile`, `trybuild`, `tracing`, `tokio`}; neither library crate may depend on `sc-observe` or `sc-observability-otlp`. Every existing ban still holds.
   - `validate_repo_boundaries.sh`: `required_members` and `shared_crate_roots` include the three packages; the same no-`sc-observe`/no-`otlp` rule is asserted.
   - `validate_docs_consistency.sh`: the rustdoc `-Dmissing-docs` loop includes `sc-observability-log` and `sc-observability-log-macros`.
8. **Layering docs.** `docs/requirements.md` gains rules after LAY-007: `sc-observability-log-macros` depends on no workspace crate; `sc-observability-log` depends on `sc-observability-types`, `sc-observability` and `sc-observability-log-macros` only and not on `sc-observe` or `sc-observability-otlp`. `docs/architecture.md` section 6 gains a boundary-table row per library crate, plus an overview paragraph placing the pair beside `sc-observability`. `docs/api-design.md` section 3.1 no longer says "4-crate workspace": the statement at line 82 and its list (lines 84-87) name every workspace member after a-6 (the four existing crates, `sc-observability-log-macros`, `sc-observability-log` and the unpublished `sc-observability-log-consumer-check`), the baseline-update bullets at lines 91-92 and 96-97 describe that shape, and a new bullet after line 100 states the pair's position next to the linear layering. Line 100, `` `sc-observability-types <- sc-observability <- sc-observe <- sc-observability-otlp` ``, contains the `api_checks` needle in `validate_docs_consistency.sh` (lines 28-33) and is not edited. The consistency needles in `validate_docs_consistency.sh` (the stack diagram in `requirements.md`, the architecture markers, the api-design layering string) are left byte-identical. No ADR is added (sc-observability has no ADR practice for this).
9. **trybuild snapshots on the destination toolchain.** sc-observability runs `cargo test --workspace` on 1.94.1 (`rust-toolchain.toml` and the `test` job), while btit ran the crates' tests on 1.98.1. The `tests/ui/*.stderr` files are regenerated on 1.94.1 with `TRYBUILD=overwrite`, and the diff is reviewed so every case still names its unsupported form.
10. **Public API governance.** sc-observability CI runs the three scripts in the `public-api-governance` job, with `continue-on-error: true` on the diff step (`.github/workflows/ci.yml:112-115`; the same in `release-preflight.yml:92-95`). The diff script diffs each publishable library or proc-macro package against its latest crates.io release (`cargo public-api --manifest-path <manifest> -sss diff --deny all latest`, `validate_public_api_diff.sh:24`) and exits 1 on any diff (lines 49-51). Verified while planning on a copy of sc-observability `develop@18a1238` with the two new packages added as members: `cargo public-api` also exits 1 for a crate that is not on crates.io, the script then aborts at the first new package under `set -e` (line 35), leaves `target/public-api/public-api-diff.txt` truncated and writes no `public-api-diff.status`, and `validate_public_api_docs.sh` then fails when it sources the missing status file (line 49). a-6 therefore:
    - extends `validate_public_api_diff.sh` as shown in the code samples: before diffing, it looks the package up in the crates.io sparse index; HTTP 404 writes a `new crate` section with the package's full `cargo public-api -sss` listing and sets `PUBLIC_API_DIFF_FOUND=1`; any status other than 200 or 404, or a failed lookup, exits 2;
    - adds the approval record `docs/api-approvals/sc-observability-log-crates.md` with the `## Scope`, `## Approval` and `## Affected Artifacts` headings that `validate_public_api_docs.sh` requires (lines 25-35), covering both published crates.

    The requirement is: the diff script runs to completion and exits 0 or 1, never another status. Its report `target/public-api/public-api-diff.txt` contains a `new crate` section for exactly `sc-observability-log-macros` and `sc-observability-log`, and every existing crate's `-`/`+` item lines are identical to those in the report the same script produces on the `develop` base commit, saved as `target/public-api/public-api-diff.base.txt` (the sections also carry cargo progress output with timings, so only item lines are compared). Pre-existing drift is therefore allowed but not introduced by a-6: on `18a1238` the `sc-observability-types` section already shows a `Timestamp` `time::Duration` diff. `validate_public_api_semver.py` and `validate_public_api_docs.sh` pass, with the approval record under `docs/api-approvals/`. The report goes into `handoff-a-6.md`. (`cargo semver-checks` cannot check a crate that is not on crates.io, or a proc-macro crate, so the semver script passes these two through its approval-record path, `validate_public_api_semver.py:116-118`; this is recorded in the approval record.)
11. **Docs.** The sc-observability docs and adoption-skill references route `log` and `tracing` migrations to the new crates. They describe migration from tracing 0.1 as "Migration from tracing 0.1 is an import rename for every supported form. The rejected-forms table lists the tracing-valid forms that fail loudly at compile time.", and link that rejected-forms table (`parent:`, `follows_from`, deferred fields via `tracing::field::Empty` or `fields(x)` without a value, the `"" = v` empty key, reserved-prefix keys `sc_observability_log.*` in string and dotted forms, and span macros).
12. **Merged PR.** The PR in `../sc-observability` is merged, with team approval and every CI job green on ubuntu, macOS and Windows.
13. **btit evidence.** `docs/plans/phase-a/handoff-a-6.md` records the fields in the code sample.

## Required Work

- Before opening the PR, run sc-observability's full gate set locally on the copied tree.
- Check the no-duplicate rule with the duplicate-names command in Required Validation and compare its output with `allowed_duplicate_names` from `review-a-5.md`. The command builds each name set from the public API (`cargo public-api -sss`, the tool and flags `validate_public_api_diff.sh:24` already uses; it needs the nightly toolchain that the CI job installs), not from `pub struct` text in source. A source grep cannot see macro-generated types: `error_wrapper!` (`crates/sc-observability-types/src/errors.rs:19-34`, invoked at lines 36-67) generates `InitError`, `EventError`, `FlushError`, `ShutdownError`, `ProjectionError`, `SubscriberError`, `LogSinkError` and `ExportError`, and `validated_name_type!` (`validation.rs:42`, invoked at lines 159-218) generates `ToolName`, `EnvPrefix`, `ServiceName`, `TargetCategory`, `ActionName`, `MetricName`, `MetricUnit`, `StateName`, `CorrelationId`, `OutcomeLabel`, `SinkName` and `SchemaVersion`. Verified while planning: the old grep finds none of these, and the public-API extraction finds all of them. Names are the last path segment of top-level `pub struct|enum|trait|union|type` items; associated types (`pub type …::ActionName::Error`) and `pub use` re-exports are excluded, because a re-export of an `sc-observability-types` item is the same item, not a duplicate. The re-exported names that overlap `sc-observability-types` are recorded separately (`reexported_overlap`), and each must be a `pub use sc_observability_types::…` re-export.
- Produce the base report: in a `git worktree` of sc-observability `origin/develop`, run `bash scripts/ci/validate_public_api_diff.sh` and copy its `target/public-api/public-api-diff.txt` to `target/public-api/public-api-diff.base.txt` in the a-6 tree.
- Record in `handoff-a-6.md` the full name sets (`types_public_type_names`, `new_crates_public_type_names`), the duplicate-names output, `reexported_overlap` and the public API diff report.
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
description = "log bridge, tracing-compatible event macros and #[instrument] for sc-observability."

[lints.rust]
missing_debug_implementations = "warn"
unsafe_op_in_unsafe_fn = "warn"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
unreachable = "deny"
todo = "deny"
unimplemented = "deny"
indexing_slicing = "deny"
```

```bash
# ../sc-observability/scripts/ci/validate_public_api_diff.sh (inserted after line 22, `echo "=== ${package} ===" >>"$report_path"`)
    # crates.io sparse index path for names of 4+ characters: <first 2>/<next 2>/<name>
    index_url="https://index.crates.io/${package:0:2}/${package:2:2}/${package}"
    index_status="$(curl -sS -o /dev/null -w '%{http_code}' "$index_url")" || {
        echo "crates.io index lookup failed for ${package}" >&2
        exit 2
    }
    case "$index_status" in
        200) ;;
        404)
            has_diff=1
            echo "new crate: not yet published on crates.io; full public API follows" >>"$report_path"
            cargo public-api --manifest-path "$manifest_path" -sss >>"$report_path"
            echo >>"$report_path"
            continue
            ;;
        *)
            echo "crates.io index lookup for ${package} returned HTTP ${index_status}" >&2
            exit 2
            ;;
    esac
```

```python
# ../sc-observability/scripts/ci/validate_version_literals.py (lines 24-26, changed)
workspace_dep_version_pattern = re.compile(
    r'^\s*([A-Za-z0-9_.-]+)\s*=\s*\{(?=.*\bversion\s*=\s*"=?(\d+\.\d+\.\d+)")(?=.*\bpath\s*=\s*"([^"]+)").*\}\s*$'
)
```

```markdown
<!-- docs/plans/phase-a/handoff-a-6.md (QA evidence artifact) -->
btit_source_tag: phase-a/handoff-a-6-source
btit_source_crates_tree: <git rev-parse phase-a/handoff-a-6-source:crates>
sc_observability_pr: https://github.com/randlee/sc-observability/pull/<n>
merge_commit: <sha on sc-observability develop>
approved_by: <reviewer(s)>
ci_run: <url>   jobs: fmt ✅ clippy ✅ docs-consistency ✅ dependency-bans ✅ version-literals ✅ public-api-governance ✅ manifest-validation ✅ tests(ubuntu/macos/windows) ✅
types_public_type_names: [<sorted output of api_type_names crates/sc-observability-types/Cargo.toml>]
new_crates_public_type_names: [<sorted output for sc-observability-log and sc-observability-log-macros>]
duplicate_names_check: <output of the duplicate-names command>   allowed_duplicate_names (review-a-5.md): [...]
reexported_overlap: <output of the re-export overlap command>   each is `pub use sc_observability_types::…`: yes
public_api_diff_report: |
  <contents of target/public-api/public-api-diff.txt; new-crate sections for exactly the two library crates>
public_api_diff_existing_sections_match_develop_base: yes (<develop base sha>)
public_api_approval: docs/api-approvals/sc-observability-log-crates.md
phase_b_followup_issue: https://github.com/randlee/beads-task-issue-tracker/issues/<n>
source_of_truth: ../sc-observability (later changes to these crates land there first)
```

## This Sprint Does Not Close

- **Publishing to crates.io.** The sc-observability release workflow and its `publisher` agent own this, per `../sc-observability/docs/publishing.md`.
- **Switching btit to the published crates and deleting btit `crates/sc-observability-log*`.** This is phase-b.
- **Span export over OTLP.**
- **Applying the no-panic lints or error-enum convention to existing sc-observability crates.**

## Acceptance Criteria

1. `handoff-a-6.md` exists with every field shown in the code sample populated, and `git rev-parse phase-a/handoff-a-6-source:crates` equals `btit_source_crates_tree`. QA verifies closure from this file alone.
2. The recorded PR URL shows it merged, and the recorded CI run shows every listed job green on all three OSes.
3. Both library crates inherit the sc-observability workspace version and neither sets `publish = false`; `sc-observability-log-macros` is pinned with `=` in `[workspace.dependencies]`, and `validate_version_literals.py` captures that pin.
4. `sc-observability-log-macros` has `publish_order = 5` and `sc-observability-log` has `publish_order = 6`, and `validate_publish_order.sh` passes.
5. Every public type, trait, enum, union and type-alias name, macro-generated ones included, that appears in both the `sc-observability-types` public API and the new crates' public API is listed in `allowed_duplicate_names` in `review-a-5.md`, and every item group the a-5 decision moved into `sc-observability-types` is no longer defined in the new crates. The duplicate-names command output is a subset of `allowed_duplicate_names`, every `reexported_overlap` name is a `pub use sc_observability_types::…` re-export, and both name sets are recorded in `handoff-a-6.md`.
6. `validate_dependency_bans.sh`, `validate_repo_boundaries.sh` and `validate_docs_consistency.sh` name the new packages, and each fails when a new crate gains a banned dependency or loses its docs (demonstrated locally once and noted in the PR description).
7. The three packages' `[lints]` tables contain the no-panic deny set, and `cargo clippy --all-targets --all-features -- -D warnings` passes in `../sc-observability` on its 1.94.1 toolchain.
8. The phase-b follow-up issue exists and is linked.
9. `validate_public_api_diff.sh` runs to completion with exit 0 or 1. Its report has `new crate` sections for exactly `sc-observability-log-macros` and `sc-observability-log`, and every existing crate's `-`/`+` item lines match the `develop` base report. `validate_public_api_semver.py` and `validate_public_api_docs.sh` exit 0, `docs/api-approvals/sc-observability-log-crates.md` exists, and the report is recorded in `handoff-a-6.md`.

## Required Validation

**In `../sc-observability`** (run from the sc-observability repo root in bash; the duplicate-names and re-export commands use process substitution and need the nightly toolchain for `cargo public-api`):
- `cargo fmt --check --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `bash scripts/ci/validate_dependency_bans.sh`
- `bash scripts/ci/validate_publish_order.sh`
- `bash scripts/ci/validate_docs_consistency.sh`
- `python3 scripts/ci/validate_version_literals.py`
- `bash scripts/ci/validate_repo_boundaries.sh`
- `bash scripts/ci/validate_public_api_diff.sh; rc=$?; [ "$rc" -eq 0 ] || [ "$rc" -eq 1 ]` (CI runs this step with `continue-on-error: true`; the gate is that it completes and writes `target/public-api/public-api-diff.txt` and `target/public-api/public-api-diff.status`)
- `grep -B1 -x 'new crate: not yet published on crates.io; full public API follows' target/public-api/public-api-diff.txt | grep -oE '^=== [a-z-]+ ===$' | diff - <(printf '=== sc-observability-log-macros ===\n=== sc-observability-log ===\n')` (order follows `[workspace].members`)
- `existing_items() { awk '/^=== /{c=$2} /^[-+]/{ if (c!="sc-observability-log" && c!="sc-observability-log-macros") print c": "$0 }' "$1"; }; diff <(existing_items target/public-api/public-api-diff.base.txt) <(existing_items target/public-api/public-api-diff.txt)`
- `test -f docs/api-approvals/sc-observability-log-crates.md`
- `python3 scripts/ci/validate_public_api_semver.py`
- `bash scripts/ci/validate_public_api_docs.sh`
- `python3 scripts/release_artifacts.py validate-manifest --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml`
- duplicate names: `api_type_names() { cargo public-api --manifest-path "$1" -sss | sed -nE 's/^pub (struct|enum|trait|union|type) ([a-z_][a-z0-9_]*::)+([A-Z][A-Za-z0-9_]*)(<.*|\(.*|:[^:].*| .*)?$/\3/p' | LC_ALL=C sort -u; }; comm -12 <(api_type_names crates/sc-observability-types/Cargo.toml) <(cat <(api_type_names crates/sc-observability-log/Cargo.toml) <(api_type_names crates/sc-observability-log-macros/Cargo.toml) | LC_ALL=C sort -u)`
- re-export overlap: `api_type_names() { cargo public-api --manifest-path "$1" -sss | sed -nE 's/^pub (struct|enum|trait|union|type) ([a-z_][a-z0-9_]*::)+([A-Z][A-Za-z0-9_]*)(<.*|\(.*|:[^:].*| .*)?$/\3/p' | LC_ALL=C sort -u; }; comm -12 <(api_type_names crates/sc-observability-types/Cargo.toml) <(cargo public-api --manifest-path crates/sc-observability-log/Cargo.toml -sss | sed -nE 's/^pub use ([a-z_][a-z0-9_]*::)+([A-Z][A-Za-z0-9_]*)$/\2/p' | LC_ALL=C sort -u)`

**In btit:**
- `test -f docs/plans/phase-a/handoff-a-6.md`
- `test "$(git rev-parse phase-a/handoff-a-6-source:crates)" = "$(sed -n 's/^btit_source_crates_tree: //p' docs/plans/phase-a/handoff-a-6.md)"`
