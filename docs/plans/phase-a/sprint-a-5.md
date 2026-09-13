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

Recommended agent: not set. The btit developer pane is still `tbd` in `.atm.toml`.

Planning advice; team-lead assigns from the active pool.

## Goal

- Get a critical review of `sc-observability-log` and `sc-observability-log-macros` from the sc-observability team.
- Close every Blocking and Important finding inside btit `crates/` (and, when a fix changes the runtime graph, the btit files that consume it).
- Agree with the team on which items belong in the shared low-level `sc-observability-types` crate and which stay in the new crates, and record that decision, including which same-named items are deliberately kept in both.
- Every closure gate in this sprint is a btit artifact. None depends on merging a change in another repository.

## Hard Dependencies

- a-3 is pushed (it is the stack parent). The a-4 PR is merged to `develop`, and the `phase-a-core` layers have been rebased onto that `develop` before development starts.
- Review coordination runs over ATM between btit `team-lead` (team `bit`, `.atm.toml`) and `cobs` on team `sc-observability` (`../sc-observability/.atm.toml`), following `../sc-observability/docs/team-protocol.md` (ack on receipt, completion message, receiver ack). btit `team-lead` sends the review request and receives findings; `cobs` owns the review on the sc-observability side, including which of that team's reviewers it involves.

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA;
merge parent → child before every dev/fix round. PR-completion trigger: parent
PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- a-3 → a-5 — `must_follow` (a-5 follows a-3): the review covers the complete crate API; stack parent.
- a-4 → a-5 — `must_follow` (a-5 follows a-4): the review covers the crates as adopted by btit. The a-4 PR merges, and the stack is rebased onto `develop`, before a-5 development starts.
- a-5 → a-6 — `must_follow` (a-6 follows a-5): a-6 copies the reviewed, fixed crates.

Stack: `phase-a-core` · layer 4.

## Exact Targets

- `crates/sc-observability-log/**`, `crates/sc-observability-log-macros/**`, `crates/sc-observability-log-consumer-check/**` (review fixes only)
- `crates/Cargo.toml`, `crates/Cargo.lock` (review fixes only)
- `crates/runtime-deps.txt` (only when a fix changes a runtime dependency; regenerated in the same commit)
- `src-tauri/Cargo.lock` (only when a fix changes a runtime dependency; refreshed with `cargo check --manifest-path src-tauri/Cargo.toml` in the same commit). a-4 has merged and no `parallel_safe` sprint is open, so this does not intersect another sprint.
- `docs/plans/phase-a/review-a-5.md` (new; findings, dispositions, type-placement decision)
- `docs/plans/phase-a/sprint-a-5.md` (`status:` frontmatter only)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

1. **Review anchor.** Before sending the request, btit `team-lead` pushes the annotated tag `phase-a/review-a-5-r1` on the a-5 branch head (after the rebase onto `develop`). The tag keeps the reviewed commit reachable after later rebases; `review-a-5.md` records the tag and the content hash `git rev-parse phase-a/review-a-5-r1:crates` (a tree hash, which identifies the reviewed crate content independently of history rewrites).
2. **Review request.** btit `team-lead` sends it over ATM to `cobs` (`atm send cobs --team sc-observability …`) for the tag. It covers the crate paths, `docs/mapping.md`, `docs/compatibility.md`, `docs/field-value-dispatch.md`, btit's adoption diff from a-4, and the explicit review items below.
3. **`review-a-5.md`.** Lists every finding with id, reviewer, severity (Blocking / Important / Minor), file:line at the review tag, and disposition (`fixed` or `rejected — reviewer agreed <message ref>`). A `fixed` disposition is proven by a commit on the branch whose message carries the trailer `Review-Finding: R-NNN`, not by a SHA, so it survives the rebase cascade.
4. **Fixes.** Every Blocking and Important finding is fixed, and the full `crates` and btit gate sets are re-run after the last fix. Each Minor finding is either fixed, or recorded in `review-a-5.md` as a follow-up for a-6 or the sc-observability backlog.
5. **Type-placement decision.** `review-a-5.md` records the team's decision and target crate (`sc-observability-types` or the new crates) for each of these item groups, and, for each, whether a same-named item may exist in both crates after a-6 (the `allowed_duplicate_names` list consumed by a-6):
   - tracing-style `Level` constants (`sc_observability_log::Level` vs `sc_observability_types::Level`)
   - `TraceId`/`SpanId` generation
   - `TargetCategory`/`ActionName` sanitizers
   - `LevelFilter` conversions
   - error types: the enum `InitError`/`FlushError`/`ShutdownError` in `sc-observability-log` vs the `error_wrapper!` structs of the same names in `sc-observability-types`
   - dropped-event accounting: `DropCause`/`DroppedEvents` vs `LoggingHealthReport.dropped_events_total`
   - process identity resolution (`ProcessIdentityPolicy` applied in the bridge because the 1.2.0 runtime does not apply it)
