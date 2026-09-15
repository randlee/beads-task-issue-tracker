---
id: b-12
title: App crate hardening — workspace lints, no panics, fmt/clippy clean, A2; phase docs
status: in_progress
branch: feature/sprint-b-12-app-hardening
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-12-app-hardening
target: integrate/phase-b
recommended_model: standard (mechanical lint fixes across the app crate; documentation collation)
dependency_relations:
  - prerequisite: b-11
    dependent: b-12
    relation: must_follow
    rationale: "the lint rollout touches every app module after the last behaviour fix; b-11 is the join layer of group B, so b-9 and b-10 are already merged below; stack parent"
---

# Sprint b-12 — App crate hardening and phase documentation

## Recommended Agent / Model

Recommended model: standard (mechanical lint fixes across the app crate; documentation collation).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Bring `crates/btit-app` to the standard the other crates already meet: `[lints] workspace = true` (the deny set plus pedantic), no panics in production code, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean, `-D warnings` on the Windows cross-check (closing refactor item A2).
- Publish the phase's documentation: `CHANGELOG.md`, `CLAUDE.md`, `.claude/codebase-map.md` backend section, and the disposition table in `docs/crate-split-refactor-issues.md`.

## Hard Dependencies

- b-11 pushed (with b-8, b-9 and b-10 merged in below it).

## Dependency Relations

Trigger definitions, per-branch QA and fix-layer rules: `plan-phase-b.md` "Dependency relations" and "Parallel groups: fork and re-merge".

- b-11 → b-12 — `must_follow`.

Stack: `phase-b-core` · layer 9 (top). Later fixes to any integrated layer land as `fix/sprint-b-N-<slug>` layers above this one.

## Exact Targets

Line numbers are at `a18c724` (post-b-8 file set: no `cli.rs`, `types.rs`, `issues.rs`, `test_support.rs`).

