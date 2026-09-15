---
id: b-9
title: Pinned-behaviour fixes in btit-beads (B1, B4, B5, B7, B11, B13 parts)
status: in_progress
branch: feature/sprint-b-9-beads-domain-fixes
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-9-beads-domain-fixes
target: integrate/phase-b
recommended_model: standard (bounded fixes to pure functions, each pinned by a replaced test)
dependency_relations:
  - prerequisite: b-3
    dependent: b-9
    relation: must_follow
    rationale: "edits the pure functions b-3 moved into btit-beads, behind the API frozen by tests/api_freeze.rs (content dependency); the branch is forked from the b-4 head as a member of group A"
  - prerequisite: b-9
    dependent: b-7
    relation: must_follow
    rationale: "join layer of group A: b-9 is merged into the b-7 branch; b-7 runs the test-preservation gate with b-9's replacement list"
  - prerequisite: none
    parallel_pair: [b-9, b-5]
    relation: parallel_safe
    rationale: "crates/btit-beads/** behind frozen public signatures vs crates/btit-bd/**"
  - prerequisite: none
    parallel_pair: [b-9, b-6]
    relation: parallel_safe
    rationale: "crates/btit-beads/** vs crates/btit-br/**"
---

# Sprint b-9 — Pinned-behaviour fixes in `btit-beads`

## Recommended Agent / Model

Recommended model: standard (bounded fixes to pure functions, each pinned by a replaced test).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Fix the review items from `docs/crate-split-refactor-issues.md` whose code lives in `btit-beads` after b-3: B1, B4, B5, B11, and the B13 parts (relations fixture, out-of-range warning case, blocking dependency types). Handle B7 per OQ-4.
- Replace each pinning test in the same change, by name, so the workspace test-preservation gate stays mechanical.
- Do not change any public signature (`tests/api_freeze.rs` stays byte-identical); b-4..b-8 build against the frozen API.

## Hard Dependencies

- b-3 pushed (content). Forked from the `feature/sprint-b-4-btit-cli` head with the other group A members once b-4's closure criteria are met and QA-1 has no Blocking finding.
- OQ-4 (B7) answered, or defaulted to "doc comment marked unverified, code unchanged".

## Dependency Relations

Trigger definitions, per-branch QA and fix-layer rules: `plan-phase-b.md` "Dependency relations" and "Parallel groups: fork and re-merge".

- b-3 → b-9 — `must_follow` (content).
- b-9 → b-7 — `must_follow` (join layer of group A).
- b-9 ↔ b-5, b-9 ↔ b-6 — `parallel_safe`.

Stack: group A · layer 5 (b-5 | b-6 | b-9, first to close). If this sprint closes first it is linked as layer 5; otherwise it is merged into the b-7 branch when it closes.

## Exact Targets

Line numbers are at `a18c724`; after b-3 the code is in `crates/btit-beads/src/`.