6. **Explicit review items.** `review-a-5.md` records the reviewer's position on each of these phase-a decisions, as a finding (any severity) or as an explicit `agreed`:
   - errors are discriminated-union enums with per-variant `code()`/`remediation()`, differing from sc-observability's `error_wrapper!` convention
   - the no-panic lint set (`unwrap_used`, `expect_used`, `panic`, `unreachable`, `todo`, `unimplemented`, `indexing_slicing` = `deny`) and the per-crate `clippy.toml` test allowances, which sc-observability's own crates do not use
   - `catch_unwind` around `Logger::try_log` and helper-thread containment of `flush`/`shutdown`, which exist because sc-observability 1.2.0 has `expect` on internal mutexes and unbounded flush/join
   - `ProcessIdentityPolicy::Auto` resolving `hostname = None` (std has no hostname API)
   - the exact `=` version pin between the two crates and the `__private` semver exemption
   - the tracing compatibility policy: migration from tracing 0.1 is an import rename for every supported form, and the rejected-forms table lists the tracing-valid forms that fail loudly at compile time (`parent:`, `follows_from`, deferred fields via `tracing::field::Empty` or `fields(x)` without a value, the `"" = v` empty key, reserved-prefix keys `sc_observability_log.*` in string and dotted forms, and span macros), plus `Level` ordering
7. **Re-review confirmation.** `cobs` confirms to btit `team-lead` over ATM that no Blocking or Important finding remains open, and the ATM message reference is recorded in `review-a-5.md`.

## Required Work

- If a fix needs a runtime dependency change, record it in `review-a-5.md`, regenerate `crates/runtime-deps.txt`, refresh `src-tauri/Cargo.lock`, and commit all three together.
- Re-run the btit gates, because btit consumes the crates through a path dependency after a-4.
- Findings and dispositions follow the error-enum and no-panic standards; a fix that reintroduces a wrapper-struct error or a panic path is not an acceptable disposition unless the decision in Deliverable 5 or 6 changes the standard and records who agreed.

## Explicit Code Samples

```markdown
<!-- docs/plans/phase-a/review-a-5.md -->
# a-5 review record
review_tag: phase-a/review-a-5-r1
reviewed_crates_tree: <git rev-parse phase-a/review-a-5-r1:crates>
coordination: team-lead@bit ⇄ cobs@sc-observability (ATM)
review_request: <atm message ref>

| id | reported by | severity | file:line (at review_tag) | finding | disposition |
|----|----------|----------|-----------|---------|-------------|
| R-001 | cobs | Important | crates/sc-observability-log/src/bridge.rs:42 | ... | fixed (trailer `Review-Finding: R-001`) |

## Type placement
| item group | decision | target crate | allowed duplicate names | decided by (message ref) |
|------------|----------|--------------|-------------------------|--------------------------|
| Level constants | ... | sc-observability-types / sc-observability-log | `Level` / none | ... |
| error types | ... | ... | `InitError`, `FlushError`, `ShutdownError` / none | ... |

allowed_duplicate_names: [<names from the table>]

## Explicit review items
| item | position (agreed / finding id) | message ref |
|------|-------------------------------|-------------|
| error enums vs error_wrapper! | ... | ... |

## Re-review
confirmation: <message ref>   open_blocking: 0   open_important: 0
```

```text
# commit message trailer for a review fix
fix(log): bound flush on writer stall

Review-Finding: R-001
Co-Authored-By: ...
```

## This Sprint Does Not Close

- Copying the crates into `../sc-observability`, including its CI gates, publish manifest and PR (a-6).
- Moving any type into `sc-observability-types` (a-6, in `../sc-observability`).
- Publishing to crates.io.

## Acceptance Criteria

