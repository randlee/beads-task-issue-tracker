# CLAUDE.md

## Context Documents

- **[.claude/codebase-map.md](.claude/codebase-map.md)** — Architecture, all pages, components, composables, utils, Tauri commands, types, data flow
- **[docs/attachments.md](docs/attachments.md)** — Attachment system (filesystem-only, `external_ref` reserved for real external refs)

Consult these before starting any task.

## Workflows

### Issues
- `/run-issue <id>` — Always run before starting work on any issue
- `/close-issue` — Always ask confirmation before closing
- `/review-to-commit` — Always use when user asks to commit

### Session Completion
All steps mandatory. Work is NOT complete until `git push` succeeds.
1. File issues for remaining work
2. Run quality gates (if code changed): `pnpm test && npx vue-tsc --noEmit`
3. Close finished issues
4. `git pull --rebase && bd sync && git push && git status`

### Testing
- **Run before committing**: `pnpm test` — runs all Vitest unit tests
- **Watch mode**: `pnpm test:watch` — for development
- Tests live in `tests/` mirroring `app/` structure (e.g., `tests/utils/markdown.test.ts` → `app/utils/markdown.ts`)
- Pure logic must be extracted into `app/utils/` for testability (not buried in composables)
- When adding or modifying pure logic (filtering, sorting, parsing, transformations), add or update corresponding tests

### Code Organization
- **Never overload `app/pages/index.vue`** — extract logic into composables (`app/composables/`) and UI sections into dedicated components (`app/components/`)
- Keep `index.vue` as an orchestrator: layout structure, composable wiring, and minimal glue code
- Prefer reusable composables over inline logic for state, dialogs, resize, filtering, etc.
- **Prefer shared components** over duplication — if a UI element is used in multiple places, extract it into a shared component
- Rust crates live under `crates/`, and library crates (`btit-types`, `btit-beads`, `btit-cli`, `btit-bd`, `btit-br`) never depend on `tauri` or `sc-observability-log` — only `crates/btit-app` does

### Context Management
- **Always prefer `/continue-task` over `/compact`** — it preserves issue context, progress, and next steps far better
- When the session is long and context is getting large, proactively run `/continue-task` before auto-compact triggers
- If a `PreCompact` hook fires with "auto" trigger, immediately run `/continue-task` instead of letting compact proceed blindly

### CLI Policy (bd first)
- **`bd` (Go, [steveyegge/beads](https://github.com/steveyegge/beads)) is the primary and default CLI.** Target **bd 1.x** — that is what the maintainer runs and what new features are built against.
- **Warn on bd < 1.0.** Pre-1.0 versions (0.49 SQLite/JSONL, 0.50–0.56 embedded-Dolt/server-mode transition) are legacy. The app should keep working where the version-gated helpers already allow it, but surface a warning to the user (see `check_bd_compatibility` in `crates/btit-app/src/backend.rs` and the warnings in `crates/btit-beads/src/compat.rs`) rather than silently degrading.
- **`br` (Rust, [beads_rust](https://github.com/Dicklesworthstone/beads_rust)) remains supported** as a secondary CLI and can be selected in Settings. It is not a priority: do not block bd work on br parity, but do not break br detection or the `CliClient::Br` code paths either.
- **Default binary**: auto-detection probes `bd` first, then `br`, and falls back to `bd` when neither is found. The probe must use `get_extended_path()` so GUI launches (Finder/Dock, minimal PATH) resolve Homebrew/Go/Cargo installs.
- **History**: the original author pinned bd 0.49.x and recommended br because bd 0.50–0.56 removed embedded Dolt in favor of server mode (see [beads#2050](https://github.com/steveyegge/beads/issues/2050)). This project is now maintained independently and follows current bd. The branch `feat/bd-056-server-mode` holds earlier server-mode work (detection, adaptive polling, migration, DoltServerBanner) and can be mined when needed.

### bd Backward Compatibility
- Never assume all projects use Dolt — check `project_uses_dolt()` before skipping legacy paths (br and legacy bd projects are SQLite/JSONL); `project_uses_dolt` is `BeadsBackend::project_uses_dolt`
- Use version-gated helpers for any feature that depends on a specific bd version: `BeadsBackend::capabilities()`, backed by the `_for` cores in `crates/btit-beads/src/gates.rs`

### Logging
- **Never use `console.log`** — always use the native logger so logs end up in the app log file.
- **Frontend (TypeScript)**: `logFrontend('info', '[context] message')` — import from `~/utils/bd-api`. Calls the Rust `log_frontend` Tauri command, which logs through the `sc-observability-log` bridge with `target: "frontend"`; a leading `[tag]` in the message becomes the record's `action`.
- **Backend (Rust)**: `log_info!("[context] message")`, `log_error!(...)` macros — write directly to the native log.
- Levels: `'info'`, `'warn'`, `'error'`
- **Log file**: `<app_log_dir>/logs/beads-task-issue-tracker.log.jsonl` (macOS: `~/Library/Logs/com.beads.manager/logs/beads-task-issue-tracker.log.jsonl`) — structured JSONL written by `sc_observability_log::init` (phase-a); the debug panel renders it through `app/utils/log-format.ts`.

### Dev Server
Always kill zombies before starting: `pkill -f "beads-issue-tracker" 2>/dev/null && pnpm tauri:dev`

## GitHub — Account: w3dev33

### Releases
1. **Update `CHANGELOG.md`** with the target version heading and all changes
2. `python3 scripts/check_version_sync.py --set X.Y.Z` (same version as CHANGELOG). SSOT is `[workspace.package].version` in the root `Cargo.toml`; the script mirrors it into `package.json` and `Cargo.lock` (`tauri.conf.json` has no `version` — Tauri falls back to Cargo). CI's `version sync` job runs the script without `--set` and fails on any drift, including the Rust toolchain pin (`rust-toolchain.toml` ↔ `rust-version` ↔ workflow `toolchain:`).
3. Commit, tag (`git tag -a vX.Y.Z`), push with tags
4. `gh release create vX.Y.Z --title "..." --notes "..."`
5. **Update `.claude/codebase-map.md`** to reflect any structural changes (new files, composables, commands, etc.)

**Release notes must include:**
- bd compatibility version (e.g., `> Requires **bd 1.x**. bd < 1.0 is legacy and triggers a warning. br is supported as a secondary CLI.`)
- **Never upload DMG manually** — GitHub Actions handles artifacts
- macOS unsigned certificate notice:
  ```
  xattr -cr /Applications/Beads\ Task-Issue\ Tracker.app
  ```

### Commits
Keep `Co-Authored-By: Claude Code <noreply@anthropic.com>` for transparency.

## Permissions

### Always Allowed (no confirmation needed)
- All `bd` CLI commands
- File operations on `.claude/` and `.beads/`
- `~/.claude/` (global config)

### Always Require Confirmation
- `git commit`, `git push`, `/close-issue`

## Plan Mode

Save plans in `.claude/plans/` (local to project), never `~/.claude/plans/`.