- `crates/btit-app/Cargo.toml`: `[lints] workspace = true`; `crates/btit-app/clippy.toml` (test allowances)
- `crates/btit-app/src/logging.rs:1-9`: the file-level `#![deny(..)]` becomes redundant and is removed (the workspace lint covers it)
- Production panic sites known at the baseline (clippy `-D warnings` with the deny set finds the complete list): `.lock().unwrap()` at `polling.rs:79,178,209`, `migration.rs:207,237,298`; `.unwrap()` at `issue_commands.rs:111,116,119`; indexing/slicing at `attachments.rs:130-145` (`base64_encode`), `:240` (`sanitize_filename`), `:310` (`issue_short_id`), `:339` (`resolve_duplicate_filename`), `migration.rs:560-562` (`reprefix_id`), `:696-698`; platform-only imports `updates.rs:4`, `attachments.rs:2` (A2); the 19 baseline clippy warnings (`cargo clippy --manifest-path crates/btit-app/Cargo.toml --all-targets`)
- `.github/workflows/ci.yml`: `rust-quality` becomes workspace-wide (`cargo fmt --check --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`; this now also covers the three in-tree `sc-observability-log*` crates, which already pass both in the `crates` job); the `backend` and `crates` jobs unchanged
- `CHANGELOG.md` `[Unreleased]`; `CLAUDE.md` lines 44, 51 (and the "bd Backward Compatibility" bullets) to name `btit-beads`/`btit-bd`; `.claude/codebase-map.md` `## Backend Structure` (lines 208-365), the architecture box (lines 15-19) and the stale `backendMode` row (line 390); `docs/crate-split-refactor-issues.md` (appended disposition table)
- `crates/btit-beads/src/issues.rs`: the bare `"parent-child"` literal in `transform_issue`'s child filter (issue #69; added 2026-09-15 from b-9 QA-1 RBP-F001)
- `docs/plans/phase-b/sprint-b-12.md` (`status:` frontmatter only)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Workspace lints on the app crate.** `crates/btit-app/Cargo.toml` gains `[lints] workspace = true`; `clippy.toml` with the four `allow-*-in-tests` keys; `logging.rs`'s file-level `#![deny(..)]` removed. `cargo clippy -p beads-issue-tracker --all-targets -- -D warnings` passes with **no** `#[allow(clippy::…)]`/`#[expect(clippy::…)]` for the deny set in production code (`! grep -rnE 'allow\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)' crates/btit-app/src`). Pedantic lints may be allowed per item with a one-line justification comment; the PR lists them.
2. **No panics, behaviour preserved.** Each site is rewritten to a non-panicking form with the same result on every input the code reaches today: `PoisonError::into_inner` for locks; `entry().or_insert` or `if let Some(v) = map.get_mut(..)` for `bd_count` counters (`issue_commands.rs:108-122`), keeping the by-type/by-priority semantics; `chunks`/`get`/`iter().nth` for `base64_encode` with a unit test proving identical output against the existing `base64_encode_*` tests (`attachments.rs:650-673`); `split_at`/`rfind`+`get` for `sanitize_filename`, `issue_short_id`, `resolve_duplicate_filename`, `reprefix_id`, and the prefix detection in `bd_migrate_to_dolt`. Every existing test in those modules passes unchanged.
3. **A2 closed.** `use std::process::Command;` removed from `updates.rs:4` and `attachments.rs:2`; the macOS/Linux call sites use `std::process::Command::new(..)` fully qualified (as `attachments.rs:50` already does for Windows). `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets` passes with `RUSTFLAGS="-D warnings"`, and so does the native check on macOS and Linux in CI.
4. **Formatting.** `cargo fmt --all` applied to the app crate in a dedicated commit (`style(app): cargo fmt`, no other change); `cargo fmt --check --all` is a CI gate for the whole workspace.
5. **CI.** `rust-quality` runs `cargo fmt --check --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo rustdoc -p <each library crate> -- -D missing-docs` (not the app), and the per-crate `cargo tree` dependency gates from b-2..b-4 in one Linux step.
6. **`CHANGELOG.md`.** One `[Unreleased]` entry for phase-b collating the "Changelog lines" sections of b-1..b-11 plus this sprint (lint rollout, A2), under `### Changes` (behaviour fixes) and `### Internal` (layout, crates, hardening). The bd-compatibility line stays: `> Requires **bd 1.x**. bd < 1.0 is legacy and triggers a warning. br is supported as a secondary CLI.`
7. **`CLAUDE.md`.** "CLI Policy (bd first)" line 44 points at `check_bd_compatibility` in `crates/btit-app/src/backend.rs` and the warnings in `crates/btit-beads/src/compat.rs`; "bd Backward Compatibility" line 51 says version-gated helpers are `BeadsBackend::capabilities()` backed by the `_for` cores in `crates/btit-beads/src/gates.rs`, and `project_uses_dolt` is `BeadsBackend::project_uses_dolt`; "Code Organization" gains one bullet: Rust crates live under `crates/` and library crates never depend on `tauri` or `sc-observability-log`.
8. **`.claude/codebase-map.md`.** The architecture box names the six crates; `## Backend Structure` is rewritten per crate (files, purpose, line counts at the sprint head), the Tauri command tables keep their 65 rows, "Key Backend Patterns" 1-3 describe the backend slot, `ProjectLocks` and `BeadsBackend::sync`, and "Global State (Rust)" lists `SLOT`, `PROJECT_LOCKS`, `LOGGING_ENABLED`/`VERBOSE_LOGGING` (in `btit-beads`), `LAST_SYNC_TIME`, `LAST_KNOWN_MTIME`, `PROBE_CHILD`, watcher state. The stale "Backend mode … `beads:proj:{hash}:backendMode`" row (line 390; no such key exists in `app/` at `a18c724`) is removed.
9. **`docs/crate-split-refactor-issues.md`.** An appended `## Disposition (phase-b)` table: item → sprint → closing test or doc, matching the plan's issue inventory, with B7 and the B13 double warning marked per OQ-4/OQ-5.
10. **Issue #69 — `"parent-child"` literal.** `crates/btit-beads/src/issues.rs` gains `const PARENT_CHILD: &str = "parent-child";`, used by both `STRUCTURAL_TYPES` and the child filter in `transform_issue`; no behaviour change (existing `transform_issue` tests unchanged and passing); `grep -c '"parent-child"' crates/btit-beads/src/issues.rs` outside `#[cfg(test)]` and comments is 1 (the constant). Closes #69.

## Required Work