- `crates/btit-beads/src/issues.rs`: `priority_to_number` (`issues.rs:8-15`), `normalize_issue_type` (17-24), `normalize_issue_status` (26-33), `transform_issue` structural set (`issues.rs:61`)
- `crates/btit-beads/src/gates.rs`: `supports_delete_hard_flag_for` (`cli.rs:434-439`), `uses_jsonl_files_for` (392-398), `uses_dolt_backend_for` (453-459), the doc comment of `supports_list_all_flag_for` (`cli.rs:407-412`)
- `crates/btit-beads/src/compat.rs`: `cli_compatibility_warnings` condition `minor >= 50` (`cli.rs:227`)
- Tests (in the `btit-beads` test modules): `normalize_issue_type_defaults_unknown` (`issues.rs:526`), `normalize_issue_status_defaults_unknown` (546), `priority_to_number_defaults_invalid_inputs` (497), `transform_issue_extracts_relations` (649), `supports_delete_hard_flag_table_driven` (`cli.rs:1376`), `uses_jsonl_files_table_driven` (1330), `uses_dolt_backend_table_driven` (1399), `warnings_for_0_50_through_0_56_include_dolt_note` (1089)
- `docs/plans/phase-b/sprint-b-9.md` (`status:` frontmatter and Implementation Notes)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **B1 — unknown types and statuses pass through.** `normalize_issue_type` and `normalize_issue_status` return their input unchanged (bd 1.x built-in types include `decision`, `message`, `molecule`, `gate`, `spike`, `story`, `milestone`, `../beads/internal/types/types.go:710-721`, plus `types.custom`; rewriting them to `task`/`open` on save loses data). Tests: `normalize_issue_type_defaults_unknown` → replaced by `normalize_issue_type_passes_unknown_through` (`"decision"` → `"decision"`, `"custom-x"` → `"custom-x"`); `normalize_issue_status_defaults_unknown` → `normalize_issue_status_passes_unknown_through`; `normalize_issue_type_accepts_valid_types` and `normalize_issue_status_accepts_valid_statuses` kept.
2. **B4 — `--hard` cutoff.** `supports_delete_hard_flag_for(Bd, major, minor, _)` returns `major == 0 && minor < 51` (bd `v0.50.0:cmd/bd/delete.go:63,737` still defines `--hard`; the tombstone system left in 0.51.0). Doc comment updated. Test `supports_delete_hard_flag_table_driven` updated: `(0,50,0)` and `(0,50,3)` → `true`, `(0,51,0)` → `false`.
3. **B5 — JSONL/Dolt cutoff.** `uses_jsonl_files_for(Bd, ..)` → `major == 0 && minor < 51`; `uses_dolt_backend_for(Bd, ..)` → `major > 0 || minor >= 51` (`v0.50.0:internal/storage/factory/factory.go:49` maps an empty backend to SQLite; removed in 0.51.0). Doc comments updated. Tests `uses_jsonl_files_table_driven`, `uses_dolt_backend_table_driven` updated for 0.50.x and 0.51.0 rows. `cli_compatibility_warnings`: the Dolt note condition becomes `minor >= 51`; its message text is unchanged (whether the daemon left in 0.50 or 0.51 is not verified in this sprint and is recorded in Implementation Notes).
4. **B7 — per OQ-4.** Default: the doc comment on `supports_list_all_flag_for` changes from `br: NO` to `br: returns true (unverified against beads_rust; OQ-4)`; code and the br row of `supports_list_all_flag_table_driven` unchanged. If the maintainer confirms br does not support `--all`, the `Br` arm returns `false` and the br row of `supports_list_all_flag_table_driven` flips (plus the literal `capabilities_for_pins_a18c724_values` `(Br, 0.1.33)` row). No other crate changes: b-4/b-6 argv and capability expectations are derived from `capabilities_for`.
5. **B11 — priority parsing.** `priority_to_number`: accepts `p`/`P` case-insensitively; digits `0`–`4` return that digit; anything else (including `p5`–`p9`, which bd rejects: `../beads/internal/validation/bead.go` `ParsePriority` returns `-1` outside 0–4) returns `"3"`, today's default. Test `priority_to_number_defaults_invalid_inputs` replaced by `priority_to_number_is_case_insensitive_and_bounded` (`"P1"` → `"1"`, `"p7"` → `"3"`, `"p"` → `"3"`, `"x"` → `"3"`); `priority_to_number_round_trips_valid_strings` kept.
6. **B13 — relations fixture.** `transform_issue_extracts_relations` uses `relates-to` (`types.go:1225`) instead of the non-existent `related-to`; assertions updated accordingly.
7. **B13 — blocking dependency types.** `transform_issue` treats `conditional-blocks` and `waits-for` (`types.go:1216-1217`) like `blocks`: they feed `blocked_by`/`blocks` and are excluded from `relations` (the `structural_types` set at `issues.rs:61` and the `dep_type == "blocks"` checks at `:166,172,187`). New test `transform_issue_treats_conditional_blocks_and_waits_for_as_blocking` covers both directions (dependencies and dependents).
8. **B13 — out-of-range warning case.** `warnings_for_0_50_through_0_56_include_dolt_note` drops `(0, 99, 0)` and, with Deliverable 3, starts at `(0, 51, 0)`; the message-substring assertions stay.
9. **Frozen API untouched.** `crates/btit-beads/tests/api_freeze.rs` and `crates/btit-beads/src/lib.rs` re-exports are byte-identical to the b-3 branch.