1. `review-a-5.md` exists, with `review_tag`, `reviewed_crates_tree`, every finding in the table format above, and `open_blocking: 0`, `open_important: 0`. The tag is on `origin`: `git ls-remote --tags origin refs/tags/phase-a/review-a-5-r1` is non-empty and its object id equals the local `git rev-parse refs/tags/phase-a/review-a-5-r1` (the annotated tag object). `git rev-parse phase-a/review-a-5-r1:crates` equals `reviewed_crates_tree`.
2. Every `fixed` disposition `R-NNN` has at least one commit whose message contains `Review-Finding: R-NNN`, reachable from the pushed branch head. The check runs before merge, after `git fetch origin develop feature/sprint-a-5-sc-review`, with the local `HEAD` equal to `origin/feature/sprint-a-5-sc-review`, and `git log -F --grep "Review-Finding: R-NNN" --format=%H origin/develop..origin/feature/sprint-a-5-sc-review` is non-empty for each id (the trailer command in Required Validation).
3. The type-placement table has a decision and an allowed-duplicate-names entry for all seven item groups, and `allowed_duplicate_names` is present.
4. Every explicit review item in Deliverable 6 has a recorded position.
5. The ATM review-request and `cobs` re-review confirmation message references are recorded.
6. Every command in Required Validation passes at the branch head.

## Required Validation

Run from the repo root in bash.

**crates gates**
- `cargo fmt --check --all --manifest-path crates/Cargo.toml`
- `cargo clippy --locked --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings`
- `cargo test --locked --manifest-path crates/Cargo.toml --workspace`
- `cargo tree --locked --manifest-path crates/Cargo.toml -p sc-observability-log -e normal --target all --prefix none --format '{p}' | sed -E 's/ \(.*$//' | LC_ALL=C sort -u | diff - crates/runtime-deps.txt`
- `for f in crates/*/tests/*.rs; do if grep -qE '\binit\(' "$f"; then n=$(grep -cE '#\[([A-Za-z_]+::)*test\b' "$f"); [ "$n" -eq 1 ] || { echo "isolation violation: $f has $n test fns"; exit 1; }; fi; done`
- `cargo tree --locked --manifest-path crates/Cargo.toml -p sc-observability-log-consumer-check -e normal --depth 1 --prefix none --format '{p}' | sed -E 's/ \(.*$//' | diff - <(printf 'sc-observability-log-consumer-check v0.1.0\nsc-observability-log v0.1.0\n')`
- `! grep -rnE 'pub struct [A-Za-z]*Error|allow\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)|expect\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)' crates/*/src`
- `cargo rustdoc --locked --manifest-path crates/Cargo.toml -p sc-observability-log -- -D missing-docs && cargo rustdoc --locked --manifest-path crates/Cargo.toml -p sc-observability-log-macros -- -D missing-docs`
- `cargo +1.94.1 check --locked --manifest-path crates/Cargo.toml --workspace --all-targets`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --manifest-path crates/Cargo.toml --target x86_64-pc-windows-msvc --workspace --all-targets`

**btit gates** (the same set a-4 passed)
- `pnpm test`
- `npx vue-tsc --noEmit`
- `cargo test --locked --manifest-path src-tauri/Cargo.toml`
- `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`

**review anchor and trailers** (run before merge, against the pushed branch)
- `git fetch origin develop feature/sprint-a-5-sc-review && test "$(git rev-parse HEAD)" = "$(git rev-parse origin/feature/sprint-a-5-sc-review)"`
- `test -n "$(git ls-remote --tags origin refs/tags/phase-a/review-a-5-r1)" && test "$(git ls-remote --tags origin refs/tags/phase-a/review-a-5-r1 | cut -f1)" = "$(git rev-parse refs/tags/phase-a/review-a-5-r1)"`
- `test "$(git rev-parse phase-a/review-a-5-r1:crates)" = "$(sed -n 's/^reviewed_crates_tree: //p' docs/plans/phase-a/review-a-5.md)"`
- `for id in $(sed -nE 's/^[|] (R-[0-9]{3}) [|].*[|] fixed .*$/\1/p' docs/plans/phase-a/review-a-5.md); do test -n "$(git log -F --grep "Review-Finding: $id" --format=%H origin/develop..origin/feature/sprint-a-5-sc-review)" || { echo "no Review-Finding commit for $id"; exit 1; }; done`
