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
    rationale: "b-1 merges the phase-a crates/ workspace into the root workspace and moves src-tauri/; it needs develop@94e44d3 (PR #56: phase-a crates and the a-5 logging.rs)"
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
- Merge the transitional phase-a workspace (`crates/Cargo.toml`, `crates/Cargo.lock`) into the root workspace: the three `sc-observability-log*` crates become root members at their own version `0.1.0`, unchanged in source, and `btit-app` keeps its path dependency on `crates/sc-observability-log` through `[workspace.dependencies]` (OQ-3 resolved: in-tree for all of phase-b).
- Update every tool that encodes the old layout: CI, release workflow, `scripts/check_version_sync.py`, `.gitignore`, `rust-toolchain.toml` comments, `package.json` scripts (unchanged in text, verified to work), repo docs.

## Hard Dependencies

- `integrate/phase-b` exists (created from `develop` after the phase-a merge). This sprint's branch is created from `origin/integrate/phase-b`.
- `origin/integrate/phase-b` contains `develop@94e44d3` (PR #56): the three crates under `crates/sc-observability-log*` and the a-5 `logging.rs`. Nothing from a-6 is needed (`docs/plans/phase-a/handoff-a-6.md` does not exist and is not read).
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
- `crates/btit-app/Cargo.toml`: `[workspace]` tables removed (lines 5-15 today); `[package]` inherits; the `sc-observability-log` path dependency (line 38, `{ path = "../crates/sc-observability-log" }`) becomes `sc-observability-log.workspace = true` (the path now lives in the root `[workspace.dependencies]`)
- `crates/btit-app/tauri.conf.json`: `build.frontendDist` `../.output/public` → `../../.output/public` (line 6)
- `crates/btit-app/.gitignore`: keep `/gen/schemas`; drop `/target/` (target moves to the root)
- Deleted: `crates/Cargo.toml` (its `[workspace.dependencies]` lines 15-30 and `[workspace.lints]` lines 32-43 move to the root manifest) and `crates/Cargo.lock` (60 packages, merged into the root lockfile by `cargo check --workspace`)
- `crates/sc-observability-log/Cargo.toml:3-6`, `crates/sc-observability-log-macros/Cargo.toml:3-6`, `crates/sc-observability-log-consumer-check/Cargo.toml:3-6`: the four `*.workspace = true` keys become explicit `version = "0.1.0"`, `edition = "2024"`, `rust-version = "1.94.1"`, `license = "MIT"` (the values of `crates/Cargo.toml:9-13`); no other line of these crates changes
- `crates/runtime-deps.txt`: regenerated only if the merged lockfile moves a version of a runtime dependency of `sc-observability-log` (crate names unchanged)
- `.gitignore`: line 41 `crates/target/` → `/target/`
- `.github/workflows/ci.yml`: `backend` job lines 81-90; `crates` job lines 92-153 re-pointed at the root (cache line 113, `--manifest-path crates/Cargo.toml` at 116, 119, 122, 132, 144, 145, 153; the isolation glob at 138; `--workspace` at 119, 122, 153 → `-p` selection of the three crates)
- `.github/workflows/release.yml`: lines 89, 109, 133, 143, 171-175, 210-214 (`src-tauri/target/` → `target/`)
- `scripts/check_version_sync.py`: lines 4-14 (docstring), 32-34 (paths), 71-75 (manifest `version.workspace` check) and 86-95 (lockfile check) gain the `INDEPENDENT_VERSION_MEMBERS` exemption; line 117 (OK line)
- `rust-toolchain.toml`: comment lines 1-4
- `CLAUDE.md` lines 44, 51, 67; `.claude/codebase-map.md` lines 16, 208, 420; `docs/attachments.md` line 78; `.sc/repowise/repowise.yaml` lines 8, 12, 21
- `docs/architecture.md`: ADR-009 appended (created with the `origin/docs/adr-initial:docs/architecture.md` header and status legend if PR #42 is not yet merged; plan "Prerequisites")
- `docs/plans/phase-b/sprint-b-1.md` (`status:` frontmatter and Implementation Notes: implementation baseline SHA and re-verified line cites)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Root workspace.** `Cargo.toml` at the repository root as in the code sample: `members = ["crates/btit-app", "crates/sc-observability-log", "crates/sc-observability-log-macros", "crates/sc-observability-log-consumer-check"]`, `resolver = "2"`, `[workspace.package]` carrying today's values from `src-tauri/Cargo.toml:8-15` (`version = "1.24.5"`, authors, `license = "MIT"`, `repository = ""`, `edition` per OQ-1, `rust-version = "1.98.1"`), `[workspace.dependencies]` = every dependency `src-tauri/Cargo.toml:31-49` declares today **plus** every entry of `crates/Cargo.toml:15-30` (`sc-observability`, `sc-observability-types`, `thiserror`, `hostname`, `syn`, `quote`, `proc-macro2`, `tempfile`, `trybuild`, `tracing`, and the `sc-observability-log-macros = { version = "=0.1.0", path = "crates/sc-observability-log-macros" }` pin with its path re-rooted), plus `sc-observability-log = { path = "crates/sc-observability-log" }`; where both manifests declare the same crate the feature lists are the union (`log`: `["std", "kv"]` from `crates/Cargo.toml:18` ∪ `["kv"]` from `src-tauri/Cargo.toml:37`; `tokio`: `["rt", "rt-multi-thread", "macros", "time"]` from `crates/Cargo.toml:29` ∪ `src-tauri/Cargo.toml:49`; `serde`/`serde_json` identical), which is what Cargo's feature unification already produces for one workspace. `[workspace.lints]` equal to `crates/Cargo.toml:32-43`. Later sprints append their own dependencies. One `Cargo.lock` at the root, produced by moving today's app lockfile, deleting `crates/Cargo.lock` and running `cargo check --workspace`: every version already in the app lockfile stays as it is; the 60 packages of `crates/Cargo.lock` that are new to the root lockfile resolve afresh (diff of the lockfile reviewed in the PR; `crates/runtime-deps.txt` regenerated with the `ci.yml:132` command from the root if any of its versions moved, and the name set checked unchanged).
2. **Tauri crate at `crates/btit-app`.** Every file under `src-tauri/` is moved with `git mv` (rename detection ≥ 95% in `git diff -M --stat`). `crates/btit-app/Cargo.toml` keeps `name = "beads-issue-tracker"`, `[lib] name = "app_lib"`, `crate-type = ["staticlib", "cdylib", "rlib"]`, `[build-dependencies] tauri-build`, and inherits `version`, `authors`, `license`, `repository`, `edition`, `rust-version` from the workspace. Every `[dependencies]` entry becomes `name.workspace = true`. It does **not** add `[lints] workspace = true` (b-12 does; the crate has 19 clippy warnings and 283 rustfmt diffs at the baseline). No file under `crates/btit-app/src/` changes content.
3. **`sc-observability-log` in-tree.** `[workspace.dependencies] sc-observability-log = { path = "crates/sc-observability-log" }` and `crates/btit-app/Cargo.toml` `sc-observability-log.workspace = true` — the same path dependency as today (`src-tauri/Cargo.toml:38`), re-rooted. `crates/btit-app/src/logging.rs` and its `logging/` submodules (as merged by PR #56) compile **unchanged** against the in-tree crate: the crate is byte-identical to the one they compiled against at `94e44d3`, so no import changes and no API check are needed. `! git diff --stat 94e44d3...HEAD -- crates/btit-app/src` is part of Acceptance Criterion 1.
4. **Phase-a workspace merged.** `crates/Cargo.toml` and `crates/Cargo.lock` are deleted after their contents move into the root manifest and lockfile (Deliverable 1). The three `crates/sc-observability-log*` crates stay, as root members, with exactly one edit each: the `[package]` keys `version`, `edition`, `rust-version`, `license` (`Cargo.toml:3-6` in each) change from `*.workspace = true` to the explicit values `"0.1.0"`, `"2024"`, `"1.94.1"`, `"MIT"` (`crates/Cargo.toml:9-13`), because the root `[workspace.package]` carries the app's values. Their `[dependencies]`/`[dev-dependencies]` `*.workspace = true` entries and `[lints] workspace = true` resolve against the root tables unchanged; the consumer-check's `sc-observability-log = { path = "../sc-observability-log" }` (`crates/sc-observability-log-consumer-check/Cargo.toml:11`) is untouched. `src/`, `tests/`, `docs/`, `clippy.toml`, `README.md` of the three crates are not modified (frozen for the phase; ownership table). `crates/` then contains exactly `btit-app/`, the three `sc-observability-log*` directories and `runtime-deps.txt`.
5. **`tauri.conf.json`.** Only `build.frontendDist` changes (`../../.output/public`). `productName`, `identifier`, window, CSP, bundle and icon entries are byte-identical.
6. **Frontend scripts unchanged and verified.** `package.json` scripts `tauri:dev` (`cargo tauri dev`) and `tauri:build` (`cargo tauri build`) stay as they are; the developer verifies from the repo root that `pnpm tauri:build` finds `crates/btit-app/tauri.conf.json` (tauri-cli lookup depth 3, plan "Tauri app crate location") and writes bundles under `target/release/bundle/`. Evidence (the CLI's `Found Tauri project` debug line via `cargo tauri build -v`, and the bundle paths) goes into the PR description.
7. **CI.** `backend` job: `Swatinem/rust-cache` without `workspaces:`; `cargo check --workspace --all-targets`; `cargo test --workspace` (now also builds and tests the three in-tree crates). The `crates` job keeps its name, matrix and every phase-a gate, re-pointed at the root as in the code sample: cache without `workspaces:` (`ci.yml:113`); `cargo fmt --check -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check`; `cargo clippy --locked -p … (the three) --all-targets --all-features -- -D warnings`; `cargo test --locked -p … (the three)` (covers `tests/api_freeze.rs`, the trybuild `ui.rs` tests and consumer-check); the runtime-dependency diff without `--manifest-path`; the isolation loop over `crates/sc-observability-log*/tests/*.rs` (the `crates/*/tests/*.rs` glob at `ci.yml:138` would sweep `btit-*` tests from b-2 on); rustdoc `-D missing-docs` for `sc-observability-log` and `-macros`; the MSRV step `cargo +1.94.1 check --locked -p … (the three) --all-targets` (not `--workspace`: `btit-app` requires 1.98.1). `version-sync` and `frontend` jobs unchanged.
8. **Release workflow.** Every `src-tauri/target/` path becomes `target/` (six edits). `tauri-apps/tauri-action@v0` keeps `projectPath` at its default (it globs `**/tauri.conf.json`; plan "Release workflow").
9. **Version script.** `scripts/check_version_sync.py` reads `ROOT/Cargo.toml`, `ROOT/Cargo.lock`, `ROOT/crates/btit-app/tauri.conf.json`; the docstring names the new paths. The three `sc-observability-log*` crates keep `0.1.0` and are exempted as in the code sample: `INDEPENDENT_VERSION_MEMBERS` are skipped by the `version.workspace = true` check (`:71-75`) and must instead carry one identical explicit `[package].version` equal to the `=0.1.0` pin of `[workspace.dependencies].sc-observability-log-macros`; they are skipped by the lockfile check (`:86-95`); `--set` never rewrites them (it edits only `[workspace.package].version`, `package.json` and the lockfile entries of the inheriting packages, `:128-160`). Every other check keeps its semantics. `python3 scripts/check_version_sync.py` prints `version sync OK: app 1.24.5, rust toolchain 1.98.1 (beads-issue-tracker; independent: sc-observability-log, sc-observability-log-macros, sc-observability-log-consumer-check @ 0.1.0)`.
10. **Docs and comments.** Path references listed in Exact Targets are rewritten to the new locations. No content change beyond paths; `.claude/codebase-map.md` line 16 reads `crates/btit-app/src/` and line 208 heading reads ``## Backend Structure (`crates/btit-app/`)`` (b-12 rewrites the section).
11. **Tests and app behaviour.** `cargo test --workspace` runs the same test set as the implementation baseline (142 at `a18c724`; the re-baselined count is recorded in Implementation Notes); `pnpm tauri:dev` launches the app and its startup log lines (`[startup] …`, `lib.rs:40-66`) appear in the JSONL log.
12. **ADR-009.** `docs/architecture.md` gains ADR-009 "Stacked sprint groups: fork and re-merge, per-branch QA-1, fix layers" with the sections the existing ADRs use (Status: Accepted by maintainer direction 2026-09-13; Context: gh-stack linearity verified in phase-a, ADR-005; Decision: the four-step rule, per-branch QA, fix layers, `--rebase-merges` for join layers; Consequences: join layers carry merge commits, downstream layer numbers are group-relative; Links: `plan-phase-b.md` "Parallel groups: fork and re-merge"). The ADR table gets its row. If `docs/architecture.md` is absent on `develop` (PR #42 unmerged), the file is created with the header and status legend of `origin/docs/adr-initial:docs/architecture.md` and only ADR-009; PR #42 rebases onto it.

## Required Work

- **Re-baselining (first task).** Record `develop@<sha>` (the post-phase-a merge commit this branch is created from) as `implementation_baseline` in this doc's Implementation Notes, together with the `generate_handler!` command count and the `cargo test -- --list` test count at that commit; these three values are the inputs of the plan's command-signature gate (`IMPLEMENTATION_BASELINE`), the `generate_handler!` diff and the test-preservation gate in every later sprint. Re-verify every `file:line` cite in `plan-phase-b.md` and `sprint-b-2.md`..`sprint-b-12.md` against that commit (`git grep -n` / `awk 'NR==n'`), and list every drifted cite with its new line number in Implementation Notes; `logging.rs` is expected to drift (PRs #50/#52). Later sprints read the corrected numbers from there; the plan itself is not edited.
- Move order: (1) `git mv src-tauri crates/btit-app`; (2) write the root `Cargo.toml` (app tables + `crates/Cargo.toml:15-43` merged); (3) edit `crates/btit-app/Cargo.toml`; (4) `git mv crates/btit-app/Cargo.lock Cargo.lock`; (5) `git rm crates/Cargo.toml crates/Cargo.lock` and make the four `[package]` keys explicit in the three `sc-observability-log*` manifests; (6) `cargo check --workspace` (merges the lockfile), then the runtime-deps diff from the root; (7) tooling and docs. Commit (1) separately from the rest so the rename is reviewable.
- `.gitignore`: `/target/` at the root; `crates/btit-app/.gitignore` keeps only `/gen/schemas`.
- Do not run `cargo fmt` or `cargo clippy --fix` on the moved sources (b-12 owns formatting).
- `rust-toolchain.toml` comment: "Pinned Rust toolchain (repo root, applies to every workspace crate and to the Tauri CLI). Keep in sync with `rust-version` in Cargo.toml and the `toolchain:` pins in .github/workflows/*.yml — `python3 scripts/check_version_sync.py` enforces this."
- Changelog lines (collated by b-12): "Rust workspace moved to the repository root; the Tauri crate lives at `crates/btit-app/` (package name unchanged). The in-tree `sc-observability-log` crates joined the root workspace at their own version 0.1.0."

## Explicit Code Samples

```toml
# Cargo.toml (repository root)
[workspace]
members = [
    "crates/btit-app",   # b-2..b-6 append btit-types, btit-beads, btit-cli, btit-bd, btit-br
    "crates/sc-observability-log",                 # phase-a crates, in-tree at 0.1.0 (OQ-3 resolved)
    "crates/sc-observability-log-macros",
    "crates/sc-observability-log-consumer-check",
]
resolver = "2"

[workspace.package]
version = "1.24.5"                       # single source of truth (ADR-003); scripts/check_version_sync.py --set bumps it
authors = ["Laurent Chapin <laurent.chapin@w3dev.fr>"]
license = "MIT"
repository = ""
edition = "2021"                         # OQ-1
rust-version = "1.98.1"                  # equals rust-toolchain.toml channel (ADR-004)

[workspace.dependencies]
# app (src-tauri/Cargo.toml:31-49)
serde = { version = "1", features = ["derive"] }
serde_json = "1"
log = { version = "0.4", features = ["std", "kv"] }   # union of crates/Cargo.toml:18 and src-tauri/Cargo.toml:37
sc-observability-log = { path = "crates/sc-observability-log" }   # in-tree for all of phase-b (OQ-3)
tauri = { version = "2.9.5", features = [] }
tauri-build = { version = "2.5.3", features = [] }
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"
dirs = "6.0"
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
notify = "7.0"
notify-debouncer-mini = "0.5"
dotenvy = "0.15"
tokio = { version = "1", features = ["rt", "rt-multi-thread", "macros", "time"] }   # dev-only; union of crates/Cargo.toml:29 and src-tauri/Cargo.toml:49
# phase-a crates (crates/Cargo.toml:15-30, moved verbatim; paths re-rooted)
sc-observability = "1.2.0"
sc-observability-types = "1.2.0"
thiserror = "2"
hostname = "0.4"
syn = { version = "2", features = ["full"] }
quote = "1"
proc-macro2 = "1"
tempfile = "3"
trybuild = "1"   # dev-only: compile-fail UI tests for the rejected macro forms
tracing = "0.1" # dev-only: proves the compatibility fixture is genuine tracing syntax
sc-observability-log-macros = { version = "=0.1.0", path = "crates/sc-observability-log-macros" }

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

```toml
# crates/sc-observability-log/Cargo.toml — the only change (same for -macros and -consumer-check, lines 3-6 each)
[package]
name = "sc-observability-log"
version = "0.1.0"          # was version.workspace = true (crates/Cargo.toml:10); not the app version
edition = "2024"           # was edition.workspace = true (crates/Cargo.toml:11)
rust-version = "1.94.1"    # was rust-version.workspace = true (crates/Cargo.toml:12)
license = "MIT"            # was license.workspace = true (crates/Cargo.toml:13)
# everything below this line is unchanged
```

```python
# scripts/check_version_sync.py — path constants (lines 32-34 today) and the independent-version exemption
CARGO_TOML = ROOT / "Cargo.toml"
CARGO_LOCK = ROOT / "Cargo.lock"
TAURI_CONF = ROOT / "crates" / "btit-app" / "tauri.conf.json"
# Workspace members that keep their own version (in-tree phase-a crates; not bumped with the app).
INDEPENDENT_VERSION_MEMBERS = {"sc-observability-log", "sc-observability-log-macros", "sc-observability-log-consumer-check"}

# in check(): manifest loop (lines 71-75 today)
#   pkg = load_toml(manifest_path)["package"]
#   if pkg["name"] in INDEPENDENT_VERSION_MEMBERS:
#       independent[pkg["name"]] = pkg.get("version")          # must be an explicit semver string
#       continue
#   ... existing `version.workspace = true` check ...
# after the loop:
#   pin = cargo["workspace"]["dependencies"]["sc-observability-log-macros"]["version"]   # "=0.1.0"
#   if len(set(independent.values())) != 1 or f"={next(iter(independent.values()))}" != pin:
#       errors.append(f"{rel(CARGO_TOML)}: independent members must share one explicit version equal to the {pin!r} pin")
# lockfile loop (lines 86-95 today): `for name in packages:` iterates only the inheriting packages (independent ones excluded).
# OK line (line 117): ... f"({', '.join(packages)}; independent: {', '.join(sorted(independent))} @ {next(iter(independent.values()))})"
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

```yaml
# .github/workflows/ci.yml — crates job (lines 92-153), same name/matrix/gates, re-pointed at the root
      - name: Cache cargo
        uses: Swatinem/rust-cache@v2                      # line 113 `workspaces: crates` removed

      - name: Format
        run: cargo fmt --check -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check

      - name: Clippy
        run: cargo clippy --locked -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check --all-targets --all-features -- -D warnings

      - name: Test
        run: cargo test --locked -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check

      - name: Runtime dependency graph                    # line 132 minus --manifest-path
        if: runner.os == 'Linux'
        shell: bash
        env:
          CARGO_TERM_COLOR: never
        run: cargo tree --locked -p sc-observability-log -e normal --target all --prefix none --format '{p}' | sed -E 's/ \(.*$//' | LC_ALL=C sort -u | diff - crates/runtime-deps.txt

      - name: Test-isolation contract                    # line 138: glob narrowed to the phase-a crates
        if: runner.os == 'Linux'
        shell: bash
        run: |
          for f in crates/sc-observability-log*/tests/*.rs; do if grep -qE '\binit\(' "$f"; then n=$(grep -cE '#\[([A-Za-z_]+::)*test\b' "$f"); [ "$n" -eq 1 ] || { echo "isolation violation: $f has $n test fns"; exit 1; }; fi; done

      - name: Rustdoc missing-docs
        if: runner.os == 'Linux'
        shell: bash
        run: |
          cargo rustdoc --locked -p sc-observability-log -- -D missing-docs
          cargo rustdoc --locked -p sc-observability-log-macros -- -D missing-docs

      - name: MSRV check (1.94.1)                         # lines 150-153; -p selection, not --workspace (btit-app needs 1.98.1)
        if: runner.os == 'Linux'
        run: |
          rustup toolchain install 1.94.1 --profile minimal --no-self-update
          cargo +1.94.1 check --locked -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check --all-targets
```

## This Sprint Does Not Close

- Any crate other than `btit-app` (b-2 to b-6).
- Any change to the three `sc-observability-log*` crates beyond the four `[package]` keys; switching `btit-app` to the published crate and deleting the in-tree copies (follow-up after a-6; plan "Not part of phase-b").
- Formatting, clippy cleanliness or workspace lints on `btit-app` (b-12).
- The `## Backend Structure` rewrite in `.claude/codebase-map.md` and the `CHANGELOG.md` entry (b-12).
- Any behaviour change (b-9, b-10, b-11).

## Acceptance Criteria

1. `src-tauri/` does not exist; `crates/` contains exactly `btit-app/`, `sc-observability-log/`, `sc-observability-log-macros/`, `sc-observability-log-consumer-check/` and `runtime-deps.txt` (no `crates/Cargo.toml`, no `crates/Cargo.lock`); `git diff -M --stat origin/integrate/phase-b...HEAD` shows the `src-tauri/**` → `crates/btit-app/**` renames and no content change to any `crates/btit-app/src/*.rs` file (`git diff -M origin/integrate/phase-b...HEAD -- 'crates/btit-app/src/*.rs' | grep -c '^[+-][^+-]'` is `0`); `git diff --stat 94e44d3...HEAD -- 'crates/sc-observability-log*/src' 'crates/sc-observability-log*/tests' 'crates/sc-observability-log*/docs' 'crates/sc-observability-log*/clippy.toml' 'crates/sc-observability-log*/README.md'` is empty, and `git diff 94e44d3...HEAD -- 'crates/sc-observability-log*/Cargo.toml' | grep -E '^[+-][^+-]' | grep -vE '^[+-](version|edition|rust-version|license)'` prints nothing (only the four keys changed).
2. `cargo metadata --format-version 1 --no-deps` reports `workspace_root` = repo root and exactly four packages: `beads-issue-tracker 1.24.5` (`rust_version 1.98.1`, `edition 2021`) and `sc-observability-log`, `sc-observability-log-macros`, `sc-observability-log-consumer-check`, each `0.1.0` (`rust_version 1.94.1`, `edition 2024`).
3. `cargo tree -e normal -p beads-issue-tracker --depth 1` lists `sc-observability-log v0.1.0 (<repo>/crates/sc-observability-log)` (path source); `cargo tree --locked -p sc-observability-log -e normal --target all --prefix none --format '{p}' | sed -E 's/ \(.*$//' | LC_ALL=C sort -u | diff - crates/runtime-deps.txt` is empty, and `diff <(git show 94e44d3:crates/runtime-deps.txt | sed -E 's/ v.*//') <(sed -E 's/ v.*//' crates/runtime-deps.txt)` is empty (versions may move with the merged lockfile, names may not).
4. `cargo test --workspace` passes: the 142 app tests plus the three phase-a crates' tests (their count at `94e44d3` recorded in Implementation Notes, unchanged).
5. `python3 scripts/check_version_sync.py` exits 0 with the OK line from Deliverable 9, and `python3 scripts/check_version_sync.py --set 1.24.5` is idempotent (`git diff --exit-code` after it).
6. `pnpm tauri:build` from the repo root succeeds and produces bundles under `target/release/bundle/` (paths in the PR description); the release workflow's `files:` globs match those paths.
7. `grep -rn 'src-tauri' --include='*.md' --include='*.toml' --include='*.yml' --include='*.yaml' --include='*.py' --include='*.json' . | grep -v node_modules | grep -v '^./docs/plans/' | grep -v '^./docs/crate-split-refactor-issues.md' | grep -v '^./CHANGELOG.md' | grep -v '^./.sc/repowise/data/'` prints nothing.
8. CI is green on the PR (`version-sync`, `frontend` ×3, `backend` ×3, `crates` ×3); the `crates` job runs every step of the code sample, including the MSRV step on `1.94.1`.
9. `grep -c '^### ADR-009' docs/architecture.md` is `1` and the ADR table lists ADR-009; Implementation Notes carry `implementation_baseline: develop@<sha>` and the re-verified cite list.
10. Every command in Required Validation passes.

## Required Validation

- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `python3 scripts/check_version_sync.py`
- `cargo clippy --locked -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check --all-targets --all-features -- -D warnings`
- `cargo test --locked -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check`
- the runtime-dependency diff and the isolation loop from the `crates` job code sample
- `pnpm test`
- `npx vue-tsc --noEmit`
- `pnpm tauri:build` (manual: bundle paths recorded in the PR)
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- the grep from Acceptance Criterion 7
