# Crate Split Refactor — Review Issues

Findings from the critical review of the `src-tauri/src/lib.rs` refactor (stack #31) that must be carried into the next refactor.

| Layer | PR | Branch | Content |
|---|---|---|---|
| L1 | #29 | `refactor/lib-rs-tests` | Characterization tests; pure `_for` cores for version gates |
| L2 | #30 | `refactor/lib-rs-split` | Move `lib.rs` into 15 modules |
| L3 | #32 | `refactor/lib-rs-cleanup` | Pure `project_uses_dolt_for` core, codebase map |

Line numbers refer to `src-tauri/src/` at `f6a0db3` (top of the stack). bd source references are to `../beads` (steveyegge/beads).

The review had two questions:

- **A.** Does any code behave differently than before the refactor?
- **B.** Do the new tests lock in questionable pre-existing behavior?

Every item below was checked against the code. Items not independently confirmed are marked **unverified**.

---

## A. Differences introduced by the refactor

### A1. Log target changed (observable in log text only)

- **Before:** every log call lived in the crate root, so log lines showed `[app_lib]`.
- **After:** `log_info!`/`log_warn!`/`log_error!` (`logging.rs`) and direct `log::*!` calls expand in the calling module. Lines now show `[app_lib::cli]`, `[app_lib::watcher]`, `[app_lib::logging]` (for `log_frontend`), and so on. `run()` still logs as `app_lib`.
- **Impact:** no functional consumer. The logger has no `.filter`/`level_for`, `app/` never references `app_lib`, and `DebugPanel.vue` only parses the `[LEVEL]`/`[context]` tags.
- **To restore identical output:** pass `target: "app_lib"` in the three macros and in the direct `log::` calls.

### A2. Imports used only inside platform `cfg` blocks

Each import below is unused on some targets. Running `cargo fix` on those targets deletes the import and breaks the others. The Windows break fixed in #30 (`crate::cli::new_command` in `attachments.rs`) was the same kind of bug.

Confirmed with `cargo xwin check --target x86_64-pc-windows-msvc --all-targets`:

| File | Import | Used only in | Unused on |
|---|---|---|---|
| `logging.rs:1` | `use std::env;` | `#[cfg(target_os = "macos")]` | Linux, Windows |
| `updates.rs:4` | `use std::process::Command;` | `#[cfg(target_os = "macos")]` | Linux, Windows |
| `attachments.rs:2` | `use std::process::Command;` | macOS and Linux blocks | Windows |

A related warning also appears: `updates.rs:443` has an unused `result` on non-macOS (see B10).

- **Fix:** fully qualify at the call site, or move the `use` inside the `cfg` block.
- **CI:** CI does not use `-D warnings`, so these never fail the build.

Windows cross-check from macOS (needs the Homebrew LLVM tools on `PATH`):

```bash
PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --target x86_64-pc-windows-msvc --all-targets
```

### A3. L1/L3 wrapper + pure-core extraction

`supports_daemon_flag`, `uses_jsonl_files`, `supports_list_all_flag`, `supports_delete_hard_flag`, `uses_dolt_backend` and `project_uses_dolt` were each split into a wrapper plus a pure `_for` core so they can be tested without spawning `bd`.

The review compared them against `develop` and found them result-equivalent for every input:

- same match arms and version comparisons
- same `None`/`Unknown` defaults
- one `get_cli_client_info()` call per wrapper
- version probe still runs before any filesystem access

This is an implementation change, not a behavior change.

### A4. Misplaced comments and visibility

- **Orphaned doc comment:** the doc comment for `ensure_refs_migrated_v3` was left at `cli.rs:582-583`, where it now sits above `CompatibilityInfo`. The function (`migration.rs:14`) has no doc comment.
- **Moved comment:** the "Cached CLI client info" comment moved from `enum CliClient` to `static CLI_CLIENT_INFO` (`cli.rs:17-18`).
- **Visibility:** 23 structs that were `pub` at the crate root are now in private modules. The only external consumer is `main.rs` (`app_lib::run()`).

### Verified unchanged by the split (L2)

- **Items:** all 334 match after normalizing module paths, including attributes, serde/tauri attributes, error strings and constants.
- **Platform code:** the same `cfg` set (8 windows, 5 not-windows, 4 macos, 2 linux, 1 macos+aarch64), each in the same function.
- **Globals:** each defined once, with the same initializers.
- **Commands:** `generate_handler!` has the same 65 commands in the same order.
- **Tests:** 133 in L1 and L2, with identical bodies.

---

## B. Pre-existing behavior pinned by tests

Characterization tests pin what the old code did, including bugs. None of these were introduced by the refactor.

When the production behavior is fixed, update or replace the pinning test in the same change. Items are ordered by maintenance risk.

### B1. Unknown issue types and statuses are rewritten on save — High

- **Code:** `issues.rs:17-33`. `normalize_issue_type` allows only `bug|task|feature|epic|chore`; anything else becomes `task`. `normalize_issue_status` maps unknown values to `open`.
- **Tests:** `normalize_issue_type_defaults_unknown`, `normalize_issue_status_defaults_unknown` (`issues.rs:526, 546`).
- **Why it matters:**
  - bd 1.x built-in types also include `decision`, `message`, `molecule`, `gate`, `spike`, `story`, `milestone` (`../beads/internal/types/types.go:710-721`), plus `types.custom`.
  - `IssueForm.vue` always sends `type` and `status`, and `bd_update` forwards them (`issue_commands.rs:296-303`).
  - As a result, saving an issue of an unlisted type rewrites it to `task`.
  - `tombstone` is still on the allowed-status list, although bd 0.51.0 removed the tombstone system (`../beads` CHANGELOG, "Phase 4").
- **Direction:** pass unknown values through, or only send changed fields. Replace the tests with pass-through assertions.

### B2. `is_real_external_ref` substring match can drop real refs during migration — High

- **Code:** `attachment_refs.rs:12-23` treats any ref containing `/attachments/` or `/.beads/`, or starting with `/`, as local.
- **Test:** `is_real_external_ref_rejects_beads_refs` asserts `"path/attachments/file"` is not real.
- **Why it matters:**
  - Migration v3 keeps only refs that pass this check (`migration.rs:99-107`).
  - A real URL containing `/attachments/` (for example a Redmine attachment URL) would be dropped.
  - Conversely, a Windows absolute path (`C:\...\.beads\attachments\x.png`) contains neither `/attachments/` nor a leading `/`, so it is classified as real. The existing local-path test only covers Unix paths.
- **Direction:** accept anything with a URL scheme first, and treat Windows absolute paths as local. Add both cases to the tests.

### B3. Dolt detection false negatives on bd 1.x — High

- **Code:** `project_uses_dolt_for`, `cli.rs:482-517`. Returns true only if one of these holds:
  - `.beads/.dolt` exists, or
  - `metadata.json` contains the literal `"backend":"dolt"` (two whitespace spellings only) **and** a `.beads/dolt/<name>/.dolt` directory exists.
- **Tests:** `project_uses_dolt_for_nested_layout_needs_metadata_and_dolt_dir`, and the empty-dir case at `cli.rs:1284`.
- **Why it matters for bd 1.x:**
  - The Dolt data dir can live outside `.beads`: `dolt_data_dir` / `BEADS_DOLT_DATA_DIR` (`../beads/internal/configfile/configfile.go:36, 622-626`).
  - `dolt_mode` can be `server` (`configfile.go:29`).
  - `GetBackend()` falls back to Dolt when `backend` is missing or not a registered name (`configfile.go:297-312`).
  - SQLite was removed in 0.51.0. bd 1.x `GetBackend()` also recognizes `postgres` and `mysql` (`configfile.go:300-305`), so "not Dolt" on 1.x does not imply SQLite/JSONL.
- **Downstream effect of a false result:**
  - the watcher runs non-recursive (`watcher.rs`)
  - `get_beads_mtime` checks SQLite files (`polling.rs`)
  - migration status can report a partial migration (`migration.rs`)
- **Direction:** for bd ≥ 0.51, decide backend from `metadata.json` using bd's `GetBackend()` rules (missing → Dolt) instead of probing `.beads/dolt/*/.dolt`. Until then, comment these tests as a known limitation.

### B4. `--hard` delete cutoff is one minor version early — Medium

- **Code:** `supports_delete_hard_flag_for` (`cli.rs:434-438`) returns false for bd ≥ 0.50.0. The doc comment says "--hard flag was removed in bd 0.50.0".
- **Test:** asserts bd 0.50.0 does not support `--hard`.
- **Evidence:** `v0.50.0:cmd/bd/delete.go:63,737` still defines and reads `--hard` ("Permanently delete (skip tombstone…)"). The tombstone system was removed in 0.51.0.
- **Effect:** on 0.50.x the app does a soft (tombstone) delete.
- **Direction:** move the cutoff to 0.51, or document the approximation.

### B5. `uses_dolt_backend` / `uses_jsonl_files` cutoff at 0.50 — Medium

- **Tests:** bd 0.50.0 → Dolt, bd 0.50.0 → no JSONL.
- **Evidence:** `v0.50.0:internal/storage/factory/factory.go:49` maps an empty backend to SQLite, and `internal/storage/sqlite/` exists through v0.50.3. It is removed in v0.51.0 ("Phase 6: Remove SQLite backend entirely").
- **Impact:** limited. The CLI-level flag feeds `CompatibilityInfo`; per-project decisions use `project_uses_dolt`.
- **Direction:** move the cutoff to 0.51, or document it. The compatibility warning test for 0.50–0.56 (`warnings_for_0_50_through_0_56_include_dolt_note`, `cli.rs:1089`) pins message text built on the same assumption.

### B6. Test that cannot fail — Medium

- **Test:** `get_beads_mtime_returns_none_without_beads_dir` (`polling.rs:225`) asserts `result.is_some() || result.is_none()`.
- **Side effect:** it also spawns the real `bd --version` through `project_uses_dolt` and fills the global version cache.
- **Direction:** add a pure `_for` variant and assert `None`, or delete the test.

### B7. `supports_list_all_flag` doc contradicts code for br — Medium (unverified)

- **Conflict:** the doc comment (`cli.rs:409`) says "br: NO". The code (`cli.rs:416`) and the test return `true`.
- **Scope:** used on every poll.
- **Unverified:** the br source is not on disk, so which side is correct is not confirmed.
- **Direction:** confirm against br, then fix the doc or the code.

### B8. Attachment folders keyed by short ID — Medium

- **Code:** `resolve_attachment_dir` uses `issue_short_id` (`attachments.rs:303-319`), which strips everything up to the last `-`. `docs/attachments.md` describes `.beads/attachments/{issue-id}/`.
- **Tests:** `attachments.rs:793, 800`.
- **Collision case:** attachment dirs are per project (`.beads/` of that project), so they only collide when one database contains issues with different prefixes and the same suffix. In that case:
  - the issues share a folder
  - `bd_delete` removes the shared folder (`issue_commands.rs:494-497`)
- **Direction:** decide whether the doc or the code is authoritative. At minimum, comment the collision case in the test.

### B9. Pre-release version comparison — Medium

- **Code:** `compare_versions` (`updates.rs:130`) splits on `.` and drops parts that don't parse as `u32`.
  - `"1.0.0-alpha"` → `[1, 0]` (the test comment wrongly says `[1, 0, 0]`)
  - `"1.2.0-rc.1"` → `[1, 2, 1]`, so an RC build is never offered 1.2.0
- **Test:** `compare_versions_with_prerelease_parses_numeric_parts_only` (`updates.rs:484`).
- **Direction:** strip everything from `-` before splitting; fix the comment and assertions.

### B10. Tests that assert nothing off macOS — Low

- `find_platform_asset_matches_macos_arm64` (`updates.rs:424`): the whole body is `#[cfg(all(macos, aarch64))]`, so the test is empty elsewhere.
- `find_platform_asset_returns_none_for_no_match` (`updates.rs:438`): asserts only on macOS and leaves an unused variable elsewhere.
- `project_uses_dolt_false_without_beads_dir` (`cli.rs:1239`): calls the wrapper, which spawns real `bd`. It should call `project_uses_dolt_for`.

### B11. Priority parsing gaps — Low

- **Code:** `priority_to_number` (`issues.rs:8-15`).
  - It returns `"5"`–`"9"` unchanged for `p5`–`p9`.
  - It maps uppercase `P1` to `"3"`, because `strip_prefix('p')` is case-sensitive.
- **bd behavior:** accepts `P0`–`P4` in either case and rejects values outside 0–4 (`../beads/internal/validation/bead.go:14-26`).
- **Direction:** match bd (case-insensitive, clamp to 0–4) and extend the test.

### B12. `sanitize_filename` does not sanitize the extension — Low

- **Code:** `attachments.rs:236-290` sanitizes the stem but only lowercases the extension (text after the last `.`).
- **Example:** `"a.b<c>"` keeps `.b<c>`, which is invalid on Windows.
- **Direction:** sanitize the extension, and assert exact output in the test.

### B13. Test names and fixtures that don't match what they assert — Low

- **Misleading name:** `project_uses_dolt_for_br_and_legacy_bd_never_true` (`cli.rs:1295`) also asserts a `true` case.
- **Out-of-range case:** `warnings_for_0_50_through_0_56_include_dolt_note` (`cli.rs:1089`) also asserts 0.99.0 and pins message substrings.
- **Invalid fixture:** a relations test uses `"related-to"` (`issues.rs:653, 661`), which is not a bd dependency type. bd has `related` and `relates-to` (`types.go:1220, 1225`).
- **Blocking deps shown as relations:** `conditional-blocks` and `waits-for` block work in bd (`types.go:1308`) but are shown as plain relations.
- **Failed version parse is not cached:** `get_cli_client_info` does not cache a failure (`cli.rs:361-364`), so every version-gated call re-spawns `bd --version`.
- **Double warning:** tests pin "unparseable bd version counts as legacy", so the frontend shows the legacy banner on top of the parse warning (unverified in the UI).

---

## Disposition (phase-b)

Every item above closed against `docs/plans/phase-b/plan-phase-b.md` "Issue inventory" and each sprint's own acceptance criteria; the closing test or doc is the sprint's own change, not a re-verification against `../beads` beyond what the sprint doc records. Sprint docs are `docs/plans/phase-b/sprint-b-N.md`.

| Item | Disposition | Sprint | Closing test or doc |
| --- | --- | --- | --- |
| A1 log target | Closed by phase-a a-4. No phase-b work. | — | `plan-phase-a.md` issue inventory |
| A2 platform-gated imports (`updates.rs:4`, `attachments.rs:2`; `logging.rs` done in a-4) | Closed: fully-qualified `std::process::Command::new(..)` call sites, matching the existing Windows call site | b-12 | `cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`; `git diff` on `updates.rs`/`attachments.rs` |
| A3 wrapper + pure-core extraction | No action (verified result-equivalent). The `_for` cores moved unchanged. | — | `crates/btit-beads/src/gates.rs` table-driven tests |
| A4 orphaned doc comment (`cli.rs:594-595`), moved comment, visibility | Closed: the orphaned comment deleted with `CompatibilityInfo`'s move (b-2); `ensure_refs_migrated_v3` doc comment restored when `migration.rs` was rewired (b-8); visibility redefined by crate boundaries (`pub` = crate API, everything else private) | b-2, b-8 | crate diffs at each sprint head |
| B1 unknown type/status rewritten | Closed: `normalize_issue_type`/`normalize_issue_status` pass unknown values through | b-9 | `crates/btit-beads/src/issues.rs` tests |
| B2 `is_real_external_ref` substring match | Closed: URL scheme accepted first, Windows absolute paths local | b-11 | `crates/btit-app/src/attachment_refs.rs` tests |
| B3 Dolt detection false negatives on bd 1.x | Closed: bd ≥ 0.51 decides from `metadata.json` (`GetBackend()` rule); no `.dolt` probe; `dolt_data_dir` honoured | b-10 | `crates/btit-bd/src/backend.rs`, `crates/btit-bd/src/dolt.rs` tests |
| B4 `--hard` cutoff | Closed: cutoff moved to 0.51.0 | b-9 | `crates/btit-beads/src/gates.rs` tests |
| B5 `uses_dolt_backend`/`uses_jsonl_files` cutoff | Closed: cutoff moved to 0.51.0 | b-9 | `crates/btit-beads/src/gates.rs` tests |
| B6 test that cannot fail | Closed: `get_beads_mtime_for(uses_dolt, uses_jsonl, dir)` pure core; test asserts `None` | b-11 | `crates/btit-app/src/polling.rs` tests |
| B7 `supports_list_all_flag` doc vs code for br | **Open (OQ-4).** br source unavailable locally and the binary is not installed, so the behaviour could not be verified; the doc comment now reads "unverified against beads_rust (OQ-4)", code unchanged | b-9 | doc comment on `supports_list_all_flag_for` |
| B8 attachment folders keyed by short ID | Closed as documentation: `docs/attachments.md` `{issue-id}` → `{short-id}` with the collision note. Changing the layout (a data migration) is out of scope | b-11 | `docs/attachments.md` |
| B9 pre-release version comparison | Closed: strip from `-` before splitting | b-11 | `crates/btit-app/src/updates.rs` tests |
| B10 tests that assert nothing off macOS; wrapper-calling test | Closed: `find_platform_asset` tests use an injectable suffix (b-11); `project_uses_dolt_false_without_beads_dir` calls the `_for` core (b-10) | b-10, b-11 | `crates/btit-app/src/updates.rs`, `crates/btit-bd` tests |
| B11 priority parsing | Closed: case-insensitive `p`/`P`; values outside 0–4 fall back to `"3"` | b-9 | `crates/btit-beads/src/issues.rs` tests |
| B12 `sanitize_filename` extension | Closed: extension sanitized, bare trailing `.` dropped | b-11 | `crates/btit-app/src/attachments.rs` tests |
| B13 misleading test name (`cli.rs:1295`) | Closed: renamed, moved with `project_uses_dolt_for` | b-10 (moved to `btit-bd` in b-5) | `crates/btit-bd` tests |
| B13 out-of-range case (`cli.rs:1089`) | Closed: 0.99.0 case removed; message substrings kept | b-9 | `crates/btit-beads/src/compat.rs` tests |
| B13 invalid `related-to` fixture (`issues.rs:653,661`) | Closed: fixture uses `relates-to` | b-9 | `crates/btit-beads/src/issues.rs` tests |
| B13 blocking deps shown as relations (`conditional-blocks`, `waits-for`) | Closed: added to `STRUCTURAL_TYPES`/`BLOCKING_TYPES`, feed `blocked_by`/`blocks` | b-9 | `crates/btit-beads/src/issues.rs` tests |
| B13 failed version parse not cached | Closed: `CliRunner` caches the failed probe as `ProbeState::Failed`; reset on binary change or a compatibility recheck | b-11 | `crates/btit-cli/src/runner.rs` tests |
| B13 double warning (legacy banner plus parse warning) | **Out of scope (OQ-5).** Frontend banner behaviour, unverified in the UI. | — | — |

### Follow-ups filed during phase-b

Not owned by this table's A/B items; filed as separate issues for work explicitly deferred by a sprint doc or this sprint's own review. None are fixed by b-12 (see sprint-b-12.md "Known and out of scope").

- **#55** — subprocess timeouts, `spawn_blocking` at the `btit-app` command boundary, and `ProjectLocks` eviction (plan QA RSH-001/002/005; headroom noted in the plan's trait design section, not fixed by any phase-b sprint)
- **#65** — a flaky Windows attachments test; related in theme to B10 (platform-conditional tests) but not the same test and not fixed here
- **#67** — `migration.rs` decomposition (1909 lines at the b-12 head); this sprint kept it as two linear functions with `#[expect(clippy::too_many_lines, reason = "...")]` rather than splitting it, per the plan's "no behaviour change" constraint
- **#68** — further structuring of `btit-app`'s `Result<_, String>` command-boundary errors, beyond `BeadsError`'s discriminated union (b-3) and `map_err(|e| e.to_string())` at the IPC edge
- **#70** — a frontend duplicate-create issue; not a Rust backend item
- **#73** — logging defaults; not addressed by this sprint's logging work (the `#![deny(..)]` removal in `logging.rs` and the workspace lint adoption are unrelated)
- **#74** — `bd migrate --to-dolt` follow-up; `bd_migrate_to_dolt_with` (`migration.rs`) is unchanged behaviourally by b-12 beyond the panic-site and lint fixes recorded in this sprint's Implementation Notes
- **#76** — CLI probe retry; `CliRunner`'s `ProbeState::Failed` cache (b-11, B13) records a failed probe but does not retry it automatically
- **#77** — reserved filenames (e.g. Windows `CON`, `PRN`) in `sanitize_filename`; not covered by B12's extension sanitization
- Upstream **sc-observability#96** and **sc-observability#97** — tracked in the `../sc-observability` repository, not this one; not part of this table's A/B inventory