## Required Work

- Replaced-test list (for the test-preservation gate; b-7 applies it, b-11 extends it):

  | Removed name | Replacement name |
  |---|---|
  | `normalize_issue_type_defaults_unknown` | `normalize_issue_type_passes_unknown_through` |
  | `normalize_issue_status_defaults_unknown` | `normalize_issue_status_passes_unknown_through` |
  | `priority_to_number_defaults_invalid_inputs` | `priority_to_number_is_case_insensitive_and_bounded` |
  | (kept, body changed) `transform_issue_extracts_relations`, `supports_delete_hard_flag_table_driven`, `uses_jsonl_files_table_driven`, `uses_dolt_backend_table_driven`, `warnings_for_0_50_through_0_56_include_dolt_note`, `capabilities_for_pins_a18c724_values` (the `(Bd, 0.50.0)` row becomes `{false, true, false, false, true}` with B4/B5; the other rows are unchanged) | same names |
  | (new) | `transform_issue_treats_conditional_blocks_and_waits_for_as_blocking` |

- Each fix is its own commit with the item id in the subject (`fix(beads): B4 …`).
- Changelog lines (collated by b-12): "Unknown issue types and statuses are no longer rewritten to `task`/`open`; `--hard` and the JSONL/Dolt version gates cut over at bd 0.51; `P1`-style priorities are accepted and out-of-range priorities fall back to p3; `conditional-blocks` and `waits-for` dependencies are shown as blockers."

## Explicit Code Samples

```rust
// crates/btit-beads/src/issues.rs (after B1, B11, B13)
pub fn normalize_issue_type(issue_type: &str) -> String { issue_type.to_string() }      // B1: pass through
pub fn normalize_issue_status(status: &str) -> String { status.to_string() }            // B1: pass through

pub fn priority_to_number(priority: &str) -> String {                                    // B11
    let digits = priority.strip_prefix(['p', 'P']).unwrap_or(priority);
    match digits {
        "0" | "1" | "2" | "3" | "4" => digits.to_string(),
        _ => "3".to_string(),
    }
}

/// Dependency types that carry structure (parent-child) or block work (bd types.go:1216-1217).
const STRUCTURAL_TYPES: [&str; 4] = ["blocks", "conditional-blocks", "waits-for", "parent-child"];
const BLOCKING_TYPES: [&str; 3] = ["blocks", "conditional-blocks", "waits-for"];
// transform_issue: `structural_types.contains(..)` uses STRUCTURAL_TYPES; the three `dep_type == "blocks"` checks become `BLOCKING_TYPES.contains(&dep_type.as_str())`.
```

```rust
// crates/btit-beads/src/gates.rs (after B4, B5)
pub fn supports_delete_hard_flag_for(client: CliClient, major: u32, minor: u32, _patch: u32) -> bool {
    match client { CliClient::Bd => major == 0 && minor < 51, _ => false }
}
pub fn uses_jsonl_files_for(client: CliClient, major: u32, minor: u32, _patch: u32) -> bool {
    match client { CliClient::Br => true, CliClient::Bd => major == 0 && minor < 51, CliClient::Unknown => false }
}
pub fn uses_dolt_backend_for(client: CliClient, major: u32, minor: u32, _patch: u32) -> bool {
    match client { CliClient::Br => false, CliClient::Bd => major > 0 || minor >= 51, CliClient::Unknown => false }
}
```

## This Sprint Does Not Close

- B3, B10 (btit-bd part), B13 rename (b-10); B2, B6, B8, B9, B10 (app part), B12, B13 probe cache (b-11).
- Any frontend change (B13 double warning, OQ-5).
- `supports_daemon_flag_for`'s 0.50 cutoff (not a review item; unchanged).

