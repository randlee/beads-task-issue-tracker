---
id: b-11
title: Pinned-behaviour fixes in the app and btit-cli (B2, B6, B8, B9, B10, B12, B13 probe cache)
status: planned
branch: feature/sprint-b-11-app-cli-fixes
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-11-app-cli-fixes
target: integrate/phase-b
recommended_model: standard (bounded fixes, each pinned by a replaced test; several small test seams)
dependency_relations:
  - prerequisite: b-8
    dependent: b-11
    relation: must_follow
    rationale: "edits app modules b-8 rewires (polling.rs, updates.rs, attachments.rs, attachment_refs.rs); join layer of group B"
  - prerequisite: b-10
    dependent: b-11
    relation: must_follow
    rationale: "join layer of group B: b-10 is merged into this branch; the test-preservation gate applies the b-9, b-10 and b-11 replacement lists"
  - prerequisite: b-11
    dependent: b-12
    relation: must_follow
    rationale: "lint rollout runs after the last behaviour fix"
---

# Sprint b-11 — Pinned-behaviour fixes in the app and `btit-cli`

## Recommended Agent / Model

Recommended model: standard (bounded fixes, each pinned by a replaced test; several small test seams).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Fix the remaining review items from `docs/crate-split-refactor-issues.md`: B2 (`is_real_external_ref`), B6 (mtime test), B8 (attachment folder documentation), B9 (pre-release compare), B10 (platform-dependent update tests), B12 (extension sanitization), B13 (failed-probe caching in `btit-cli`). Replace each pinning test by name.
- Apply the OQ-4 follow-up in `btit-br` if b-9 flipped B7.
- Run the test-preservation gate with the b-9, b-10 and b-11 replacement lists.

## Hard Dependencies

- Group B: b-8 and b-10 pushed (merge-forward); both merged into this branch before closure (PR-completion). This branch is created from the group B first closer's head (layer 7).

## Dependency Relations

Trigger definitions, per-branch QA and fix-layer rules: `plan-phase-b.md` "Dependency relations" and "Parallel groups: fork and re-merge".

- b-8, b-10 → b-11 — `must_follow` (join layer of group B).
- b-11 → b-12 — `must_follow`.

Stack: `phase-b-core` · layer 8.

## Exact Targets

Line numbers are at `a18c724`; paths are the post-b-8 locations.