- Run `cargo clippy --fix -p beads-issue-tracker --all-targets --allow-dirty` only for the 17 machine-applicable suggestions clippy reports at the baseline, then review the diff; hand-fix the rest.
- Do not change any `Result<_, String>` command signature or error text while fixing lints.
- The formatting commit is separate from the lint commits so `git diff -w` reviews stay small.

## Explicit Code Samples

```toml
# crates/btit-app/Cargo.toml (addition)
[lints]
workspace = true
```

```rust
// crates/btit-app/src/attachments.rs — base64_encode without indexing (behaviour identical to attachments.rs:123-151)
pub(crate) fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let sym = |n: u32| -> char { ALPHABET.get((n & 0x3F) as usize).copied().unwrap_or(b'A') as char }; // `n & 0x3F` < 64: the fallback is unreachable but non-panicking
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let mut it = chunk.iter().copied();
        let (b0, b1, b2) = (it.next().unwrap_or(0), it.next(), it.next());
        let n = (u32::from(b0) << 16) | (u32::from(b1.unwrap_or(0)) << 8) | u32::from(b2.unwrap_or(0));
        out.push(sym(n >> 18));
        out.push(sym(n >> 12));
        out.push(if b1.is_some() { sym(n >> 6) } else { '=' });
        out.push(if b2.is_some() { sym(n) } else { '=' });
    }
    out
}
```

```yaml
# .github/workflows/ci.yml — rust-quality steps (final form)
      - run: cargo fmt --check --all
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings
      - if: runner.os == 'Linux'
        shell: bash
        run: |
          for c in btit-types btit-beads btit-cli btit-bd btit-br; do cargo rustdoc -p "$c" -- -D missing-docs; done
          # per-crate dependency gates from sprints b-2..b-4
```

## This Sprint Does Not Close

- Any behaviour change (all fixes landed in b-9/b-10/b-11).
- `missing_docs` on the app crate.
- The "sc-lint" section (pending maintainer input).

## Acceptance Criteria

