---
id: b-10
title: App crate hardening — workspace lints, no panics, fmt/clippy clean, A2; phase docs
status: planned
branch: feature/sprint-b-10-app-hardening
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-10-app-hardening
target: integrate/phase-b
recommended_model: standard (mechanical lint fixes across the app crate; documentation collation)
dependency_relations:
  - prerequisite: b-9
    dependent: b-10
    relation: must_follow
    rationale: "the lint rollout touches every app module after the last behaviour fix; stack parent"
---

# Sprint b-10 — App crate hardening and phase documentation

## Recommended Agent / Model

Recommended model: standard (mechanical lint fixes across the app crate; documentation collation).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Bring `crates/btit-app` to the standard the other crates already meet: `[lints] workspace = true` (the deny set plus pedantic), no panics in production code, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean, `-D warnings` on the Windows cross-check (closing refactor item A2).
- Publish the phase's documentation: `CHANGELOG.md`, `CLAUDE.md`, `.claude/codebase-map.md` backend section, and the disposition table in `docs/crate-split-refactor-issues.md`.

## Hard Dependencies

- b-9 pushed.

## Dependency Relations

`must_follow` merge-forward trigger: parent development is pushed, not QA; merge parent → child before every dev/fix round. PR-completion trigger: parent PR merges first. `parallel_safe`: no gate; state non-intersecting ownership.

- b-9 → b-10 — `must_follow`.

Stack: `phase-b-core` · layer 9 (top).

## Exact Targets

Line numbers are at `a18c724` (post-b-7 file set).

- `crates/btit-app/Cargo.toml`: `[lints] workspace = true`; `crates/btit-app/clippy.toml` (test allowances)
- `crates/btit-app/src/logging.rs:1-9`: the file-level `#![deny(..)]` becomes redundant and is removed (the workspace lint covers it)
- Production panic sites known at the baseline (clippy `-D warnings` with the deny set finds the complete list): `.lock().unwrap()` at `polling.rs:79,178,209`, `migration.rs:207,237,298`; `.unwrap()` at `issue_commands.rs:111,116,119`; indexing/slicing at `attachments.rs:130-145` (`base64_encode`), `:240` (`sanitize_filename`), `:310` (`issue_short_id`), `:339` (`resolve_duplicate_filename`), `migration.rs:560-562` (`reprefix_id`), `:696-698`; platform-only imports `updates.rs:4`, `attachments.rs:2` (A2); the 19 baseline clippy warnings (`cargo clippy --manifest-path crates/btit-app/Cargo.toml --all-targets`)
- `.github/workflows/ci.yml`: `rust-quality` becomes workspace-wide (`cargo fmt --check --all`, `cargo clippy --workspace --all-targets -- -D warnings`); the `backend` job unchanged
- `CHANGELOG.md` `[Unreleased]`; `CLAUDE.md` lines 44, 51 (and the "bd Backward Compatibility" bullets) to name `btit-beads`/`btit-bd`; `.claude/codebase-map.md` `## Backend Structure` (lines 208-365) and the architecture box (lines 15-19); `docs/crate-split-refactor-issues.md` (appended disposition table)
- `docs/plans/phase-b/sprint-b-10.md` (`status:` frontmatter only)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Workspace lints on the app crate.** `crates/btit-app/Cargo.toml` gains `[lints] workspace = true`; `clippy.toml` with the four `allow-*-in-tests` keys; `logging.rs`'s file-level `#![deny(..)]` removed. `cargo clippy -p beads-issue-tracker --all-targets -- -D warnings` passes with **no** `#[allow(clippy::…)]`/`#[expect(clippy::…)]` for the deny set in production code (`! grep -rnE 'allow\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)' crates/btit-app/src`). Pedantic lints may be allowed per item with a one-line justification comment; the PR lists them.
2. **No panics, behaviour preserved.** Each site is rewritten to a non-panicking form with the same result on every input the code reaches today: `PoisonError::into_inner` for locks; `entry().or_insert` or `if let Some(v) = map.get_mut(..)` for `bd_count` counters (`issue_commands.rs:108-122`), keeping the by-type/by-priority semantics; `chunks`/`get`/`iter().nth` for `base64_encode` with a unit test proving identical output against the existing `base64_encode_*` tests (`attachments.rs:650-673`); `split_at`/`rfind`+`get` for `sanitize_filename`, `issue_short_id`, `resolve_duplicate_filename`, `reprefix_id`, and the prefix detection in `bd_migrate_to_dolt`. Every existing test in those modules passes unchanged.
3. **A2 closed.** `use std::process::Command;` removed from `updates.rs:4` and `attachments.rs:2`; the macOS/Linux call sites use `std::process::Command::new(..)` fully qualified (as `attachments.rs:50` already does for Windows). `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets` passes with `RUSTFLAGS="-D warnings"`, and so does the native check on macOS and Linux in CI.
4. **Formatting.** `cargo fmt --all` applied to the app crate in a dedicated commit (`style(app): cargo fmt`, no other change); `cargo fmt --check --all` is a CI gate for the whole workspace.
5. **CI.** `rust-quality` runs `cargo fmt --check --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo rustdoc -p <each library crate> -- -D missing-docs` (not the app), and the per-crate `cargo tree` dependency gates from b-2..b-6 in one Linux step.
6. **`CHANGELOG.md`.** One `[Unreleased]` entry for phase-b collating the "Changelog lines" sections of b-1..b-9 plus this sprint (lint rollout, A2), under `### Changes` (behaviour fixes) and `### Internal` (layout, crates, hardening). The bd-compatibility line stays: `> Requires **bd 1.x**. bd < 1.0 is legacy and triggers a warning. br is supported as a secondary CLI.`
7. **`CLAUDE.md`.** "CLI Policy (bd first)" line 44 points at `check_bd_compatibility` in `crates/btit-app/src/backend.rs` and the warnings in `crates/btit-beads/src/compat.rs`; "bd Backward Compatibility" line 51 says version-gated helpers are `BeadsBackend::capabilities()` backed by the `_for` cores in `crates/btit-beads/src/gates.rs`, and `project_uses_dolt` is `BeadsBackend::project_uses_dolt`; "Code Organization" gains one bullet: Rust crates live under `crates/` and library crates never depend on `tauri` or `sc-observability-log`.
8. **`.claude/codebase-map.md`.** The architecture box names the six crates; `## Backend Structure` is rewritten per crate (files, purpose, line counts at the sprint head), the Tauri command tables keep their 65 rows, "Key Backend Patterns" 1-3 describe the backend slot, `ProjectLocks` and `BeadsBackend::sync`, and "Global State (Rust)" lists `SLOT`, `PROJECT_LOCKS`, `LOGGING_ENABLED`/`VERBOSE_LOGGING` (in `btit-beads`), `LAST_SYNC_TIME`, `LAST_KNOWN_MTIME`, `PROBE_CHILD`, watcher state. The stale "Backend mode … `beads:proj:{hash}:backendMode`" row (line 390; no such key exists in `app/` at `a18c724`) is removed.
9. **`docs/crate-split-refactor-issues.md`.** An appended `## Disposition (phase-b)` table: item → sprint → closing test or doc, matching the plan's issue inventory, with B7 and the B13 double warning marked per OQ-4/OQ-5.

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
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - if: runner.os == 'Linux'
        shell: bash
        run: |
          for c in btit-types btit-beads btit-cli btit-bd btit-br; do cargo rustdoc -p "$c" -- -D missing-docs; done
          # per-crate dependency gates from sprints b-2..b-6
