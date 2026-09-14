---
id: b-1
title: Root workspace and Tauri crate move to crates/btit-app
status: planned
branch: feature/sprint-b-1-workspace-foundation
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-1-workspace-foundation
target: integrate/phase-b
recommended_model: standard (mechanical move; tooling paths verified in the plan)
dependency_relations:
  - prerequisite: phase-a merge to develop; integrate/phase-b created
    dependent: b-1
    relation: must_follow
    rationale: "b-1 deletes the phase-a crates/ workspace and depends on sc-observability-log published from ../sc-observability, recorded in docs/plans/phase-a/handoff-a-6.md"
  - prerequisite: b-1
    dependent: b-2
    relation: must_follow
    rationale: "btit-types is a member of the root workspace b-1 creates; stack child"
---

# Sprint b-1 — Root workspace and Tauri crate move to `crates/btit-app`

## Recommended Agent / Model

Recommended model: standard (mechanical move; tooling paths verified in the plan).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Create the single Cargo workspace at the repository root and move the Tauri crate from `src-tauri/` to `crates/btit-app/` without changing any Rust source line except the `use`-free manifest and path edits listed below.
- Replace the phase-a path dependency on `crates/sc-observability-log` with the published dependency, and delete the transitional phase-a workspace.
- Update every tool that encodes the old layout: CI, release workflow, `scripts/check_version_sync.py`, `.gitignore`, `rust-toolchain.toml` comments, `package.json` scripts (unchanged in text, verified to work), repo docs.

## Hard Dependencies

- `integrate/phase-b` exists (created from `develop` after the phase-a merge). This sprint's branch is created from `origin/integrate/phase-b`.
- `docs/plans/phase-a/handoff-a-6.md` exists and names the published `sc-observability-log` version, or the merged `../sc-observability` commit (OQ-3). If neither is recorded, this sprint cannot start (hard stop; report to the maintainer).
- OQ-1 (edition) and OQ-2 (package name) answered or defaulted (`2021`, `beads-issue-tracker`).

## Dependency Relations

Trigger definitions, per-branch QA and fix-layer rules: `plan-phase-b.md` "Dependency relations" and "Parallel groups: fork and re-merge".

- phase-a merge → b-1 — `must_follow`: see Hard Dependencies.
- b-1 → b-2 — `must_follow` (b-2 follows b-1): `btit-types` joins the workspace b-1 creates.

Stack: `phase-b-core` · layer 1 (bottom). Merged with `gh stack merge <b-1 PR> --yes` as soon as it passes.

## Exact Targets

Line numbers are at `a18c724`.