1. `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo fmt --check --all` pass on all three CI OSes.
2. No panicking call in app production code: verified by `cargo clippy` with the deny set (Acceptance Criterion 1) plus `! grep -rnE 'allow\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)' crates/btit-app/src`.
3. `RUSTFLAGS="-D warnings" PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets` passes (A2).
4. `cargo test --workspace` passes with the same test set as the b-11 head (no test removed or renamed); the plan's command-signature gate (1) diffs empty and the frontend invoke-subset gate (2) prints nothing (lint fixes changed no command header; any `#[allow(..)]` or doc comment added to a command sits above `#[tauri::command]`, never between it and `fn`, per "Command contract gates").
5. `CHANGELOG.md` `[Unreleased]` contains every "Changelog lines" item from b-1..b-11 and this sprint; `CLAUDE.md` has no `src-tauri` reference and names `crates/btit-beads/src/gates.rs`; `.claude/codebase-map.md` `## Backend Structure` lists six crates and has no `backendMode` row; `docs/crate-split-refactor-issues.md` ends with the disposition table covering A1-A4 and B1-B13.
6. The formatting commit touches only whitespace/layout (`git diff -w --stat <fmt-commit>^ <fmt-commit>` shows no non-whitespace change).
7. QA-1 complete; CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `for c in btit-types btit-beads btit-cli btit-bd btit-br; do cargo rustdoc -p "$c" -- -D missing-docs; done`
- `test "$(cargo tree -e normal,features -p beads-issue-tracker | grep -c 'test-support')" = 0` (the `test-support` features never reach the normal build)
- `RUSTFLAGS="-D warnings" PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `pnpm test`
- `npx vue-tsc --noEmit`
- `python3 scripts/check_version_sync.py`
- `pnpm tauri:build` (manual: the app launches, lists issues, and writes the JSONL log)
- command-signature gate (1) and frontend invoke-subset gate (2) from `plan-phase-b.md` "Command contract gates"
- `git diff --check`

## Implementation Notes

### Panic-site inventory (before → after)

All in `crates/btit-app/src/`, deny-set clean (`cargo clippy -p beads-issue-tracker --all-targets -- -D warnings` finds none) and `! grep -rnE 'allow\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)' crates/btit-app/src` finds nothing.

| Site | Before | After |
| --- | --- | --- |
| `migration.rs` `LAST_SYNC_TIME.lock()` (×3) | `.unwrap()` | `.unwrap_or_else(std::sync::PoisonError::into_inner)` |
| `polling.rs` `LAST_KNOWN_MTIME.lock()` (×3) | `.unwrap()` | `.unwrap_or_else(std::sync::PoisonError::into_inner)` |
| `issue_commands.rs` `bd_count` by-type/by-priority counters | `*by_type.get_mut(&k).unwrap() += 1` | `if let Some(count) = by_type.get_mut(&k) { *count += 1; }` (same by-type/by-priority semantics: a key not in the fixed five-entry map is silently not counted, as today) |
| `issue_commands.rs` `last_updated` comparison | `last_updated.is_none() \|\| issue.updated_at > *last_updated.as_ref().unwrap()` | `last_updated.as_ref().is_none_or(\|last\| issue.updated_at > *last)` |
| `attachments.rs` `base64_encode` | `buf[..chunk.len()]` slice write + `ALPHABET[n as usize & 0x3F]` indexing | the sprint doc's exact `chunks`/`.get(..).unwrap_or(0)`/`ALPHABET.get(..).copied().unwrap_or(b'A')` form; all five existing `base64_encode_*` tests pass unchanged (same output for every input reached today) |
| `attachments.rs` `sanitize_filename` | `filename[..pos]` / `filename[pos+1..]` after `rfind('.')` | `filename.get(..pos).unwrap_or(filename)` / `filename.get(pos+1..).unwrap_or("")` (`'.'` is ASCII, so `pos`/`pos+1` are always char boundaries — behaviourally identical, no panic possible either way) |
| `migration.rs` `reprefix_id` | `&id[..last_dash]` / `&id[last_dash..]` | `id.get(..last_dash)` / `id.get(last_dash..)`, guarded by `if let (Some(_), Some(_))` |
| `migration.rs` `bd_migrate_to_dolt_with` prefix detection (×2 sites: prefix-counting scan, per-issue re-prefix) | `&id[..last_dash]` / `&id[last_dash+1..]` / `&id[last_dash..]` / `&val[old_prefix.len()..]` | `.get(..)`/`.get(..)` pairs, `unwrap_or` fallbacks for the dependency-value slice |
| `migration.rs` `ensure_refs_migrated_v3` JSONL rewrite | `v["external_ref"] = ...` (`serde_json::Value` index-assign) | `if let Some(obj) = v.as_object_mut() { obj.insert(...) }` |
| `migration.rs` `bd_migrate_to_dolt_with` re-prefix / truncate steps | `v.as_object_mut().unwrap()` (×2), `v["external_ref"].as_str().unwrap()` | `if let Some(obj) = v.as_object_mut() { ... }`, `v.get("external_ref").and_then(\|e\| e.as_str()).unwrap_or_default()` |
| `updates.rs` / `attachments.rs` `use std::process::Command` (A2) | platform-gated `use`, unused (and previously stripped by `cargo fix`) on non-matching targets | removed; macOS/Linux/Windows call sites all fully-qualify `std::process::Command::new(..)` / `btit_cli::command::new_command(..)`, matching the existing Windows call site's style |

No panic site changes any `Result<_, String>` shape or error text; every rewrite gives the same result as the code reaches today (verified by the unchanged 106 `btit-app` tests plus the workspace's 412 after-tests).

### Gate outputs

- Command-signature gate (1) vs `develop@94e44d3` (`/tmp/btit-baseline-94e44d3`): `diff` — empty.
- Frontend invoke-subset gate (2): `comm -23` — prints nothing.
- Test-preservation gate: `IMPLEMENTATION_BASELINE=94e44d3`, `BASELINE_TEST_COUNT=155` (baseline list confirmed non-vacuous at exactly 155 lines); replacement lists are the b-9 (3 names), b-10 (4 names) and b-11 (4 names) tables from their own sprint docs, 11 names total after `sort -u`; after-list (workspace, `--all-features`) = 412 lines; `comm -23 <(grep -vxFf replaced.txt baseline-tests.txt) after-tests.txt` — empty. List files kept in the session scratchpad, not `/tmp/replaced.txt`; stderr not suppressed on any gate command.
- `cargo tree -e normal,features -p beads-issue-tracker | grep -c test-support` — `0`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean.
- `cargo fmt --check --all` — clean (after the dedicated `style(app): cargo fmt` commit).
- `RUSTFLAGS="-D warnings" PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets` — clean.
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin clippy --workspace --target x86_64-pc-windows-msvc --all-targets -- -D warnings` — clean after one fix (see Deviations: the Windows/Linux `open_image_file` blocks' `format!` calls, invisible to native macOS clippy since `cfg`-gated code for other targets is stripped on the host).
- `cargo test --workspace` — all green (106 `btit-app` tests unchanged; workspace total 412 after-tests including the phase-a crates).
- `for c in btit-types btit-beads btit-cli btit-bd btit-br; do cargo rustdoc -p "$c" -- -D missing-docs; done` — all five pass (`btit-cli` prints one pre-existing `broken_intra_doc_links` warning, not gated by `-D missing-docs` and not touched by this sprint).
- `pnpm test` — 366 passed (20 files).
- `npx vue-tsc --noEmit` — clean.
- `python3 scripts/check_version_sync.py` — OK (app 1.24.5, toolchain 1.98.1).
- `git diff --check` — clean.
- `pnpm tauri:build` — not run (manual AC, left for the team-lead per the task's hard constraints; the app was not launched).

### Deviations from the sprint doc

- The doc's exact-code sample for `base64_encode` is used verbatim.
- `too_many_lines` on `ensure_refs_migrated_v3` and `bd_migrate_to_dolt_with`, `needless_pass_by_value` on the three `watcher.rs` Tauri commands (`tauri::AppHandle`/`tauri::State` are the required command parameter types), and `unnecessary_wraps` on three test-only helpers (`test_backend.rs::raw_ok`/`raw_fail`/`cwd`, whose Result/Option shape mirrors what their production callers and `reply_raw` expect) are `#[expect(clippy::…, reason = "...")]`, never `#[allow]`. No other pedantic override was needed.
- `cargo xwin clippy` (not just `cargo xwin check`) found one lint the doc's Required Validation list doesn't explicitly run: `clippy::uninlined_format_args` in the `#[cfg(target_os = "windows")]` and `#[cfg(target_os = "linux")]` blocks of `attachments.rs::open_image_file`, invisible to native clippy on macOS because cfg-gated code for other targets is stripped for the host target. Fixed (`format!("...{}", e)` → `format!("...{e}")`); no behaviour change. Recorded here since it is additional evidence for A2 beyond the doc's own gate list.
- `PollData`'s fields were renamed (`open_issues`/`closed_issues`/`ready_issues` → `open`/`closed`/`ready`, clippy's `struct_field_names`) with their `#[serde(rename = "openIssues"/...)]` attributes kept verbatim, so the JSON the frontend receives is unchanged; this is a Rust-internal rename only, not covered by the command-signature gate (which compares header text, not struct bodies) but confirmed not to change `PollData`'s serialized shape.
- Issue #69 (deliverable 10): `crates/btit-beads/src/issues.rs` gained `const PARENT_CHILD: &str = "parent-child"`, used by `STRUCTURAL_TYPES` and `transform_issue`'s child filter; `grep -c '"parent-child"' crates/btit-beads/src/issues.rs` outside `#[cfg(test)]`/comments is 1.

### Ambiguities resolved

- The doc's disposition-table instruction ("append it to `docs/crate-split-refactor-issues.md`") did not specify exact column names or ordering; used `Item | Disposition | Sprint | Closing test or doc`, matching the plan's own issue-inventory table shape, and kept the existing A/B numbering and headings unchanged (no restructuring).
- The nine follow-up issue numbers (#55, #65, #67, #68, #70, #73, #74, #76, #77) and the two upstream `sc-observability` issues are not textually referenced by any phase-b sprint doc except #55 (plan headroom notes) and #67 (`sprint-b-8.md`'s "not the #67 decomposition" aside); the rest are recorded as given, with topic-based (not doc-cited) correspondence noted as "related in theme" rather than "closes", per the "no speculative content" rule.
