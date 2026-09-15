---
id: b-10
title: Dolt detection redesign in btit-bd (B3, DoltMode, B10 wrapper test, B13 rename)
status: planned
branch: feature/sprint-b-10-dolt-detection
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-10-dolt-detection
target: integrate/phase-b
recommended_model: higher-effort (redefines Dolt detection against bd's config rules; verification table with seven cases)
dependency_relations:
  - prerequisite: b-5
    dependent: b-10
    relation: must_follow
    rationale: "rewrites project_uses_dolt_for inside crates/btit-bd (content dependency); the branch is forked from the b-7 head as a member of group B"
  - prerequisite: b-10
    dependent: b-11
    relation: must_follow
    rationale: "join layer of group B: b-10 is merged into the b-11 branch; b-11 runs the test-preservation gate with b-10's replacement list"
  - prerequisite: none
    parallel_pair: [b-10, b-8]
    relation: parallel_safe
    rationale: "crates/btit-bd/** vs crates/btit-app/**; project_uses_dolt_for's signature is frozen by crates/btit-bd/tests/api_freeze.rs"
---

# Sprint b-10 — Dolt detection redesign in `btit-bd`

## Recommended Agent / Model

Recommended model: higher-effort (redefines Dolt detection against bd's config rules; verification table with seven cases).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Close B3: on bd ≥ 0.51 decide Dolt vs not-Dolt from `metadata.json` with bd's own `GetBackend()` rule instead of probing `.beads/dolt/<name>/.dolt`, so server-mode and custom-data-dir projects are detected.
- Add `DoltMode` (embedded / server / proxied-server), resolved from `metadata.json` with bd's rules, as the single switch point for any mode-dependent behaviour (maintainer decision 2026-09-15: one implementation with an enum switch in phase-b; separate embedded and server implementations are a later-phase decision).
- Close the `btit-bd` parts of B10 and B13: the wrapper-calling test spawns no `bd`, and the misleading test name is fixed.

## Hard Dependencies

- b-5 pushed (content). Forked from the `feature/sprint-b-7-backend-slot` head with b-8 once b-7's closure criteria are met and QA-1 has no Blocking finding.

## Dependency Relations

Trigger definitions, per-branch QA and fix-layer rules: `plan-phase-b.md` "Dependency relations" and "Parallel groups: fork and re-merge".

- b-5 → b-10 — `must_follow` (content).
- b-10 → b-11 — `must_follow` (join layer of group B).
- b-10 ↔ b-8 — `parallel_safe`.

Stack: group B · layer 7 (b-8 | b-10, first to close). If this sprint closes first it is linked as layer 7; otherwise it is merged into the b-11 branch when it closes.

## Exact Targets

Line numbers are at `a18c724`; after b-5 the code is in `crates/btit-bd/src/dolt.rs`.

- `crates/btit-bd/src/dolt.rs`: `project_uses_dolt_for` (from `cli.rs:482-515`); tests from `cli.rs:1239-1302`; new `DoltMode` and `project_dolt_mode`
- `crates/btit-bd/tests/dolt_mode.rs` (new: `DoltMode` verification table and signature pin)
- `docs/plans/phase-b/sprint-b-10.md` (`status:` frontmatter and Implementation Notes)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **B3 — Dolt detection for bd ≥ 0.51.** `project_uses_dolt_for(info, beads_dir)`: `Br` → `false`; `Bd` with `minor < 50` (major 0) → `false`; `Bd` 0.50.x → today's filesystem probe unchanged (SQLite still existed); `Bd ≥ 0.51`, `Unknown`, and `None` → (a) `beads_dir/.dolt` is a dir → `true` (legacy layout); (b) else read `beads_dir/metadata.json` as JSON: absent or unparsable → `false`; `backend` equal to `sqlite`, `postgres` or `mysql` → `false`; any other value or missing key → `true` (bd's `GetBackend()` rule, `../beads/internal/configfile/configfile.go:297-311`: `postgres`/`mysql`/`sqlite`/registered names return themselves, everything else falls back to Dolt). No `dolt/<name>/.dolt` probe, so `dolt_data_dir` (`configfile.go:36`) and `dolt_mode = server` (`configfile.go:29,326`) projects are detected. `postgres`/`mysql` returning `false` means the SQLite/JSONL paths downstream are taken for them; this is recorded as a known limitation in Implementation Notes (out of scope to model a third backend kind).
2. **Verification table** (each row is a test case in `project_uses_dolt_for_bd_1x_reads_metadata_backend`, with `BD_1 = Some((Bd, 1, 0, 4))`):

   | `metadata.json` | `.dolt` dir | expected |
   |---|---|---|
   | `{"backend":"dolt"}` | no | `true` |
   | `{}` | no | `true` |
   | `{"dolt_mode":"server"}` | no | `true` |
   | `{"backend":"sqlite"}` | no | `false` |
   | `{"backend":"postgres"}` | no | `false` |
   | absent | no | `false` |
   | `not json` | no | `false` |
   | absent | yes | `true` |
   | `{"backend":"dolt"}` with `info = None` (no probe) | no | `true` (deliberate deviation, see below) |
   | `{"backend":"dolt"}` with `info = Some((Unknown, 9, 9, 9))` | no | `true` (same) |

   **Deliberate deviation (recorded in Implementation Notes and the PR).** Today the `_ =>` arm (`cli.rs:489`) applies the filesystem probe to `Unknown` and `None` too, so those cases already follow the bd-≥-0.50 rule; this sprint keeps them on the bd-≥-0.51 metadata rule (the `_ =>` arm), widening it from "`.dolt` dir or `metadata.json` + `dolt/<name>/.dolt`" to "`.dolt` dir or `metadata.json` backend rule". The alternative (treat `Unknown`/`None` as non-Dolt) would flip today's behaviour for an unprobed bd 1.x project, which is worse for the common case; b-7's factory builds `BdCli` for `Unknown`/no-probe binaries precisely because they share bd's arms today.

3. **Tests replaced.** `project_uses_dolt_for_nested_layout_needs_metadata_and_dolt_dir` → `project_uses_dolt_for_bd_1x_reads_metadata_backend` (the table above); `project_uses_dolt_for_sqlite_metadata_or_empty_dir_is_false` → `project_uses_dolt_for_bd_0_50_keeps_filesystem_probe` (today's nested-layout expectations for `(Bd, 0, 50, x)`); `project_uses_dolt_for_legacy_dolt_dir` kept.
4. **B10 (btit-bd part).** `project_uses_dolt_false_without_beads_dir` → `project_uses_dolt_for_is_false_for_dir_without_beads_layout`, calling the `_for` core with `BD_1`; no `bd` spawn remains in `btit-bd` tests.
5. **B13 rename.** `project_uses_dolt_for_br_and_legacy_bd_never_true` → `project_uses_dolt_for_legacy_dolt_dir_by_client` (same body).
6. **`DoltMode` (maintainer decision 2026-09-15).** In `crates/btit-bd/src/dolt.rs`, a `#[non_exhaustive]` `pub enum DoltMode { Embedded, Server, ProxiedServer }` and `pub fn project_dolt_mode(beads_dir: &Path) -> Option<DoltMode>`, following bd's `GetDoltMode()` at `../beads` `610339cd7` (`internal/configfile/configfile.go`: constants 325-327, `GetDoltMode` 471-479, `HostImpliesServerMode` 417-437, `IsLocalHostString` 444-450):
   - `metadata.json` absent or unparsable, or its `backend` is `sqlite`/`postgres`/`mysql` → `None` (not a Dolt project; same backend rule as deliverable 1).
   - `dolt_mode` non-empty, compared case-insensitively: `server` → `Server`; `proxied-server` → `ProxiedServer`; `embedded` or any other value → `Embedded` (bd's `IsDoltServerMode` treats every non-`server` explicit value as not-server).
   - `dolt_mode` missing or empty: `dolt_server_host` non-empty and not local (`""`, `localhost`, `127.0.0.1`, `::1`, `[::1]`, `0.0.0.0` after trim and lowercase) → `Server`; otherwise → `Embedded`.
   - **Known limitation (recorded in Implementation Notes):** the runtime overrides `BEADS_DOLT_SERVER_MODE`, `BEADS_DOLT_SHARED_SERVER`, `BEADS_DOLT_SERVER_HOST` and the `config.yaml` `dolt.mode`/`dolt.host` fallbacks are not consulted; `project_dolt_mode` reports the project-local persisted mode only. A `dolt-server.port` file or a running `dolt sql-server` process is never evidence of server mode (maintainer's vault: embedded with a stale port file; beads-ralph: embedded with a leftover server process).
   - **Switch rule.** Mode-dependent behaviour in phase-b is written as a `match` on `DoltMode` inside `btit-bd`; no second trait or implementation per mode. Phase-b adds no app caller (nothing in today's behaviour differs by mode); the enum is the switch point for later work (Dolt server settings / database listing view, issue #51 SQL access).
7. **`DoltMode` verification table** (each row a case in `crates/btit-bd/tests/dolt_mode.rs::project_dolt_mode_follows_bd_get_dolt_mode`, using a temp `.beads` dir; the file also pins `let _: fn(&Path) -> Option<DoltMode> = project_dolt_mode;`):

   | `metadata.json` | extra files | expected |
   |---|---|---|
   | `{"backend":"dolt","dolt_mode":"embedded","dolt_database":"iron"}` | `dolt-server.port` containing `3308`, `embeddeddolt/iron/` dir | `Some(Embedded)` |
   | `{"backend":"dolt","dolt_mode":"server"}` | none | `Some(Server)` |
   | `{"backend":"dolt","dolt_mode":"SERVER"}` | none | `Some(Server)` |
   | `{"backend":"dolt","dolt_mode":"proxied-server"}` | none | `Some(ProxiedServer)` |
   | `{"backend":"dolt","dolt_mode":"bogus"}` | none | `Some(Embedded)` |
   | `{"backend":"dolt"}` | none | `Some(Embedded)` |
   | `{}` | none | `Some(Embedded)` |
   | `{"backend":"dolt","dolt_server_host":"db.example.com"}` | none | `Some(Server)` |
   | `{"backend":"dolt","dolt_server_host":"127.0.0.1"}` | none | `Some(Embedded)` |
   | `{"backend":"dolt","dolt_mode":"embedded","dolt_server_host":"db.example.com"}` | none | `Some(Embedded)` |
   | `{"backend":"sqlite","dolt_mode":"server"}` | none | `None` |
   | absent | none | `None` |
   | `not json` | none | `None` |

8. **Frozen API untouched.** `crates/btit-bd/tests/api_freeze.rs` is byte-identical to the b-5 branch (the signature `fn project_uses_dolt_for(Option<(CliClient, u32, u32, u32)>, &Path) -> bool` is unchanged).

## Required Work

- Replaced-test list (b-11 applies it together with b-9's and its own):

  | Removed name | Replacement name |
  |---|---|
  | `project_uses_dolt_for_nested_layout_needs_metadata_and_dolt_dir` | `project_uses_dolt_for_bd_1x_reads_metadata_backend` |
  | `project_uses_dolt_for_sqlite_metadata_or_empty_dir_is_false` | `project_uses_dolt_for_bd_0_50_keeps_filesystem_probe` |
  | `project_uses_dolt_false_without_beads_dir` | `project_uses_dolt_for_is_false_for_dir_without_beads_layout` |
  | `project_uses_dolt_for_br_and_legacy_bd_never_true` | `project_uses_dolt_for_legacy_dolt_dir_by_client` |

- `metadata.json` parsing uses `serde_json::from_str::<serde_json::Value>`; `project_uses_dolt_for` reads only the `backend` key; `project_dolt_mode` reads `backend`, `dolt_mode` and `dolt_server_host`.
- Changelog lines (collated by b-12): "Dolt projects on bd ≥ 0.51 are detected from `metadata.json` (server mode and custom data dirs included)."

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

## This Sprint Does Not Close

- Modelling postgres/mysql as a third storage kind (limitation recorded).
- Separate embedded and server implementations, env-var/`config.yaml` mode overrides, server discovery, any `DoltMode` app caller or UI (later phase).
- Any app-side change (b-8, b-11).

## Acceptance Criteria

1. `git diff --name-only feature/sprint-b-7-backend-slot...HEAD | grep -vE '^(crates/btit-bd/|docs/plans/phase-b/sprint-b-10.md$)'` prints nothing (group B non-intersection); `git diff --exit-code feature/sprint-b-7-backend-slot...HEAD -- crates/btit-bd/tests/api_freeze.rs` is empty.
2. Every row of the verification table is a test case and passes on all three CI OSes; the replacement tests in Required Work exist and the removed names do not (`! grep -rn 'project_uses_dolt_for_nested_layout\|project_uses_dolt_for_sqlite_metadata\|project_uses_dolt_false_without_beads_dir\|never_true' crates/`).
3. No test in `btit-bd` spawns a CLI, checked mechanically on every OS: `! grep -rnE 'BdCli::new\(|BdCli::with_seeded_probe\(|Command::new|probe_cli_binary|probe_version_output' crates/btit-bd/tests` and, for in-file `#[cfg(test)]` modules, `awk '/#\[cfg\(test\)\]/{t=1} t&&/(BdCli::new\(|Command::new|probe_cli_binary|probe_version_output)/{print FILENAME":"FNR": "$0}' crates/btit-bd/src/*.rs` prints nothing. On Linux and macOS only, additionally `SAFE_PATH="$(dirname "$(command -v cargo)"):/usr/bin:/bin"; ! PATH="$SAFE_PATH" command -v bd && PATH="$SAFE_PATH" cargo test -p btit-bd` passes with no `bd` reachable (skipped on Windows, where the greps are the gate).
4. `cargo clippy -p btit-bd --all-targets -- -D warnings`, `cargo fmt --check -p btit-bd`, `cargo rustdoc -p btit-bd -- -D missing-docs` pass.
5. Implementation Notes record the postgres/mysql limitation and the `DoltMode` env-var/`config.yaml` limitation.
5a. Every row of the `DoltMode` verification table passes on all three CI OSes; `grep -n 'pub enum DoltMode' crates/btit-bd/src/dolt.rs` matches once and the enum carries `#[non_exhaustive]`.
6. QA-1 complete; CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-bd`
- `cargo clippy -p btit-bd --all-targets -- -D warnings`
- `cargo rustdoc -p btit-bd -- -D missing-docs`
- `cargo test --workspace`
- the no-spawn greps from Acceptance Criterion 3 (all OSes) and, on Linux/macOS, `SAFE_PATH="$(dirname "$(command -v cargo)"):/usr/bin:/bin"; ! PATH="$SAFE_PATH" command -v bd && PATH="$SAFE_PATH" cargo test -p btit-bd`
- `git diff --name-only feature/sprint-b-7-backend-slot...HEAD | grep -vE '^(crates/btit-bd/|docs/plans/phase-b/sprint-b-10.md$)'` prints nothing
- `python3 scripts/check_version_sync.py`
- `git diff --check`