- `crates/btit-app/src/attachment_refs.rs`: `is_real_external_ref` (12-23); tests 149-181
- `crates/btit-app/src/polling.rs`: `get_beads_mtime` (95-160); test 225-235
- `docs/attachments.md`: lines 10, 22, 31, 46 (`{issue-id}` → `{short-id}`), plus a collision note; `crates/btit-app/src/attachments.rs` tests 793-806 (comment)
- `crates/btit-app/src/updates.rs`: `compare_versions` (130-155), `find_platform_asset` (114-128); tests 424-449, 484-490
- `crates/btit-app/src/attachments.rs`: `sanitize_filename` (237-290); tests 675-713
- `crates/btit-cli/src/runner.rs`: `CliRunner::client_info` failure caching; `crates/btit-app/src/backend.rs`: reset on `replace`/`check_bd_compatibility`
- `crates/btit-br/src/backend.rs` test from b-6 Deliverable 4, only if OQ-4 flipped B7 in b-9
- `docs/plans/phase-b/sprint-b-11.md` (`status:` frontmatter and Implementation Notes)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **B2 — `is_real_external_ref`.** Order of checks: empty → `false`; `cleared:`/`att:` prefixes → `false`; a URL scheme (`^[A-Za-z][A-Za-z0-9+.-]*://`) → `true` before any path check (so `https://redmine.example/attachments/download/1` is real); Windows absolute paths (`^[A-Za-z]:[\\/]` or leading `\\`) → `false`; then today's `/`, `.beads/`, `/attachments/`, `/.beads/` rules. Tests: existing six kept; new `is_real_external_ref_accepts_urls_containing_attachments_segment`, `is_real_external_ref_rejects_windows_absolute_paths` (`C:\proj\.beads\attachments\x.png`, `\\server\share\.beads\x`). `ensure_refs_migrated_v3` and `check_refs_migration` use the same function, so migration v3 keeps real URLs.
2. **B6 — mtime test.** `get_beads_mtime_for(uses_dolt: bool, uses_jsonl: bool, beads_dir) -> Option<SystemTime>` pure core; `get_beads_mtime` passes `backend.project_uses_dolt(dir)` and `capabilities().uses_jsonl_files`. Test `get_beads_mtime_returns_none_without_beads_dir` → replaced by `get_beads_mtime_for_returns_none_for_empty_dir` asserting `None` for both `(false,false)` and `(true,_)` on an empty temp dir; no `bd` spawn.
3. **B8 — attachment folder documentation.** `docs/attachments.md` describes `.beads/attachments/{short-id}/` (short id = text after the last `-`, `attachments.rs:302-313`) and notes the collision case: two issues in one database with different prefixes and the same suffix share a folder and `bd_delete` removes it (`issue_commands.rs:493-504`). `resolve_attachment_dir_uses_short_id` and `resolve_attachment_dir_long_prefix` (`attachments.rs:793-806`) gain a comment naming the collision. Code unchanged (changing the layout needs a data migration; out of scope).
4. **B9 — pre-release compare.** `compare_versions` strips everything from the first `-` in each input before splitting, so `1.2.0-rc.1` parses as `[1, 2, 0]`. Test `compare_versions_with_prerelease_parses_numeric_parts_only` → replaced by `compare_versions_ignores_prerelease_suffix` (`("1.0.0-alpha","1.0.0-beta")` → false; `("1.0.0-alpha","1.1.0-beta")` → true; `("1.2.0-rc.1","1.2.0")` → false; `("1.2.0-rc.1","1.2.1")` → true) with a correct comment. Residual: an RC is still not offered its own final release (they compare equal); recorded in Implementation Notes as the direction the review chose.
5. **B10 — tests that assert nothing off macOS.** `find_platform_asset_for(assets, suffix) -> Option<&GitHubAsset>` pure core and `platform_asset_suffix() -> &'static str`; `find_platform_asset` composes them. Tests `find_platform_asset_matches_macos_arm64`, `find_platform_asset_returns_none_for_no_match` → replaced by `find_platform_asset_for_matches_each_suffix` (all four suffixes) and `find_platform_asset_for_returns_none_without_match`, platform-independent.
6. **B12 — extension sanitization.** `sanitize_filename` applies the stem rules (lowercase, diacritics, unsafe chars → `-`, collapse, trim) to the extension too, keeping the leading `.`. New test `sanitize_filename_sanitizes_extension`: `"a.b<c>"` → `"a.b-c"`, `"Report.MD"` → `"report.md"`, `"x.tar.gz"` → `"x-tar.gz"` (today's stem rule already turns the inner `.` into `-`, `attachments.rs:262`). Existing `sanitize_filename_*` tests kept; `sanitize_filename_fallback_for_empty_stem` asserts the exact output.
7. **B13 — probe caching.** `CliRunner::client_info()` caches a failed probe (a `ProbeState::Failed` marker or `Some(CliProbe { version: None, .. })`) so gated calls do not re-spawn `--version`; `backend::replace` (via `set_cli_binary_path`) builds a fresh runner and `check_bd_compatibility` rebuilds the slot when its fresh probe differs (b-7 Deliverable 4), which is the reset path. Tests in `btit-cli` through a scripted `probe_version_output` seam: one spawn for two `capabilities()` calls after a failed probe; a fresh `CliRunner` probes again.
8. **OQ-4 follow-up.** If b-9 flipped the br `supports_list_all_flag_for` arm, the `btit-br` `capabilities()` test expectation is updated here; otherwise no `btit-br` change.
9. **Group B join and test gate.** b-8 and b-10 are merged into this branch; the test-preservation gate is run with the union of the b-9, b-10 and b-11 replacement lists removed from the baseline and prints nothing.

## Required Work

- Replaced-test list (this sprint's part):

  | Removed name | Replacement name |
  |---|---|
  | `get_beads_mtime_returns_none_without_beads_dir` | `get_beads_mtime_for_returns_none_for_empty_dir` |
  | `compare_versions_with_prerelease_parses_numeric_parts_only` | `compare_versions_ignores_prerelease_suffix` |
  | `find_platform_asset_matches_macos_arm64` | `find_platform_asset_for_matches_each_suffix` |
  | `find_platform_asset_returns_none_for_no_match` | `find_platform_asset_for_returns_none_without_match` |
  | (new) | `is_real_external_ref_accepts_urls_containing_attachments_segment`, `is_real_external_ref_rejects_windows_absolute_paths`, `sanitize_filename_sanitizes_extension`, the `btit-cli` probe-cache tests |

- Each fix is its own commit with the item id in the subject.
- Changelog lines (collated by b-12): "Real external URLs containing `/attachments/` and Windows attachment paths are classified correctly during refs migration; pre-release suffixes no longer skew update checks; attachment filename extensions are sanitized; failed CLI version probes are cached until the binary is changed or compatibility is rechecked."

## Explicit Code Samples

```rust
// crates/btit-app/src/attachment_refs.rs (after B2)
pub(crate) fn is_real_external_ref(r: &str) -> bool {
    let t = r.trim();
    if t.is_empty() || t.starts_with("cleared:") || t.starts_with("att:") { return false; }
    if has_url_scheme(t) { return true; }                       // `[A-Za-z][A-Za-z0-9+.-]*://`
    if is_windows_absolute(t) { return false; }                 // `X:\`, `X:/`, `\\server`
    if t.starts_with('/') || t.starts_with(".beads/") { return false; }
    if t.contains("/attachments/") || t.contains("/.beads/") { return false; }
    true
}
```

```bash
# test-preservation gate with all three replacement lists (plan "Test preservation")
cat /tmp/replaced-b9.txt /tmp/replaced-b10.txt /tmp/replaced-b11.txt | sort -u > /tmp/replaced.txt
comm -23 <(grep -vxFf /tmp/replaced.txt /tmp/baseline-tests.txt) /tmp/after-tests.txt   # must print nothing
```

## This Sprint Does Not Close

- Workspace lints/formatting on the app crate (b-12).
- The attachment folder layout (documentation only).

## Acceptance Criteria

1. Every replacement test in Required Work exists and passes; none of the removed names remains in the workspace.
2. The test-preservation gate with the b-9, b-10 and b-11 lists prints nothing; b-8 and b-10 are merged in (`git branch --contains origin/<member>` lists this branch for the late finisher). Join-layer file discipline: (a) `git diff --exit-code <layer-7 head>...HEAD -- crates/btit-types crates/btit-beads` is empty; (b) b-11's own commits touch only its targets: `git log --first-parent --no-merges --format=%H <layer-7 head>..HEAD | xargs -I{} git show --name-only --format= {} | sort -u | grep -vE '^(crates/btit-app/|crates/btit-cli/|crates/btit-br/src/backend.rs$|docs/attachments.md$|Cargo.lock$|docs/plans/phase-b/sprint-b-11.md$)'` prints nothing.
3. `docs/attachments.md` contains `{short-id}` and the collision note; `grep -c '{issue-id}' docs/attachments.md` is `0`.
4. `cargo test --workspace` passes; `cargo clippy -p btit-cli -p btit-br --all-targets --all-features -- -D warnings` and `cargo fmt --check -p btit-cli -p btit-br` pass.
5. Implementation Notes record the B9 residual and the OQ-4 outcome.
6. QA-1 complete; CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-types -p btit-beads -p btit-cli -p btit-bd -p btit-br`
- `cargo clippy -p btit-cli -p btit-br --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo clippy --manifest-path crates/btit-app/Cargo.toml --all-targets` (no errors; warnings allowed until b-12)
- `pnpm test`
- `npx vue-tsc --noEmit`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- `git diff --exit-code <layer-7 head>...HEAD -- crates/btit-types crates/btit-beads`
- `git log --first-parent --no-merges --format=%H <layer-7 head>..HEAD | xargs -I{} git show --name-only --format= {} | sort -u | grep -vE '^(crates/btit-app/|crates/btit-cli/|crates/btit-br/src/backend.rs$|docs/attachments.md$|Cargo.lock$|docs/plans/phase-b/sprint-b-11.md$)'` prints nothing
- test-preservation gate with the replaced-name lists (Acceptance Criterion 2)
