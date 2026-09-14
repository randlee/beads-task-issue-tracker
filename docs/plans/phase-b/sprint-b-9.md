---
id: b-9
title: Pinned-behaviour fixes in btit-bd, btit-cli and the app (B2, B3, B6, B8, B9, B10, B12, B13 parts)
status: planned
branch: feature/sprint-b-9-storage-app-fixes
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-9-storage-app-fixes
target: integrate/phase-b
recommended_model: higher-effort (B3 redefines Dolt detection against bd's config rules; several test seams)
dependency_relations:
  - prerequisite: b-7
    dependent: b-9
    relation: must_follow
    rationale: "edits app modules b-7 rewires (attachment_refs, polling, updates, attachments) and btit-bd/btit-cli state; stack parent"
  - prerequisite: b-8
    dependent: b-9
    relation: must_follow
    rationale: "PR-completion trigger: the pinned-test replacement list extends b-8's; b-9 also applies any OQ-4 follow-up in crates/btit-br"
  - prerequisite: b-9
    dependent: b-10
    relation: must_follow
    rationale: "lint rollout runs after the last behaviour fix"
---

# Sprint b-9 — Pinned-behaviour fixes in `btit-bd`, `btit-cli` and the app

## Recommended Agent / Model

Recommended model: higher-effort (B3 redefines Dolt detection against bd's config rules; several test seams).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Fix the remaining review items from `docs/crate-split-refactor-issues.md`: B2 (`is_real_external_ref`), B3 (Dolt detection on bd ≥ 0.51), B6 (mtime test), B8 (attachment folder documentation), B9 (pre-release compare), B10 (platform-dependent tests, wrapper-calling test), B12 (extension sanitization), B13 (test rename, failed-probe caching). Replace each pinning test by name.

## Hard Dependencies

- b-7 pushed; b-8 PR merged to `integrate/phase-b` and the stack rebased onto it.

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA; merge parent → child before every dev/fix round. PR-completion trigger: parent PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- b-7 → b-9, b-8 → b-9 (PR merged) — `must_follow`.
- b-9 → b-10 — `must_follow`.

Stack: `phase-b-core` · layer 8.

## Exact Targets

Line numbers are at `a18c724`; paths are the post-b-7 locations.

- `crates/btit-app/src/attachment_refs.rs`: `is_real_external_ref` (12-23); tests 149-181
- `crates/btit-bd/src/dolt.rs`: `project_uses_dolt_for` (from `cli.rs:482-515`); tests from `cli.rs:1239-1302`
- `crates/btit-app/src/polling.rs`: `get_beads_mtime` (95-160); test 225-235
- `docs/attachments.md`: lines 10, 22, 31, 46 (`{issue-id}` → `{short-id}`), plus a collision note; `crates/btit-app/src/attachments.rs` tests 793-806 (comment)
- `crates/btit-app/src/updates.rs`: `compare_versions` (130-155), `find_platform_asset` (114-128); tests 424-449, 484-490
- `crates/btit-app/src/attachments.rs`: `sanitize_filename` (237-290); tests 675-713
- `crates/btit-cli/src/runner.rs`: `CliRunner::client_info` failure caching; `crates/btit-app/src/backend.rs`: reset on `replace`/`check_bd_compatibility`
- `crates/btit-br/src/backend.rs` test from b-6 Deliverable 5, only if OQ-4 flipped B7 in b-8
- `docs/plans/phase-b/sprint-b-9.md` (`status:` frontmatter and Implementation Notes)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **B2 — `is_real_external_ref`.** Order of checks: empty → `false`; `cleared:`/`att:` prefixes → `false`; a URL scheme (`^[A-Za-z][A-Za-z0-9+.-]*://`) → `true` before any path check (so `https://redmine.example/attachments/download/1` is real); Windows absolute paths (`^[A-Za-z]:[\\/]` or leading `\\`) → `false`; then today's `/`, `.beads/`, `/attachments/`, `/.beads/` rules. Tests: existing six kept; new `is_real_external_ref_accepts_urls_containing_attachments_segment`, `is_real_external_ref_rejects_windows_absolute_paths` (`C:\proj\.beads\attachments\x.png`, `\\server\share\.beads\x`). `ensure_refs_migrated_v3` and `check_refs_migration` use the same function, so migration v3 keeps real URLs.
2. **B3 — Dolt detection for bd ≥ 0.51.** `project_uses_dolt_for(info, beads_dir)`: `Br` → `false`; `Bd` with `minor < 50` (major 0) → `false`; `Bd` 0.50.x → today's filesystem probe unchanged (SQLite still existed); `Bd ≥ 0.51`, `Unknown`, and `None` → (a) `beads_dir/.dolt` is a dir → `true` (legacy layout); (b) else read `beads_dir/metadata.json` as JSON: absent or unparsable → `false`; `backend` equal to `sqlite`, `postgres` or `mysql` → `false`; any other value or missing key → `true` (bd's `GetBackend()` rule, `../beads/internal/configfile/configfile.go:297-311`: `postgres`/`mysql`/`sqlite`/registered names return themselves, everything else falls back to Dolt). No `dolt/<name>/.dolt` probe, so `dolt_data_dir` (`configfile.go:36`) and `dolt_mode = server` (`configfile.go:29,326`) projects are detected. `postgres`/`mysql` returning `false` means the SQLite/JSONL paths downstream are taken for them; this is recorded as a known limitation in Implementation Notes (out of scope to model a third backend kind). Tests: `project_uses_dolt_for_nested_layout_needs_metadata_and_dolt_dir` → replaced by `project_uses_dolt_for_bd_1x_reads_metadata_backend` (rows: `{"backend":"dolt"}` → true; `{}` → true; `{"backend":"sqlite"}` → false; `{"backend":"postgres"}` → false; `{"dolt_mode":"server"}` → true; no `metadata.json`, no `.dolt` → false; invalid JSON → false); `project_uses_dolt_for_sqlite_metadata_or_empty_dir_is_false` → replaced by `project_uses_dolt_for_bd_0_50_keeps_filesystem_probe` (today's nested-layout expectations for `(Bd,0,50,x)`); `project_uses_dolt_for_legacy_dolt_dir` kept.
3. **B6 — mtime test.** `get_beads_mtime_for(uses_dolt: bool, uses_jsonl: bool, beads_dir) -> Option<SystemTime>` pure core; `get_beads_mtime` passes `backend.project_uses_dolt(dir)` and `capabilities().uses_jsonl_files`. Test `get_beads_mtime_returns_none_without_beads_dir` → replaced by `get_beads_mtime_for_returns_none_for_empty_dir` asserting `None` for both `(false,false)` and `(true,_)` on an empty temp dir; no `bd` spawn.
4. **B8 — attachment folder documentation.** `docs/attachments.md` describes `.beads/attachments/{short-id}/` (short id = text after the last `-`, `attachments.rs:302-313`) and notes the collision case: two issues in one database with different prefixes and the same suffix share a folder and `bd_delete` removes it (`issue_commands.rs:493-504`). `resolve_attachment_dir_uses_short_id` and `resolve_attachment_dir_long_prefix` (`attachments.rs:793-806`) gain a comment naming the collision. Code unchanged (changing the layout needs a data migration; out of scope).
5. **B9 — pre-release compare.** `compare_versions` strips everything from the first `-` in each input before splitting, so `1.2.0-rc.1` parses as `[1, 2, 0]`. Test `compare_versions_with_prerelease_parses_numeric_parts_only` → replaced by `compare_versions_ignores_prerelease_suffix` (`("1.0.0-alpha","1.0.0-beta")` → false; `("1.0.0-alpha","1.1.0-beta")` → true; `("1.2.0-rc.1","1.2.0")` → false; `("1.2.0-rc.1","1.2.1")` → true) with a correct comment. Residual: an RC is still not offered its own final release (they compare equal); recorded in Implementation Notes as the direction the review chose.
6. **B10 — tests that assert nothing off macOS.** `find_platform_asset_for(assets, suffix) -> Option<&GitHubAsset>` pure core and `platform_asset_suffix() -> &'static str`; `find_platform_asset` composes them. Tests `find_platform_asset_matches_macos_arm64`, `find_platform_asset_returns_none_for_no_match` → replaced by `find_platform_asset_for_matches_each_suffix` (all four suffixes) and `find_platform_asset_for_returns_none_without_match`, platform-independent. `project_uses_dolt_false_without_beads_dir` (moved to `btit-bd` in b-5) → replaced by `project_uses_dolt_for_is_false_for_dir_without_beads_layout` calling the `_for` core with `BD_1`; no `bd` spawn remains in `btit-bd` tests.
7. **B12 — extension sanitization.** `sanitize_filename` applies the stem rules (lowercase, diacritics, unsafe chars → `-`, collapse, trim) to the extension too, keeping the leading `.`. New test `sanitize_filename_sanitizes_extension`: `"a.b<c>"` → `"a.b-c"`, `"Report.MD"` → `"report.md"`, `"x.tar.gz"` → `"x-tar.gz"` (today's stem rule already turns the inner `.` into `-`, `attachments.rs:262`). Existing `sanitize_filename_*` tests kept; `sanitize_filename_fallback_for_empty_stem` asserts the exact output.
8. **B13 — rename and probe caching.** `project_uses_dolt_for_br_and_legacy_bd_never_true` → `project_uses_dolt_for_legacy_dolt_dir_by_client` (same body). `CliRunner::client_info()` caches a failed probe (`Some(CliProbe { version: None, .. })` or a `ProbeState::Failed` marker) so gated calls do not re-spawn `--version`; `backend::replace` (via `set_cli_binary_path`) builds a fresh runner and `check_bd_compatibility` rebuilds the slot when its fresh probe differs (b-7 Deliverable 4), which is the reset path. Tests in `btit-cli`: a recording spawn seam shows one spawn for two `capabilities()` calls after a failed probe, and a fresh `CliRunner` probes again.
9. **OQ-4 follow-up.** If b-8 flipped the br `supports_list_all_flag_for` arm, the `btit-br` `capabilities()` test expectation is updated here; otherwise no `btit-br` change.

## Required Work

- Replaced-test list (extends b-8's; the test-preservation gate is run with both lists applied):

  | Removed name | Replacement name |
  |---|---|
  | `project_uses_dolt_for_nested_layout_needs_metadata_and_dolt_dir` | `project_uses_dolt_for_bd_1x_reads_metadata_backend` |
  | `project_uses_dolt_for_sqlite_metadata_or_empty_dir_is_false` | `project_uses_dolt_for_bd_0_50_keeps_filesystem_probe` |
  | `project_uses_dolt_false_without_beads_dir` | `project_uses_dolt_for_is_false_for_dir_without_beads_layout` |
  | `project_uses_dolt_for_br_and_legacy_bd_never_true` | `project_uses_dolt_for_legacy_dolt_dir_by_client` |
  | `get_beads_mtime_returns_none_without_beads_dir` | `get_beads_mtime_for_returns_none_for_empty_dir` |
  | `compare_versions_with_prerelease_parses_numeric_parts_only` | `compare_versions_ignores_prerelease_suffix` |
  | `find_platform_asset_matches_macos_arm64` | `find_platform_asset_for_matches_each_suffix` |
  | `find_platform_asset_returns_none_for_no_match` | `find_platform_asset_for_returns_none_without_match` |
  | (new) | `is_real_external_ref_accepts_urls_containing_attachments_segment`, `is_real_external_ref_rejects_windows_absolute_paths`, `sanitize_filename_sanitizes_extension`, the `btit-cli` probe-cache tests |

- `metadata.json` parsing uses `serde_json::from_str::<serde_json::Value>`; only the `backend` key is read.
- Each fix is its own commit with the item id in the subject.
- Changelog lines (collated by b-10): "Dolt projects on bd ≥ 0.51 are detected from `metadata.json` (server mode and custom data dirs included); real external URLs containing `/attachments/` and Windows attachment paths are classified correctly during refs migration; pre-release suffixes no longer skew update checks; attachment filename extensions are sanitized; failed CLI version probes are cached until the binary is changed or compatibility is rechecked."

## Explicit Code Samples

```rust
// crates/btit-bd/src/dolt.rs (after B3)
pub fn project_uses_dolt_for(info: Option<(CliClient, u32, u32, u32)>, beads_dir: &Path) -> bool {
    match info {
        Some((CliClient::Br, ..)) => false,
        Some((CliClient::Bd, 0, minor, _)) if minor < 50 => false,
        Some((CliClient::Bd, 0, 50, _)) => legacy_filesystem_probe(beads_dir),   // today's body, cli.rs:490-512
        _ => {
            if beads_dir.join(".dolt").is_dir() { return true; }
            let Ok(text) = std::fs::read_to_string(beads_dir.join("metadata.json")) else { return false; };
            let Ok(meta) = serde_json::from_str::<serde_json::Value>(&text) else { return false; };
            match meta.get("backend").and_then(|b| b.as_str()) {
                Some("sqlite") | Some("postgres") | Some("mysql") => false,   // configfile.go:300-305
                _ => true,                                                      // configfile.go:311: default Dolt
            }
        }
    }
}
```

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

## This Sprint Does Not Close

- Workspace lints/formatting on the app crate (b-10).
- Modelling postgres/mysql as a third storage kind (limitation recorded).
- The attachment folder layout (documentation only).

## Acceptance Criteria

1. Every replacement test in Required Work exists and passes; none of the removed names remains in the workspace.
2. The test-preservation gate, with b-8's and b-9's replaced names removed from the baseline list (`comm -23 <(grep -vxFf /tmp/replaced.txt /tmp/baseline-tests.txt) /tmp/after-tests.txt`), prints nothing.
3. `grep -rn 'Command::new\|probe_cli_binary\|probe_version_output' crates/btit-bd/src` shows no call inside `#[cfg(test)]` code, and `cargo test -p btit-bd` runs without `bd` on `PATH` (`PATH=/usr/bin:/bin cargo test -p btit-bd`).
4. `docs/attachments.md` contains `{short-id}` and the collision note; `grep -c '{issue-id}' docs/attachments.md` is `0`.
5. The B3 table rows from Deliverable 2 are each a test case and pass on all three CI OSes.
6. `cargo test --workspace` passes; `cargo clippy -p btit-cli -p btit-bd -p btit-br --all-targets -- -D warnings` and `cargo fmt --check` for those crates pass.
7. Implementation Notes record the B5 daemon note (from b-8), the B9 residual, the B3 postgres/mysql limitation and the OQ-4 outcome.
8. CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-types -p btit-beads -p btit-cli -p btit-bd -p btit-br`
- `cargo clippy -p btit-cli -p btit-bd -p btit-br --all-targets -- -D warnings`
- `cargo test --workspace`
- `PATH=/usr/bin:/bin cargo test -p btit-bd`
- `cargo clippy --manifest-path crates/btit-app/Cargo.toml --all-targets` (no errors; warnings allowed until b-10)
- `pnpm test`
- `npx vue-tsc --noEmit`
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate with the replaced-name lists (Acceptance Criterion 2)