- `Cargo.toml` (new, root): `[workspace]`, `[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints]`
- `Cargo.lock` (root): `git mv src-tauri/Cargo.lock Cargo.lock`, then refreshed by `cargo check --workspace`
- `src-tauri/**` → `crates/btit-app/**` by `git mv` (`Cargo.toml`, `build.rs`, `tauri.conf.json`, `capabilities/default.json`, `icons/*`, `src/*.rs`, `.gitignore`)
- `crates/btit-app/Cargo.toml`: `[workspace]` tables removed (lines 5-15 today); `[package]` inherits; `sc-observability-log` dependency (line 38) becomes the published one
- `crates/btit-app/tauri.conf.json`: `build.frontendDist` `../.output/public` → `../../.output/public` (line 6)
- `crates/btit-app/.gitignore`: keep `/gen/schemas`; drop `/target/` (target moves to the root)
- Deleted: `crates/Cargo.toml`, `crates/Cargo.lock`, `crates/runtime-deps.txt`, `crates/sc-observability-log/**`, `crates/sc-observability-log-macros/**`, `crates/sc-observability-log-consumer-check/**`
- `.gitignore`: line 41 `crates/target/` → `/target/`
- `.github/workflows/ci.yml`: `backend` job lines 81-90; `crates` job lines 92-153 removed
- `.github/workflows/release.yml`: lines 89, 109, 133, 143, 171-175, 210-214 (`src-tauri/target/` → `target/`)
- `scripts/check_version_sync.py`: lines 4-14 (docstring), 32-34 (paths)
- `rust-toolchain.toml`: comment lines 1-4
- `CLAUDE.md` lines 44, 51, 67; `.claude/codebase-map.md` lines 16, 208, 420; `docs/attachments.md` line 78; `.sc/repowise/repowise.yaml` lines 8, 12, 21
- `docs/plans/phase-b/sprint-b-1.md` (`status:` frontmatter only)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Root workspace.** `Cargo.toml` at the repository root as in the code sample: `members = ["crates/btit-app"]`, `resolver = "2"`, `[workspace.package]` carrying today's values from `src-tauri/Cargo.toml:8-15` (`version = "1.24.5"`, authors, `license = "MIT"`, `repository = ""`, `edition` per OQ-1, `rust-version = "1.98.1"`), `[workspace.dependencies]` for every dependency `src-tauri/Cargo.toml:31-49` declares today (later sprints append their own), and `[workspace.lints]` equal to `crates/Cargo.toml:31-43`. One `Cargo.lock` at the root, produced by moving today's lockfile and running `cargo check --workspace`, so every third-party version that is not affected by the `sc-observability-log` switch stays as it is (diff of the lockfile reviewed in the PR).
2. **Tauri crate at `crates/btit-app`.** Every file under `src-tauri/` is moved with `git mv` (rename detection ≥ 95% in `git diff -M --stat`). `crates/btit-app/Cargo.toml` keeps `name = "beads-issue-tracker"`, `[lib] name = "app_lib"`, `crate-type = ["staticlib", "cdylib", "rlib"]`, `[build-dependencies] tauri-build`, and inherits `version`, `authors`, `license`, `repository`, `edition`, `rust-version` from the workspace. Every `[dependencies]` entry becomes `name.workspace = true`. It does **not** add `[lints] workspace = true` (b-12 does; the crate has 19 clippy warnings and 283 rustfmt diffs at the baseline). No file under `crates/btit-app/src/` changes content.
3. **`sc-observability-log` from outside btit.** `[workspace.dependencies] sc-observability-log = "<version from handoff-a-6.md>"` (or the git `rev` form, OQ-3). `crates/btit-app/src/logging.rs` compiles unchanged against it: the a-4 API it uses (`init`, `ActionName`, `BridgeOptions`, `DropCause`, `LevelFilter`, `LogGuard`, `LoggerConfig`, `ServiceName`, `logging.rs:26-28`) is the API a-5/a-6 froze. If the published API differs, that is a hard stop, not a local patch.
4. **Phase-a workspace removed.** `crates/Cargo.toml`, `crates/Cargo.lock`, `crates/runtime-deps.txt` and the three `crates/sc-observability-log*` directories are deleted. Nothing else under `crates/` exists except `btit-app`.
5. **`tauri.conf.json`.** Only `build.frontendDist` changes (`../../.output/public`). `productName`, `identifier`, window, CSP, bundle and icon entries are byte-identical.
6. **Frontend scripts unchanged and verified.** `package.json` scripts `tauri:dev` (`cargo tauri dev`) and `tauri:build` (`cargo tauri build`) stay as they are; the developer verifies from the repo root that `pnpm tauri:build` finds `crates/btit-app/tauri.conf.json` (tauri-cli lookup depth 3, plan "Tauri app crate location") and writes bundles under `target/release/bundle/`. Evidence (the CLI's `Found Tauri project` debug line via `cargo tauri build -v`, and the bundle paths) goes into the PR description.
7. **CI.** `backend` job: `Swatinem/rust-cache` without `workspaces:`; `cargo check --workspace --all-targets`; `cargo test --workspace`. The `crates` job (fmt, clippy, runtime graph, isolation contract, rustdoc, MSRV) is removed together with the phase-a crates. `version-sync` and `frontend` jobs unchanged.
8. **Release workflow.** Every `src-tauri/target/` path becomes `target/` (six edits). `tauri-apps/tauri-action@v0` keeps `projectPath` at its default (it globs `**/tauri.conf.json`; plan "Release workflow").
9. **Version script.** `scripts/check_version_sync.py` reads `ROOT/Cargo.toml`, `ROOT/Cargo.lock`, `ROOT/crates/btit-app/tauri.conf.json`; the docstring names the new paths; every check and `--set` keep their semantics. `python3 scripts/check_version_sync.py` prints `version sync OK: app 1.24.5, rust toolchain 1.98.1 (beads-issue-tracker)`.
10. **Docs and comments.** Path references listed in Exact Targets are rewritten to the new locations. No content change beyond paths; `.claude/codebase-map.md` line 16 reads `crates/btit-app/src/` and line 208 heading reads ``## Backend Structure (`crates/btit-app/`)`` (b-12 rewrites the section).
11. **Tests and app behaviour.** `cargo test --workspace` runs the same 142 tests; `pnpm tauri:dev` launches the app and its startup log lines (`[startup] …`, `lib.rs:40-66`) appear in the JSONL log.

## Required Work

- Move order: (1) `git mv src-tauri crates/btit-app`; (2) write the root `Cargo.toml`; (3) edit `crates/btit-app/Cargo.toml`; (4) `git mv crates/btit-app/Cargo.lock Cargo.lock`; (5) delete the phase-a workspace files; (6) `cargo check --workspace`; (7) tooling and docs. Commit (1) separately from the rest so the rename is reviewable.
- `.gitignore`: `/target/` at the root; `crates/btit-app/.gitignore` keeps only `/gen/schemas`.
- Do not run `cargo fmt` or `cargo clippy --fix` on the moved sources (b-12 owns formatting).
- `rust-toolchain.toml` comment: "Pinned Rust toolchain (repo root, applies to every workspace crate and to the Tauri CLI). Keep in sync with `rust-version` in Cargo.toml and the `toolchain:` pins in .github/workflows/*.yml — `python3 scripts/check_version_sync.py` enforces this."
- Changelog lines (collated by b-12): "Rust workspace moved to the repository root; the Tauri crate lives at `crates/btit-app/` (package name unchanged). `sc-observability-log` is consumed from `<crates.io | sc-observability git>` instead of an in-repo copy."

## Explicit Code Samples

```toml
# Cargo.toml (repository root)
[workspace]
members = ["crates/btit-app"]   # b-2..b-6 append btit-types, btit-beads, btit-cli, btit-bd, btit-br
resolver = "2"

[workspace.package]
version = "1.24.5"                       # single source of truth (ADR-003); scripts/check_version_sync.py --set bumps it
authors = ["Laurent Chapin <laurent.chapin@w3dev.fr>"]
license = "MIT"
repository = ""
edition = "2021"                         # OQ-1
rust-version = "1.98.1"                  # equals rust-toolchain.toml channel (ADR-004)

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = { version = "0.4", features = ["kv"] }
sc-observability-log = "<version recorded in docs/plans/phase-a/handoff-a-6.md>"
# OQ-3 fallback: sc-observability-log = { git = "https://github.com/randlee/sc-observability", rev = "<merge commit from handoff-a-6.md>" }
tauri = { version = "2.9.5", features = [] }
tauri-build = { version = "2.5.3", features = [] }
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"
dirs = "6.0"
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
notify = "7.0"
notify-debouncer-mini = "0.5"
dotenvy = "0.15"
tokio = { version = "1", features = ["macros", "rt"] }   # dev-only (logging tests)

[workspace.lints.rust]
missing_debug_implementations = "warn"
unsafe_op_in_unsafe_fn = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
unreachable = "deny"
todo = "deny"
unimplemented = "deny"
indexing_slicing = "deny"
```

```toml
# crates/btit-app/Cargo.toml
[package]
name = "beads-issue-tracker"             # OQ-2
version.workspace = true
description = "Beads Task-Issue Tracker - Desktop app for managing bd Beads issues"
authors.workspace = true
license.workspace = true
repository.workspace = true
edition.workspace = true
rust-version.workspace = true

[lib]
name = "app_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build.workspace = true

[dependencies]
serde_json.workspace = true
serde.workspace = true
log.workspace = true
sc-observability-log.workspace = true
tauri.workspace = true
tauri-plugin-shell.workspace = true
tauri-plugin-dialog.workspace = true
dirs.workspace = true
reqwest.workspace = true
notify.workspace = true
notify-debouncer-mini.workspace = true
dotenvy.workspace = true

[dev-dependencies]
tokio.workspace = true
# no [lints] until b-12
```

```python
# scripts/check_version_sync.py — path constants (lines 32-34 today)
CARGO_TOML = ROOT / "Cargo.toml"
CARGO_LOCK = ROOT / "Cargo.lock"
TAURI_CONF = ROOT / "crates" / "btit-app" / "tauri.conf.json"
```

```yaml
# .github/workflows/ci.yml — backend job steps (replacing lines 81-90)
      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Cargo check
        run: cargo check --workspace --all-targets

      - name: Cargo test
        run: cargo test --workspace
```

## This Sprint Does Not Close

- Any crate other than `btit-app` (b-2 to b-6).
- Formatting, clippy cleanliness or workspace lints on `btit-app` (b-12).
- The `## Backend Structure` rewrite in `.claude/codebase-map.md` and the `CHANGELOG.md` entry (b-12).
- Any behaviour change (b-9, b-10, b-11).

## Acceptance Criteria

1. `src-tauri/` does not exist; `crates/` contains exactly `btit-app/`; `git diff -M --stat origin/integrate/phase-b...HEAD` shows the `src-tauri/**` → `crates/btit-app/**` renames and no content change to any `crates/btit-app/src/*.rs` file (`git diff -M origin/integrate/phase-b...HEAD -- 'crates/btit-app/src/*.rs' | grep -c '^[+-][^+-]'` is `0`).
2. `cargo metadata --format-version 1 --no-deps` reports `workspace_root` = repo root and exactly one package, `beads-issue-tracker 1.24.5` with `rust_version 1.98.1`.
3. `cargo tree -e normal -p beads-issue-tracker --depth 1` lists `sc-observability-log` with a crates.io (or git) source, and no `path` dependency exists in the workspace.
4. `cargo test --workspace` passes 142 tests.
5. `python3 scripts/check_version_sync.py` exits 0 with the OK line from Deliverable 9, and `python3 scripts/check_version_sync.py --set 1.24.5` is idempotent (`git diff --exit-code` after it).
6. `pnpm tauri:build` from the repo root succeeds and produces bundles under `target/release/bundle/` (paths in the PR description); the release workflow's `files:` globs match those paths.
7. `grep -rn 'src-tauri' --include='*.md' --include='*.toml' --include='*.yml' --include='*.yaml' --include='*.py' --include='*.json' . | grep -v node_modules | grep -v '^./docs/plans/' | grep -v '^./docs/crate-split-refactor-issues.md' | grep -v '^./CHANGELOG.md' | grep -v '^./.sc/repowise/data/'` prints nothing.
8. CI is green on the PR (`version-sync`, `frontend` ×3, `backend` ×3); no `crates` job exists.
9. Every command in Required Validation passes.

## Required Validation

- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `python3 scripts/check_version_sync.py`
- `pnpm test`
- `npx vue-tsc --noEmit`
- `pnpm tauri:build` (manual: bundle paths recorded in the PR)
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- the grep from Acceptance Criterion 7