## Acceptance Criteria

1. `git diff --name-only feature/sprint-b-4-btit-cli...HEAD | grep -vE '^(crates/btit-beads/|docs/plans/phase-b/sprint-b-9.md$)'` prints nothing, and `git diff --exit-code feature/sprint-b-4-btit-cli...HEAD -- crates/btit-beads/tests/api_freeze.rs crates/btit-beads/src/lib.rs` is empty.
2. For each of B1, B4, B5, B11 and the three B13 parts, the replaced or new test named in Required Work exists and passes; none of the removed names remains (`! grep -rn 'normalize_issue_type_defaults_unknown\|normalize_issue_status_defaults_unknown\|priority_to_number_defaults_invalid_inputs' crates/`).
3. `cargo test -p btit-beads` passes; `cargo test --workspace` passes.
4. B7 disposition recorded in Implementation Notes (default or maintainer answer) and reflected in the doc comment.
5. `cargo clippy -p btit-beads --all-targets -- -D warnings`, `cargo fmt --check -p btit-beads`, `cargo rustdoc -p btit-beads -- -D missing-docs` pass.
6. QA-1 complete; CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-beads`
- `cargo clippy -p btit-beads --all-targets -- -D warnings`
- `cargo rustdoc -p btit-beads -- -D missing-docs`
- `cargo test --workspace`
- `git diff --exit-code feature/sprint-b-4-btit-cli...HEAD -- crates/btit-beads/tests/api_freeze.rs crates/btit-beads/src/lib.rs`
- `git diff --name-only feature/sprint-b-4-btit-cli...HEAD | grep -vE '^(crates/btit-beads/|docs/plans/phase-b/sprint-b-9.md$)'` prints nothing
- `python3 scripts/check_version_sync.py`
- `git diff --check`

## Implementation Notes

Forked from `feature/sprint-b-4-btit-cli` head `506af12` (unchanged; b-4's closure/QA gate for group A was already satisfied when this worktree was created). All work is contained to `crates/btit-beads/src/{issues,gates,compat}.rs` (bodies, doc comments, tests) plus this file's frontmatter/notes.

**Gate results** (run from the worktree, `IMPLEMENTATION_BASELINE=94e44d3`, `BASELINE_TEST_COUNT=155`):

- `cargo fmt --check -p btit-beads` — pass.
- `cargo clippy -p btit-beads --all-targets -- -D warnings` — pass (one `clippy::doc_markdown` fix: backticked `` `beads_rust` `` in the B7 doc comment).
- `cargo rustdoc -p btit-beads -- -D missing-docs` — pass.
- `cargo test -p btit-beads` — 87 unit tests + 9 `api_freeze` tests pass.
- `cargo test --workspace` — all crates pass (btit-app 70, btit-beads 87+9, btit-bd/br/cli/types, sc-observability-log\* suites, doctests).
- Test-preservation gate (baseline `/tmp/btit-baseline-94e44d3`): baseline list is non-vacuous at exactly 155 names; `comm -23 baseline after` prints exactly the three replaced names this sprint's Required Work table lists — `normalize_issue_type_defaults_unknown`, `normalize_issue_status_defaults_unknown`, `priority_to_number_defaults_invalid_inputs` — and no others. Their replacements (`normalize_issue_type_passes_unknown_through`, `normalize_issue_status_passes_unknown_through`, `priority_to_number_is_case_insensitive_and_bounded`) and the new `transform_issue_treats_conditional_blocks_and_waits_for_as_blocking` all pass. `grep -rn` for the three removed names under `crates/` is clean.
- `git diff --exit-code feature/sprint-b-4-btit-cli...HEAD -- crates/btit-beads/tests/api_freeze.rs crates/btit-beads/src/lib.rs` — empty (frozen API untouched).
- `git diff --name-only feature/sprint-b-4-btit-cli...HEAD | grep -vE '^(crates/btit-beads/|docs/plans/phase-b/sprint-b-9.md$)'` — prints nothing.
- `cargo tree -e normal,features -p beads-issue-tracker | grep -c test-support` — `0`.
- `python3 scripts/check_version_sync.py` — OK (app 1.24.5, toolchain 1.98.1).
- `git diff --check` — clean.
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets` — passes for `crates/btit-beads` and the rest of the workspace; the only warnings are pre-existing `btit-app` lint drift (unused imports/variables in `updates.rs`, `attachments.rs`) tracked in #49, not touched by this sprint.
- `cargo fmt --all` (run once, to confirm no stray formatting) also reformatted several `crates/btit-app/**` files (pre-existing #49 drift); those were reverted with `git checkout -- crates/btit-app/` so this sprint's diff stays scoped to `btit-beads`.

**B7 (OQ-4) disposition:** default taken, per the sprint doc and OQ-4 — `br` source is unavailable locally and the binary is not installed, so the behaviour cannot be verified. The doc comment on `supports_list_all_flag_for` now reads "br: returns true (unverified against beads_rust; OQ-4)"; the code and the `Br` row of `supports_list_all_flag_table_driven` and `capabilities_for_pins_a18c724_values` are unchanged.

**Behaviour deltas (for the b-12 changelog collation):**

- B1: unknown issue types/statuses are no longer rewritten to `task`/`open` — they pass through unchanged, so bd's built-in types (`decision`, `message`, `molecule`, `gate`, `spike`, `story`, `milestone`) and custom types round-trip.
- B4: `bd delete --hard` is now offered for bd 0.50.x too (cutoff moved from `<50` to `<51`), matching bd's actual removal of `--hard` in 0.51.0.
- B5: `uses_jsonl_files`/`uses_dolt_backend` cutover moved from bd 0.50.0 to bd 0.51.0; the `cli_compatibility_warnings` Dolt-note condition moved with it (`minor >= 51`). Whether the underlying daemon removal landed in 0.50 or 0.51 is not independently re-verified in this sprint (carried over uncertainty, noted for the record); the cutover value itself follows the sprint doc's citation of `factory.go:49` / bd 0.51.0.
- B11: `priority_to_number` now accepts `p`/`P` case-insensitively (`"P1"` → `"1"`); any digit outside 0-4 (`p5`-`p9`, which bd's `ParsePriority` rejects) falls back to `"3"`, same as any other unparseable input. A bare digit (`"1"`) now returns that digit instead of `"3"`, matching bd's `ParsePriority` (QA-1 QA-002; pinned in `priority_to_number_is_case_insensitive_and_bounded`).
- B13 (relations fixture): the pinning test now uses bd's real `relates-to` dependency type instead of the nonexistent `related-to`.
- B13 (blocking types): `conditional-blocks` and `waits-for` now behave like `blocks` — they feed `blocked_by`/`blocks` in both directions and are excluded from `relations`, instead of showing up as ordinary relations.
- B13 (out-of-range warning case): the `(0, 99, 0)` case is dropped from `warnings_for_0_50_through_0_56_include_dolt_note` (bd has not shipped past 0.56.x) and the table now starts at `(0, 51, 0)`, matching the B5 cutover; the message-substring assertions are unchanged.

**Deviations from the sprint doc:** none. The sprint doc's exact code samples for `priority_to_number`, `supports_delete_hard_flag_for`, `uses_jsonl_files_for`, `uses_dolt_backend_for`, and the `STRUCTURAL_TYPES`/`BLOCKING_TYPES` constants were implemented verbatim (module-qualified as `crate::issues`/`crate::gates` rather than the plan's illustrative `cli.rs`/`issues.rs` paths, since b-3 already moved this code into `crates/btit-beads/src/`).

**Ambiguity resolved:** the doc's B1 test example gives `"decision"` → `"decision"` and `"custom-x"` → `"custom-x"` for `normalize_issue_type_passes_unknown_through` but does not give explicit values for `normalize_issue_status_passes_unknown_through`. Used the same two values (`"decision"`, `"custom-x"`) for the status test for consistency, since any unrecognized string demonstrates the pass-through behaviour equally well.