```

## This Sprint Does Not Close

- Any behaviour change (all fixes landed in b-8/b-9).
- `missing_docs` on the app crate.
- The "sc-lint" section (pending maintainer input).

## Acceptance Criteria

1. `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check --all` pass on all three CI OSes.
2. `! grep -rnE '\.unwrap\(\)|\.expect\(|panic!|unreachable!|todo!|unimplemented!' $(git ls-files 'crates/btit-app/src/*.rs')` restricted to non-test code: verified by `cargo clippy` with the deny set (Acceptance Criterion 1) plus `! grep -rnE 'allow\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)' crates/btit-app/src`.
3. `RUSTFLAGS="-D warnings" PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets` passes (A2).
4. `cargo test --workspace` passes with the same test set as the b-9 head (no test removed or renamed).
5. `CHANGELOG.md` `[Unreleased]` contains every "Changelog lines" item from b-1..b-9 and this sprint; `CLAUDE.md` has no `src-tauri` reference and names `crates/btit-beads/src/gates.rs`; `.claude/codebase-map.md` `## Backend Structure` lists six crates; `docs/crate-split-refactor-issues.md` ends with the disposition table covering A1-A4 and B1-B13.
6. The formatting commit touches only whitespace/layout (`git diff -w --stat <fmt-commit>^ <fmt-commit>` shows no non-whitespace change).
7. CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `for c in btit-types btit-beads btit-cli btit-bd btit-br; do cargo rustdoc -p "$c" -- -D missing-docs; done`
- `RUSTFLAGS="-D warnings" PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `pnpm test`
- `npx vue-tsc --noEmit`
- `python3 scripts/check_version_sync.py`
- `pnpm tauri:build` (manual: the app launches, lists issues, and writes the JSONL log)
- `git diff --check`
