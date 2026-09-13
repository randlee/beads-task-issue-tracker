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
