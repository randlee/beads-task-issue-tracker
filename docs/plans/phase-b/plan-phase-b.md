---
phase: b
title: "phase-b: crate split — root workspace, btit-types, beads backend traits, btit-bd / btit-br"
canonical_path: docs/plans/phase-b/plan-phase-b.md
planning_branch: plan/phase-b
integration_branch: integrate/phase-b
status: draft
owner: Rand Lee
authored: 2026-09-13
baseline: integrate/phase-a@a18c724 (planning baseline; line numbers below are at this commit)
---

# phase-b: crate split

## Why this phase exists

btit's Rust backend is one Tauri crate, `src-tauri/` (package `beads-issue-tracker`, lib `app_lib`, 17 source files, 7 047 lines, 65 Tauri commands in `generate_handler!` at `src-tauri/src/lib.rs:71-137`). The module split of stack #31 (PRs #29, #30, #32) left the beads-CLI logic, the frontend data contract and the Tauri glue in the same crate, and its review recorded behaviour issues to carry into "the next refactor" (`docs/crate-split-refactor-issues.md`). ADR-003 (`git show origin/docs/adr-initial:docs/architecture.md`) states the decision: "Moving btit's backend into root `crates/` and splitting it into multiple crates is phase-b."

phase-b does that split. It is a **behaviour-preserving restructuring** followed by two bounded behaviour-fix sprints:

1. one Cargo workspace at the repository root with every Rust crate under `crates/`, the same layout as `../atm-core`, `../sc-compose` and `../sc-observability` (each has a root `Cargo.toml` `[workspace]` with `members = ["crates/…"]`, verified 2026-09-13);
2. `btit-types`: shared data types only;
3. `btit-beads`: the backend traits shared by bd and br, the backend-specific traits, the discriminated-union error, and the pure beads logic (version parsing, capability gates, issue transformation);
4. `btit-cli`: the process-spawning transport both CLIs share (extended PATH, `--json`, per-project lock);
5. `btit-bd` and `btit-br`: the two backend implementations;
6. `btit-app`: the Tauri crate, reduced to command glue, filesystem features (attachments, migration, polling, watcher, updates, probe, logging) and backend selection.

Maintainer direction (2026-09-13) is authoritative for the constraints below and is restated in "Binding outcomes".

## Prerequisites (outside this phase)

- **phase-a merged to `develop`.** `integrate/phase-a` is merged to `develop` through the single phase PR after a-6 closes (`docs/plans/phase-a/plan-phase-a.md`, "Phase closure"). At planning time this has not happened: PR #50 (a-5) is open and a-6 has not started (no `docs/plans/phase-a/handoff-a-6.md`). phase-b sprint branches are created only after this merge.
- **`integrate/phase-b` exists, created from `develop` after that merge.** This plan does not create it. Every sprint's `target:` is `integrate/phase-b`.
- **`sc-observability-log` is available outside btit.** a-6 records the merged `../sc-observability` PR in `docs/plans/phase-a/handoff-a-6.md`. b-1 switches btit from the path dependency `sc-observability-log = { path = "../crates/sc-observability-log" }` (`src-tauri/Cargo.toml:38`) to the published crates.io version named in that record, or, when no crates.io release exists yet, to a `git` dependency on `https://github.com/randlee/sc-observability` pinned with `rev = <merge commit recorded in handoff-a-6.md>`. This is the "first phase-b sprint" step that `plan-phase-a.md` ("Not part of phase-a") assigns to phase-b.
- **Toolchain.** Rust 1.98.1 pinned by `rust-toolchain.toml` and enforced by `scripts/check_version_sync.py` (PR #36; ADR-004). Unchanged by this phase.

## Binding outcomes

### Workspace layout (maintainer requirement 1)

- **One workspace, root manifest.** `Cargo.toml` at the repository root holds `[workspace]`, `[workspace.package]`, `[workspace.dependencies]` and `[workspace.lints]`. Members are `crates/btit-app`, `crates/btit-types`, `crates/btit-beads`, `crates/btit-cli`, `crates/btit-bd`, `crates/btit-br`. One `Cargo.lock` at the root, one `target/` at the root.
- **Why not `crates/Cargo.toml`.** The transitional phase-a workspace at `crates/Cargo.toml` (`[workspace.package] version = "0.1.0"`, `edition = "2024"`, `rust-version = "1.94.1"`, `crates/Cargo.toml:9-13`) cannot be the btit workspace: `[workspace.package].version` must be the app version (`1.24.5`, `src-tauri/Cargo.toml:9`) and `check_version_sync.py:71-75` requires every member to inherit it. Cargo also resolves a package's workspace by walking up to the nearest `Cargo.toml` with `[workspace]`, so a second workspace nested under `crates/` would claim any new `crates/btit-*` crate. b-1 therefore dissolves the phase-a workspace: its three crates leave btit (prerequisite above) together with `crates/Cargo.toml`, `crates/Cargo.lock`, `crates/runtime-deps.txt` and the `crates` CI job (`.github/workflows/ci.yml:92-153`).
- **Version single source of truth (ADR-003).** `[workspace.package].version` in the root `Cargo.toml`. Every crate uses `version.workspace = true`. `tauri.conf.json` keeps no `version`. `scripts/check_version_sync.py` is updated in b-1 to read the root manifest, root lockfile and `crates/btit-app/tauri.conf.json`; its checks otherwise stay as they are (`check_version_sync.py:62-118`).
- **Toolchain (ADR-004).** `[workspace.package].rust-version = "1.98.1"` equals the `rust-toolchain.toml` channel; the script enforces it (`check_version_sync.py:99-107`). The phase-a MSRV job (`ci.yml:149-153`, `cargo +1.94.1`) leaves with the phase-a crates.
- **Edition.** Open question OQ-1. Sprint docs are written for `edition = "2021"` at the workspace level (today's app edition, `src-tauri/Cargo.toml:13`), so every move is code motion that the existing 142 tests pin. Switching to 2024 is a maintainer decision; if taken before b-1 starts, b-1 sets it and each crate-creating sprint runs `cargo fix --edition` as part of its move.

### Tauri app crate location (maintainer requirement 1)

The Tauri crate moves to `crates/btit-app/` (`Cargo.toml`, `build.rs`, `tauri.conf.json`, `capabilities/`, `icons/`, `src/`, `.gitignore`). Verified constraints:

- **tauri-cli 2.11.2 finds the app without `src-tauri/`.** `crates/tauri-cli/src/helpers/app_paths.rs` (`resolve_tauri_dir`, tag `tauri-cli-v2.11.2`): it honours `TAURI_APP_PATH`, then checks `<cwd>/tauri.conf.json` and `<cwd>/src-tauri/tauri.conf.json`, then walks the tree to depth 3 (`TAURI_CLI_CONFIG_DEPTH`, default 3) skipping `node_modules/` and `target/` (`crates/tauri-cli/tauri.gitignore`). From the repo root `crates/btit-app/tauri.conf.json` is at depth 3, so `pnpm tauri:dev` (`package.json:33`, `cargo tauri dev`) and `pnpm tauri:build` keep working unchanged. The frontend directory resolves to the directory holding `package.json` at the cwd (`resolve_frontend_dir`), so `beforeDevCommand`/`beforeBuildCommand` still run at the root. Running `cargo tauri dev` from inside `crates/btit-app` is not supported (the frontend fallback would be `crates/`); `pnpm tauri:dev` from the root is the documented entry point (CLAUDE.md "Dev Server").
- **`tauri.conf.json` paths are relative to the config directory.** `build.frontendDist` changes from `../.output/public` to `../../.output/public` (`src-tauri/tauri.conf.json:6`). `devUrl`, `beforeDevCommand`, `beforeBuildCommand`, `bundle.icon` (relative `icons/…`), `capabilities/default.json` (`$schema: ../gen/schemas/desktop-schema.json`) and `build.rs` (`tauri_build::build()`) are unchanged.
- **Release workflow.** `tauri-apps/tauri-action@v0` (`.github/workflows/release.yml:82`) locates the app by globbing `**/tauri.conf.json` under `projectPath` (default `.`), ignoring `target` and `node_modules` (`tauri-action/src/utils.ts`, `getTauriDir`), and resolves the target dir to `<workspace root>/target` (`getTargetDir`). The artifact globs `src-tauri/target/...` (`release.yml:89,109,133,143,171-175,210-214`) become `target/...`.
- **Package and binary name.** The package stays `beads-issue-tracker` with `[lib] name = "app_lib"` (`src-tauri/Cargo.toml:18,28`), so the executable name inside every bundle, the `Cargo.lock` entry and `env!("CARGO_PKG_VERSION")` (`updates.rs:62`) do not change. Renaming the package is OQ-2 (tauri-utils 2.8.2 exposes `mainBinaryName`, `config.rs:3289`, which would keep the binary name).
- **CI.** The `backend` job runs `cargo check --workspace --all-targets` and `cargo test --workspace` from the root (`ci.yml:87,90` today point at `src-tauri/Cargo.toml`); `Swatinem/rust-cache` `workspaces: src-tauri` (`ci.yml:84`) is removed (default `.`). A `rust-quality` job (fmt, clippy `-D warnings`) covers the new crates from b-2 and the app crate from b-10.

### `btit-types`: data only (maintainer requirement 2)

`btit-types` contains only shared data types: `struct`/`enum` definitions with `serde` derives, trivial constructors/`From` impls and `Display` for value types. No functions with I/O, no `std::process`, no `std::fs`, no Tauri, no `log`. Dependencies: `serde` (`derive`) and `serde_json` (needed by `BdRawIssue.metadata: Option<serde_json::Value>`, `BdRawComment.id: serde_json::Value`, `Issue.metadata`, `types.rs:71,78,121`). Enforced by b-2's acceptance gate (`cargo tree -e normal -p btit-types` equals `btit-types`, `serde`, `serde_json` plus their transitive closure, and `! grep -rE 'std::(process|fs|env)|tauri|log::' crates/btit-types/src`). The type-move inventory is below.

### Traits live in `btit-beads`, not in `btit-types` (maintainer requirement 3)

Traits are behaviour contracts: they name operations (`list`, `sync`, `project_uses_dolt`), carry `Result<_, BeadsError>` and are implemented by I/O crates. Placing them in `btit-types` would put behaviour and an error type into the "only data types" crate and would force every consumer of a DTO to compile the contract. They therefore live in `btit-beads`, one level above `btit-types`, together with the error enum and the pure beads logic that every backend shares (version parsing, capability `_for` cores, issue transformation). `btit-types` stays a leaf that the frontend contract, the traits and future transports can all depend on.

### Trait design (maintainer requirements 3, 4, 5)

Three trait families, all in `crates/btit-beads/src/backend.rs`, all object-safe (`dyn`-usable) and `Send + Sync`:

- **`BeadsBackend`** (common; implemented by `btit-bd` and `btit-br`): the operations both CLIs support today. It has no notion of a binary, a process or `--json`; a future SQL transport against a beads Dolt server, or a DoltHub client, implements this trait alone. Backend-specific extensions are reached through optional accessors (`fn dolt(&self) -> Option<&dyn DoltOperations>`, `fn close_suggestions(&self) -> Option<&dyn CloseSuggestions>`), so the app holds `Arc<dyn CliBackend>` and never downcasts.
- **`CliBackend: BeadsBackend`** (implemented by `btit-bd` and `btit-br`): what only a spawned CLI has: `binary()`, `probe()`, `run_raw()` (the un-JSON'd, un-locked invocations that `migration.rs` makes today), and `release_source()` (the GitHub repo used by `check_bd_cli_update`, `updates.rs:285-292`).
- **Backend-specific traits:** `DoltOperations` (bd only: `doctor --fix --yes`, `migrate --to-dolt --yes`, `init --prefix`, `import -i`; `migration.rs:333-339,623-629,665-671,879-885`) and `CloseSuggestions` (br only: `close <id> --suggest-next`, `issue_commands.rs:410-413`).

Design headroom, not planned work: `ProjectRef` is a `#[non_exhaustive]` enum with the single variant `Local { cwd: Option<String> }` (today's `cwd: Option<&str>`); a remote transport adds a variant and callers that only pass a `ProjectRef` through do not change. The traits are synchronous, matching today's blocking `Command::output()` calls inside `async` Tauri commands (`cli.rs:555-560`); an async twin is not planned.

Signatures are in the trait inventory below and, authoritatively, in `sprint-b-3.md` "Explicit Code Samples".

### Behaviour preserved (maintainer requirement 6)

- **bd first, br secondary, fallback bd:** `CLI_CANDIDATES = ["bd","br"]`, `CLI_FALLBACK = "bd"`, `MIN_SUPPORTED_BD_MAJOR = 1`, `rank_cli_candidate`, `select_default_binary` (`cli.rs:76-84,130-163`) move unchanged (pure parts to `btit-beads`, the spawning `probe_cli_binary`/`default_cli_binary` to `btit-cli`).
- **bd < 1.0 warning:** `is_legacy_bd`, `cli_compatibility_warnings` (`cli.rs:115-121,218-253`) move unchanged to `btit-beads`; `check_bd_compatibility` keeps its name and its `CompatibilityInfo` JSON shape (`app/utils/bd-api.ts:711-733`).
- **Version-gated helpers:** the five pure `_for` cores keep their names and `(CliClient, u32, u32, u32)` signatures (`cli.rs:372,392,413,434,453`) so their table-driven tests (`cli.rs:1307-1421`) move byte-identical; the wrappers become `BeadsBackend::capabilities()` computed from the backend's cached probe.
- **`project_uses_dolt` checks:** `BeadsBackend::project_uses_dolt(&self, beads_dir)`; `btit-bd` keeps `project_uses_dolt_for` (`cli.rs:482-515`), `btit-br` returns `false` (the `Br` arm, `cli.rs:487`). Every call site (`watcher.rs:86`, `fs_commands.rs:50,73`, `polling.rs:96`, `migration.rs:200,270,330,513,588`) keeps calling it before choosing a legacy path.
- **`get_extended_path` probing:** `get_extended_path`, `extended_path_entries`, `new_command` (`cli.rs:22-72,200-207`) move to `btit-cli`; every spawn keeps `.env("PATH", get_extended_path())` and `--version` probes keep `current_dir(std::env::temp_dir())`.
- **Tauri command names and signatures:** the 65 names in `generate_handler!` (`lib.rs:71-137`), their argument names and their `Result<_, String>` shapes are unchanged; the 57 names the frontend invokes (`app/utils/bd-api.ts`) and the `beads-changed` event (`app/composables/useChangeDetection.ts:59`) are unchanged. Error strings the frontend matches (`SCHEMA_MIGRATION_ERROR`, `no such column: spec_id`, `bd-api.ts:253,256`; `Dolt backend configured but database not found`, `bd-api.ts:291`) are reproduced verbatim by `BeadsError`'s `Display`.
- **Gated logging:** `LOGGING_ENABLED`/`VERBOSE_LOGGING` and the `log_info!`/`log_warn!`/`log_error!`/`log_debug!` macros (`logging.rs:32-66`) move to `btit-beads::logging` as `#[macro_export]` macros, so every moved call site keeps its enabled/verbose gate. The only allowed logging change is the `context` label passed to `parse_issues_tolerant` (for example `bd_list_open`), which may become the trait method name; this is log text only, the same class of change as refactor item A1.

**Backend selection state.** Today the client kind is detected lazily and cached in `CLI_CLIENT_INFO` (`cli.rs:326-365`) and re-detected when the cache is empty. After b-7 the app builds one backend instance per configured binary (`btit_app::backend::build_backend`): the probe result selects `BrCli` (client `Br`) or `BdCli` (client `Bd` or `Unknown`, which today share every `_ =>` arm). The instance is rebuilt by `set_cli_binary_path` (today: `reset_bd_version_cache`, `config.rs:101`) and by `check_bd_compatibility` when its fresh probe reports a different client kind (today it refreshes the cache, `cli.rs:643-646`). The one accepted deviation: a binary whose `--version` fails at startup and later starts answering as `br` is still driven as `BdCli` until one of those two commands runs; today it would flip on the next gated call. This is recorded in sprint b-7.

### Engineering standards (maintainer requirement 7; ADR-001)

- **Errors are discriminated unions.** `btit_beads::BeadsError` is an `enum` with typed variants, `code()` and `remediation()` per variant, and a `Display` that reproduces today's command error strings (table in `sprint-b-3.md`). No `Box<dyn Error>`, no `String` errors cross a crate boundary. The Tauri commands keep `Result<_, String>` at the IPC edge by `map_err(|e| e.to_string())`.
- **No panics; deny lint set.** The root `[workspace.lints]` is phase-a's set (`crates/Cargo.toml:31-43`): `pedantic` warn plus `unwrap_used`, `expect_used`, `panic`, `unreachable`, `todo`, `unimplemented`, `indexing_slicing` = `deny`. `btit-types`, `btit-beads`, `btit-cli`, `btit-bd`, `btit-br` inherit it from creation (`[lints] workspace = true`, per-crate `clippy.toml` `allow-*-in-tests`). Moved code is fixed where it violates: `parts[0]`/`parts[1]`/`parts[2]` in `parse_bd_version` (`cli.rs:314-317`), `.lock().unwrap()` on `CLI_CLIENT_INFO` and `BD_PROJECT_LOCKS` (`cli.rs:327,519,548,553`). `btit-app` adopts the lint set in b-10 (`logging.rs` already carries the file-level `#![deny(..)]` from a-4).
- **Crates do not wrap `sc-observability-log`.** Only `btit-app` depends on `sc-observability-log` (`install_logging`, `logging.rs:88-110`). Library crates depend on the `log` facade only; `btit-beads::logging` gates calls, it does not wrap the bridge.
- **Formatting and clippy in CI.** `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` per new crate from the sprint that creates it; the app crate (283 rustfmt diffs and 19 clippy warnings at the baseline) joins in b-10.

### sc-lint (pending maintainer input)

_Left empty on purpose: the maintainer started this section and has not supplied its scope. Nothing here is planned._

## Crate dependency graph

```mermaid
flowchart BT
  types["btit-types<br/>data only<br/>serde, serde_json"]
  beads["btit-beads<br/>traits, BeadsError, pure beads logic, log gate<br/>+ log, serde_json"]
  cli["btit-cli<br/>shared CLI transport<br/>PATH, new_command, ProjectLocks, CliRunner"]
  bd["btit-bd<br/>BdCli: BeadsBackend + CliBackend + DoltOperations"]
  br["btit-br<br/>BrCli: BeadsBackend + CliBackend + CloseSuggestions"]
  app["btit-app (package beads-issue-tracker)<br/>Tauri commands, backend slot, fs features<br/>+ tauri, sc-observability-log, reqwest, notify, dirs"]
  beads --> types
  cli --> beads
  cli --> types
  bd --> cli
  br --> cli
  app --> bd
  app --> br
  app --> cli
  app --> beads
  app --> types
  future["(headroom, not planned)<br/>btit-dolt-sql / btit-dolthub<br/>implement BeadsBackend only"] -.-> beads
```

Dependency rules (checked by each sprint's `cargo tree` gate): `btit-types` has no workspace dependency; `btit-beads` depends only on `btit-types`; `btit-cli` on `btit-types`, `btit-beads`; `btit-bd` and `btit-br` on those three and never on each other; only `btit-app` depends on `tauri` and `sc-observability-log`.

## Trait inventory

Source columns cite `src-tauri/src/` at `a18c724`. "bd"/"br" mark the implementing crate. Full signatures are in `sprint-b-3.md`.

### `BeadsBackend` (common; bd and br)

| Method | Today's source | Notes |
| --- | --- | --- |
| `client() -> CliClient` | `get_cli_client_info` client part, `cli.rs:326-365` | from the cached probe |
| `version() -> Option<CliVersion>` | `get_cli_client_info` tuple part | same |
| `capabilities() -> BackendCapabilities` | `supports_daemon_flag`, `uses_jsonl_files`, `supports_list_all_flag`, `supports_delete_hard_flag`, `uses_dolt_backend` wrappers, `cli.rs:380-385,400-405,421-426,441-446,461-466` | computed from the `_for` cores with the cached probe |
| `project_uses_dolt(beads_dir: &Path) -> bool` | `project_uses_dolt` / `project_uses_dolt_for`, `cli.rs:473-515` | bd: full check; br: `false` |
| `list(project, &ListQuery) -> Result<Vec<BdRawIssue>>` | `bd_list` arg building and `--all` fallback, `issue_commands.rs:18-69`; `bd_count` two-call form `:81-90`; `bd_poll_data` `polling.rs:43-56` | `--limit=0` always; two-call fallback when `!supports_list_all_flag` |
| `ready(project) -> Result<Vec<BdRawIssue>>` | `bd_ready`, `issue_commands.rs:139-141` | |
| `status(project) -> Result<serde_json::Value>` | `bd_status`, `:149-152` | |
| `show(project, id) -> Result<Option<BdRawIssue>>` | `bd_show`, `:162-205` | not-found stderr and empty stdout → `None`; array-or-object; strict deserialize error |
| `create(project, &CreatePayload) -> Result<BdRawIssue>` | `bd_create`, `:214-274` | flag mapping incl. `priority_to_number` |
| `update(project, id, &UpdatePayload) -> Result<Option<BdRawIssue>>` | `bd_update`, `:285-394` | empty stdout → `show` fallback; lenient `.ok()` parse |
| `close(project, id) -> Result<serde_json::Value>` | `bd_close`, `:409-423` | br impl adds `--suggest-next` via `CloseSuggestions` |
| `search(project, query) -> Result<Vec<BdRawIssue>>` | `bd_search`, `:433-447` | empty / `[]` → empty vec; strict `Vec<BdRawIssue>` |
| `label_add` / `label_remove(project, id, label) -> Result<()>` | `:455-456`, `:463-464` | |
| `delete(project, id) -> Result<()>` | `bd_delete` args `:470-475` | `--force` plus `--hard` when `supports_delete_hard_flag` |
| `comment_add(project, id, content) -> Result<()>` | `bd_comments_add`, `:511-513` | |
| `dep_add(project, from, to, relation_type: Option<&str>) -> Result<()>` | `bd_dep_add` `:520-522`, `bd_dep_add_relation` `:538-540` | `--type <t>` when `Some` |
| `dep_remove(project, from, to) -> Result<()>` | `bd_dep_remove` `:529-531`, `bd_dep_remove_relation` `:547-549` | |
| `relation_types() -> Vec<RelationType>` | `bd_available_relation_types`, `:555-581` | br: common 7; bd/unknown: common + `tracks`, `until`, `validates` |
| `sync(project) -> Result<()>` | `sync_bd_database` spawn `migration.rs:222-232`; `bd_sync` `:278-288` | `sync [--no-daemon]`, no `--json`, no project lock (as today) |
| `dolt() -> Option<&dyn DoltOperations>` | new accessor | bd: `Some(self)`; br: `None` |
| `close_suggestions() -> Option<&dyn CloseSuggestions>` | new accessor | br: `Some(self)`; bd: `None` |

### `CliBackend: BeadsBackend` (bd and br)

| Method | Today's source | Notes |
| --- | --- | --- |
| `binary() -> &str` | `get_cli_binary`, `config.rs:58-60` | the configured name/path |
| `probe() -> Option<CliProbe>` | `probe_cli_binary`, `cli.rs:185-196` | fresh `--version` run from the temp dir |
| `run_raw(project, args: &[&str]) -> Result<CliOutput>` | `new_command(..).args(..).current_dir(..).env("PATH",..).env("BEADS_PATH",..).output()` pattern, `migration.rs:227-232,282-288,333-339,403-408,623-629,665-671,771-777,879-885,944-950,1003-1009,1081-1087` | no `--json`, no lock; returns status, stdout, stderr |
| `release_source() -> ReleaseSource` | `check_bd_cli_update`, `updates.rs:285-292` | bd/unknown: `steveyegge/beads`; br: `Dicklesworthstone/beads_rust` |

### `DoltOperations` (bd only)

| Method | Today's source |
| --- | --- |
| `doctor_fix(project) -> Result<CliOutput>` | `bd_repair_database` Dolt path, `migration.rs:333-339` (`doctor --fix --yes`) |
| `migrate_to_dolt(project) -> Result<CliOutput>` | `bd_migrate_to_dolt`, `:623-629` (`migrate --to-dolt --yes`) |
| `init(project, prefix) -> Result<CliOutput>` | `:665-671`, `:771-777` (`init --prefix <p>`) |
| `import_jsonl(project, file) -> Result<CliOutput>` | `:879-885` (`import -i <file>`) |

### `CloseSuggestions` (br only)

| Method | Today's source |
| --- | --- |
| `close_suggesting_next(project, id) -> Result<serde_json::Value>` | `bd_close` br branch, `issue_commands.rs:410-413` (`close <id> --suggest-next`) |

### Pure functions moving to `btit-beads` (not trait methods)

`detect_cli_client`, `parse_bd_version`, `parse_cli_probe` (`cli.rs:290-323,166-178`); `is_legacy_bd`, `rank_cli_candidate`, `select_default_binary`, `CLI_CANDIDATES`, `CLI_FALLBACK`, `MIN_SUPPORTED_BD_MAJOR` (`cli.rs:76-84,115-163`); `cli_client_name`, `cli_compatibility_warnings` (`cli.rs:209-253`); the five `_for` gate cores (`cli.rs:372-459`); `priority_to_string`, `priority_to_number`, `normalize_issue_type`, `normalize_issue_status`, `transform_issue`, `normalize_metadata`, `parse_issues_tolerant` (`issues.rs:3-312`).

## Type-move inventory

| Type | From (`src-tauri/src/`) | To | Change |
| --- | --- | --- | --- |
| `CliClient` | `types.rs:5-9` | `btit-types` | `pub(crate)` → `pub`; add `Eq`, `Hash` |
| `BdRawDependency`, `BdRawDependent`, `BdRawIssue`, `BdRawComment` | `types.rs:19-84` | `btit-types` | none |
| `Issue`, `Comment`, `ChildIssue`, `ParentIssue`, `Relation` | `types.rs:87-166` | `btit-types` | none |
| `CountResult`, `DirectoryEntry`, `PurgeResult`, `FsListResult` | `types.rs:169-208` | `btit-types` | none |
| `ListOptions`, `CwdOptions`, `CreatePayload`, `UpdatePayload` | `types.rs:215-281` | `btit-types` | none |
| `CliProbe` | `cli.rs:88-93` | `btit-types` | fields `pub`; `version: Option<CliVersion>` (was tuple) with `From<(u32,u32,u32)>` |
| `CliSelection` | `cli.rs:97-111` | `btit-beads::detect` | stays next to `select_default_binary` (it carries `is_legacy()` logic) |
| `CompatibilityInfo` | `cli.rs:597-619` | `btit-types` | fields `pub` |
| `CliVersion { major, minor, patch }` | new | `btit-types` | replaces `(u32, u32, u32)` at crate boundaries; `_for` cores keep the tuple |
| `BackendCapabilities` | new (today five `bool` wrappers) | `btit-types` | five `bool` fields |
| `ListQuery` | new (today `bd_list` arg building) | `btit-types` | `status`, `issue_type`, `priority`, `assignee`, `include_all` |
| `ProjectRef` | new (today `cwd: Option<&str>`) | `btit-types` | `#[non_exhaustive] enum { Local { cwd: Option<String> } }` |
| `RelationType { value, label }` | new (today `(&str, &str)` tuples, `issue_commands.rs:556-569`) | `btit-types` | |
| `ReleaseSource { api_url, releases_url }` | new (today string literals, `updates.rs:285-292`) | `btit-types` | |
| `CliOutput { status: Option<i32>, stdout, stderr }` | new (today `std::process::Output`) | `btit-types` | |
| `BeadsError` | new (today `String`) | `btit-beads::error` | discriminated union |
| `PollData` | `polling.rs:22-29` | stays in `btit-app` | app-only DTO |
| `RepairResult`, `MigrateResult`, `CleanupResult`, `MigrationStatus` | `migration.rs:304-308,436-439,452-454,475-478` | stays in `btit-app` | app-only DTOs |
| `RefsMigrationStatus`, `MigrateRefsResult` | `attachment_refs.rs:27-38` | stays in `btit-app` | app-only |
| `ImageData`, `AttachmentFile`, `TextData` | `attachments.rs:68-71,432-437,570-572` | stays in `btit-app` | app-only |
| `UpdateInfo`, `GitHubAsset`, `GitHubRelease`, `BdCliUpdateInfo` | `updates.rs:14-56` | stays in `btit-app` | app-only |
| `WatcherState`, `BeadsChangedPayload`, `WatcherStatusInfo` | `watcher.rs:13-30,118-123` | stays in `btit-app` | Tauri state |
| `AppConfig` | `config.rs:11-14` | stays in `btit-app` | settings file |

## Issue inventory: `docs/crate-split-refactor-issues.md`

Every item maps to a sprint deliverable or an explicit out-of-scope note.

| Item | Disposition | Sprint |
| --- | --- | --- |
| A1 log target | Closed by phase-a a-4 (`plan-phase-a.md` issue inventory). No phase-b work. | — |
| A2 platform-gated imports (`updates.rs:4`, `attachments.rs:2`; `logging.rs` done in a-4) | Closed: fully-qualified call sites, verified with `cargo xwin check --target x86_64-pc-windows-msvc` | b-10 |
| A3 wrapper + pure-core extraction | No action (verified result-equivalent). The `_for` cores move unchanged. | — |
| A4 orphaned doc comment (`cli.rs:594-595`), moved comment, visibility | Closed: the orphaned comment is deleted with `cli.rs` (its subject `ensure_refs_migrated_v3` gets a doc comment in b-7); visibility is redefined by the crate boundaries (`pub` = crate API, everything else private) | b-2 (comment moves with `CompatibilityInfo`), b-7 |
| B1 unknown type/status rewritten | Closed: `normalize_issue_type`/`normalize_issue_status` pass unknown values through; tests replaced | b-8 |
| B2 `is_real_external_ref` substring match | Closed: URL scheme accepted first, Windows absolute paths local; tests added | b-9 |
| B3 Dolt detection false negatives on bd 1.x | Closed: bd ≥ 0.51 decides from `metadata.json` with bd's `GetBackend()` rule (missing/unknown → Dolt; `configfile.go:297-311`), no `dolt/<name>/.dolt` probe; `dolt_data_dir` honoured; tests replaced | b-9 |
| B4 `--hard` cutoff | Closed: cutoff moves to 0.51.0; test updated | b-8 |
| B5 `uses_dolt_backend`/`uses_jsonl_files` cutoff | Closed: cutoff moves to 0.51.0; `warnings_for_0_50_through_0_56_include_dolt_note` message text updated with it | b-8 |
| B6 test that cannot fail | Closed: `get_beads_mtime_for(uses_dolt: bool, uses_jsonl: bool, dir)` pure core; test asserts `None` | b-9 |
| B7 `supports_list_all_flag` doc vs code for br | **Open question OQ-4.** br source is not on disk and `br` is not installed (verified 2026-09-13). b-8 fixes whichever side the maintainer confirms; until then the doc comment is changed to say "unverified" and the code is unchanged | b-8 |
| B8 attachment folders keyed by short ID | Closed as documentation: `docs/attachments.md` `{issue-id}` becomes `{short-id}` with the collision note; the test gains the collision comment. Changing the layout is out of scope (it would need a data migration) | b-9 |
| B9 pre-release version comparison | Closed: strip from `-` before splitting; test comment and assertions fixed | b-9 |
| B10 tests that assert nothing off macOS; wrapper-calling test | Closed: `find_platform_asset` tests use an injectable suffix; `project_uses_dolt_false_without_beads_dir` calls the `_for` core | b-9 |
| B11 priority parsing | Closed: case-insensitive `p`/`P`, clamp to 0–4 per bd (`../beads/internal/validation/bead.go:14-26`); test extended | b-8 |
| B12 `sanitize_filename` extension | Closed: extension sanitized; exact-output assertion | b-9 |
| B13 misleading test name (`cli.rs:1295`) | Closed: renamed | b-9 (test moves with `project_uses_dolt_for` to `btit-bd`) |
| B13 out-of-range case in `warnings_for_0_50_through_0_56` (`cli.rs:1089`) | Closed: 0.99.0 case removed; message substrings kept | b-8 |
| B13 invalid `related-to` fixture (`issues.rs:653,661`) | Closed: fixture uses `relates-to` | b-8 |
| B13 blocking deps shown as relations (`conditional-blocks`, `waits-for`) | Closed: added to the structural set in `transform_issue`, so they feed `blocked_by`/`blocks`; test added | b-8 |
| B13 failed version parse not cached | Closed: `CliRunner` caches the failed probe; reset on `set_cli_binary_path` and `check_bd_compatibility` | b-9 |
| B13 double warning (legacy banner plus parse warning) | Out of scope: frontend banner behaviour, unverified in the UI; recorded as OQ-5 | — |

## Sprint sequence

| Sprint | Branch | Stack · layer | Depends on (`must_follow`) | Parallel with (`parallel_safe`) | Authoritative plan | Production closure |
| --- | --- | --- | --- | --- | --- | --- |
| `b-1` | `feature/sprint-b-1-workspace-foundation` | `phase-b-core` · 1 | phase-a merged; `integrate/phase-b` exists | none | [`sprint-b-1.md`](./sprint-b-1.md) | root workspace; Tauri crate at `crates/btit-app`; phase-a crates replaced by the published dependency; CI, release, version script, docs paths updated; app builds and 142 tests pass |
| `b-2` | `feature/sprint-b-2-btit-types` | `phase-b-core` · 2 | b-1 | none | [`sprint-b-2.md`](./sprint-b-2.md) | `btit-types` crate with the inventory above; app consumes it |
| `b-3` | `feature/sprint-b-3-btit-beads` | `phase-b-core` · 3 | b-2 | none | [`sprint-b-3.md`](./sprint-b-3.md) | `btit-beads`: traits, `BeadsError`, pure logic and tests moved, logging gate; app consumes the pure logic |
| `b-4` | `feature/sprint-b-4-btit-cli` | `phase-b-core` · 4 | b-3 | b-8 | [`sprint-b-4.md`](./sprint-b-4.md) | `btit-cli`: PATH, `new_command`, `ProjectLocks`, `CliRunner`, probe/auto-detect, shared issue-op bodies |
| `b-5` | `feature/sprint-b-5-btit-bd` | `phase-b-core` · 5 | b-4 | b-6 (content), b-8 | [`sprint-b-5.md`](./sprint-b-5.md) | `btit-bd`: `BdCli` implementing `BeadsBackend`, `CliBackend`, `DoltOperations`; `project_uses_dolt_for` and its tests |
| `b-6` | `feature/sprint-b-6-btit-br` | `phase-b-core` · 6 | b-4 (content), b-5 (stack parent) | b-5 (content), b-8 | [`sprint-b-6.md`](./sprint-b-6.md) | `btit-br`: `BrCli` implementing `BeadsBackend`, `CliBackend`, `CloseSuggestions` |
| `b-7` | `feature/sprint-b-7-app-rewire` | `phase-b-core` · 7 | b-5, b-6 | b-8 | [`sprint-b-7.md`](./sprint-b-7.md) | app on `Arc<dyn CliBackend>`; `cli.rs`, `issues.rs`, `types.rs`, `test_support.rs` deleted; 65 commands unchanged; baseline tests all present in the workspace |
| `b-8` | `feature/sprint-b-8-beads-domain-fixes` | none (single PR; based on the b-3 branch until b-3 merges) | b-3 | b-4, b-5, b-6, b-7 | [`sprint-b-8.md`](./sprint-b-8.md) | B1, B4, B5, B11, B13 (fixture, out-of-range case, blocking relations) fixed in `btit-beads` with tests replaced; B7 per OQ-4 |
| `b-9` | `feature/sprint-b-9-storage-app-fixes` | `phase-b-core` · 8 | b-7, b-8 (PR merged) | none | [`sprint-b-9.md`](./sprint-b-9.md) | B2, B3, B6, B8, B9, B10, B12, B13 (rename, probe cache) fixed in `btit-bd`, `btit-cli`, `btit-app` with tests replaced |
| `b-10` | `feature/sprint-b-10-app-hardening` | `phase-b-core` · 9 | b-9 | none | [`sprint-b-10.md`](./sprint-b-10.md) | `btit-app` inherits the workspace lints, no panics, fmt/clippy `-D warnings` clean, A2 closed; CHANGELOG, CLAUDE.md, codebase map, refactor-issues dispositions |

No deliverable is repeated across sprint checklists. Each sprint's status is its own sprint doc's `status:` frontmatter; this plan carries no status rows.

### Execution lanes

```mermaid
flowchart LR
  PA["phase-a merged to develop;<br/>integrate/phase-b created"] --> B1
  subgraph core["stack phase-b-core (trunk integrate/phase-b)"]
    B1["b-1 workspace"] --> B2["b-2 btit-types"] --> B3["b-3 btit-beads"] --> B4["b-4 btit-cli"] --> B5["b-5 btit-bd"] --> B6["b-6 btit-br"] --> B7["b-7 app rewire"] --> B9["b-9 storage/app fixes"] --> B10["b-10 app hardening"]
  end
  subgraph lane2["lane 2: single PR"]
    B8["b-8 beads-domain fixes"]
  end
  B3 -. "b-3 pushed: branch b-8 from it; PR base = b-3 branch, retargeted to integrate/phase-b when b-3 merges" .-> B8
  B8 -. "b-8 PR merged; rebase stack" .-> B9
```

- **Lane 1, `phase-b-core`:** b-1 → b-2 → b-3 → b-4 → b-5 → b-6 → b-7 → b-9 → b-10, one gh-stack. b-6 depends on b-4's content only; it is stacked on b-5 because stacks are strictly linear (`~/.claude/skills/gh-stack/SKILL.md` "Known limitations" 1) and b-6 development starts as soon as the b-5 branch exists, so b-5 and b-6 are developed in parallel and merged in order.
- **Lane 2:** b-8 touches only `crates/btit-beads/**` and its own sprint doc. It cannot be stacked (b-3 already has b-4 as its child), so it is a single PR. It starts when b-3 development is pushed, with the PR based on `feature/sprint-b-3-btit-beads` (the same workaround phase-a used for a-4 while a-1 was unmerged, `sprint-a-4.md` "Implementation Notes"); when b-3 merges the PR is retargeted to `integrate/phase-b`. The maintainer merges it with the normal PR merge; it is not part of the stack.
- **Join:** b-9 development starts only after the b-8 PR is merged and the remaining stack layers are rebased onto `integrate/phase-b`.

### Why b-8 is not a stack layer between b-3 and b-4

b-4, b-5, b-6 and b-7 only *read* the public signatures b-3 freezes; b-8 changes behaviour behind those signatures. Making b-8 a layer would force four layers to rebase over behaviour changes they do not use and would serialize its merge behind theirs. As a separate PR it merges as soon as it is green.

## gh-stack and worktree workflow

The rules are those of `plan-phase-a.md` ("gh-stack and worktree workflow"; ADR-005), restated for phase-b:

- One GitHub stack, `phase-b-core`, managed with `gh stack link` and `gh stack merge`; no local stack tracking (`gh stack rebase`/`sync`/`view` fail from linked worktrees, verified in phase-a).
- Each layer is developed in its own `/sc-git-worktree` worktree under `../beads-task-issue-tracker-worktrees/<branch>`; the row in `../beads-task-issue-tracker-worktrees/worktree-tracking.md` is a local record, not a QA gate. The QA-visible record is each sprint doc's `worktree:` frontmatter.
- Layers rebase with `git rebase <parent>` inside their worktree, bottom to top, then `git push --force-with-lease`.
- Stacked PRs merge with `gh stack merge <PR> --yes` bottom-up; never `gh pr merge` for a stack layer. b-1 merges as soon as it passes so the root workspace lands early.
- Stack state is inspected only with `gh stack view --json` from the main checkout (`/gh-stack-view`).
- Evidence anchored across rebases uses annotated tags plus tree hashes, not layer SHAs.

```bash
# Layer creation (repo root), each from its parent
git worktree add -b feature/sprint-b-1-workspace-foundation ../beads-task-issue-tracker-worktrees/feature/sprint-b-1-workspace-foundation origin/integrate/phase-b
git worktree add -b feature/sprint-b-2-btit-types            ../beads-task-issue-tracker-worktrees/feature/sprint-b-2-btit-types            feature/sprint-b-1-workspace-foundation  # when b-1 is pushed
git worktree add -b feature/sprint-b-3-btit-beads            ../beads-task-issue-tracker-worktrees/feature/sprint-b-3-btit-beads            feature/sprint-b-2-btit-types            # when b-2 is pushed
git worktree add -b feature/sprint-b-4-btit-cli              ../beads-task-issue-tracker-worktrees/feature/sprint-b-4-btit-cli              feature/sprint-b-3-btit-beads            # when b-3 is pushed
git worktree add -b feature/sprint-b-5-btit-bd               ../beads-task-issue-tracker-worktrees/feature/sprint-b-5-btit-bd               feature/sprint-b-4-btit-cli              # when b-4 is pushed
git worktree add -b feature/sprint-b-6-btit-br               ../beads-task-issue-tracker-worktrees/feature/sprint-b-6-btit-br               feature/sprint-b-5-btit-bd               # as soon as the b-5 branch exists
git worktree add -b feature/sprint-b-7-app-rewire            ../beads-task-issue-tracker-worktrees/feature/sprint-b-7-app-rewire            feature/sprint-b-6-btit-br               # when b-5 and b-6 are pushed
git worktree add -b feature/sprint-b-9-storage-app-fixes     ../beads-task-issue-tracker-worktrees/feature/sprint-b-9-storage-app-fixes     feature/sprint-b-7-app-rewire            # when b-7 is pushed AND the b-8 PR is merged (rebase b-7 first)
git worktree add -b feature/sprint-b-10-app-hardening        ../beads-task-issue-tracker-worktrees/feature/sprint-b-10-app-hardening        feature/sprint-b-9-storage-app-fixes     # when b-9 is pushed

# GitHub stack: link the first two layers, then append by stack number after each layer's first push
gh stack link --base integrate/phase-b feature/sprint-b-1-workspace-foundation feature/sprint-b-2-btit-types
gh stack link <stack-number> feature/sprint-b-3-btit-beads   # likewise b-4 … b-10

# Lane 2 (not stacked)
git worktree add -b feature/sprint-b-8-beads-domain-fixes ../beads-task-issue-tracker-worktrees/feature/sprint-b-8-beads-domain-fixes feature/sprint-b-3-btit-beads   # when b-3 is pushed
gh pr create --base feature/sprint-b-3-btit-beads ...       # retarget with `gh pr edit --base integrate/phase-b` when b-3 merges

# Merge-forward before every dev/fix round on layer N (each in its own worktree)
git -C ../beads-task-issue-tracker-worktrees/<layer-N-branch> fetch origin
git -C ../beads-task-issue-tracker-worktrees/<layer-N-branch> rebase <layer-(N-1)-branch>   # layer 1 rebases onto origin/integrate/phase-b
git -C ../beads-task-issue-tracker-worktrees/<layer-N-branch> push --force-with-lease
```

## Dependency relations

`must_follow` merge-forward trigger: parent development is pushed, not QA-approved; the parent is rebased into the child before every dev/fix round. PR-completion trigger: the parent PR merges first. `parallel_safe` requires non-intersecting modules, crates, public contracts, artifacts and ownership.

| Relation | Rationale |
| --- | --- |
| `b-1 must_follow phase-a merge` | b-1 removes the phase-a `crates/` workspace and depends on the published `sc-observability-log`; both need a-6 closed. |
| `b-2 must_follow b-1` | `btit-types` is a member of the root workspace b-1 creates. |
| `b-3 must_follow b-2` | traits, `BeadsError` and the pure logic are typed with `btit-types`. |
| `b-4 must_follow b-3` | `CliRunner` returns `BeadsError`, uses the pure parsers/gates and the `log_*!` macros from `btit-beads`. |
| `b-5 must_follow b-4` | `BdCli` wraps `CliRunner` and the shared issue-op bodies. |
| `b-6 must_follow b-4` (content) and `b-6 must_follow b-5` (stack parent) | `BrCli` wraps `CliRunner`; stacked on b-5 for linearity only. |
| `b-7 must_follow b-5, b-6` | the app builds `BdCli`/`BrCli` and deletes the code they replaced. |
| `b-8 must_follow b-3` | edits the pure functions b-3 moved into `btit-beads`. |
| `b-9 must_follow b-7` and `b-9 must_follow b-8` (PR merged) | edits app modules b-7 rewires and `btit-bd`/`btit-cli` state; the pinned-test list it updates must include b-8's replacements. |
| `b-10 must_follow b-9` | lint rollout touches every app module after the last behaviour fix. |
| `b-8 parallel_safe b-4, b-5, b-6, b-7` | b-8 owns `crates/btit-beads/**` (implementation and tests behind frozen public signatures) and its sprint doc; b-4..b-7 own other crates and app files and only *use* `btit-beads`' public API, which b-8 must not change (acceptance gate: `cargo public-api`-free check via `git diff` on `crates/btit-beads/src/lib.rs` re-exports and an unchanged `crates/btit-beads/tests/api_freeze.rs`). |
| `b-5 parallel_safe b-6` (content) | disjoint crates `btit-bd` / `btit-br`; both only add a member line to the root `Cargo.toml` (resolved in the b-6 rebase). |

### Ownership table

| Artifact | b-1 | b-2 | b-3 | b-4 | b-5 | b-6 | b-7 | b-8 | b-9 | b-10 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| root `Cargo.toml` (`[workspace.*]`, lints, deps) | **creates** | adds member, deps | adds member, deps | adds member | adds member | adds member | — | — | — | app `[lints]` |
| root `Cargo.lock` | **creates** | refresh | refresh | refresh | refresh | refresh | refresh | — | refresh | refresh |
| `crates/btit-app/**` (moved app) | **moves** | re-imports types | re-imports pure logic | — | — | — | **rewires**, deletes `cli.rs`, `issues.rs`, `types.rs`, `test_support.rs` | — | fixes | lints, fmt |
| `crates/btit-types/**` | — | **creates** | — | — | — | — | — | — | — | — |
| `crates/btit-beads/**` | — | — | **creates** (API frozen by `tests/api_freeze.rs`) | — | — | — | — | **fixes behind the frozen API** | — | — |
| `crates/btit-cli/**` | — | — | — | **creates** | — | — | — | — | probe-cache fix | — |
| `crates/btit-bd/**` | — | — | — | — | **creates** | — | — | — | B3, B13 rename | — |
| `crates/btit-br/**` | — | — | — | — | — | **creates** | — | — | — | — |
| `crates/sc-observability-log*/**`, `crates/Cargo.toml`, `crates/Cargo.lock`, `crates/runtime-deps.txt`, `src-tauri/**` | **deletes/moves** | — | — | — | — | — | — | — | — | — |
| `.github/workflows/ci.yml` | **backend job, removes `crates` job** | adds `rust-quality` job | extends | extends | extends | extends | — | — | — | adds app to fmt/clippy |
| `.github/workflows/release.yml` | **artifact paths** | — | — | — | — | — | — | — | — | — |
| `scripts/check_version_sync.py`, `rust-toolchain.toml` comments, `.gitignore`, `.sc/repowise/repowise.yaml`, `package.json` scripts | **owns** | — | — | — | — | — | — | — | — | — |
| `CLAUDE.md`, `.claude/codebase-map.md`, `docs/attachments.md` path references | **path fixes only** | — | — | — | — | — | — | — | B8 doc | **content** |
| `CHANGELOG.md`, `docs/crate-split-refactor-issues.md` disposition table | — | — | — | — | — | — | — | — | — | **owns** |
| `docs/plans/phase-b/sprint-b-N.md` `status:` frontmatter | own doc | own doc | own doc | own doc | own doc | own doc | own doc | own doc | own doc | own doc |

## Cross-sprint document ownership

- **b-1** rewrites every path reference to `src-tauri/` in `CLAUDE.md` (lines 44, 51, 67), `.claude/codebase-map.md` (lines 16, 208, 420), `docs/attachments.md` (line 78), `rust-toolchain.toml` (lines 1-4), `.sc/repowise/repowise.yaml` (lines 8, 12, 21) and `.github/workflows/*.yml`. `docs/crate-split-refactor-issues.md` lines 3 and 11 cite the historical commit `f6a0db3` and are left as they are.
- **b-3** creates `crates/btit-beads/docs/backend-contract.md` (the trait contract, error inventory and the transport headroom notes).
- **b-9** updates `docs/attachments.md` for B8.
- **b-10** owns `CHANGELOG.md` (`[Unreleased]` entry for the whole phase, listing the crate layout and every B-item closed, collated from the sprint docs' "Changelog lines" sections), the `## Backend Structure` rewrite in `.claude/codebase-map.md`, the CLI-policy paragraphs in `CLAUDE.md`, and appends a disposition table to `docs/crate-split-refactor-issues.md`.
- **Every sprint** updates only the `status:` frontmatter of its own sprint doc; no sprint edits this plan.

## Error inventory

`btit_beads::BeadsError` is the only error type crossing crate boundaries. Its authoritative variant table, with `code()`, `remediation()` and the `Display` string each variant must reproduce, is in `sprint-b-3.md` "Required Work". Later sprints add no variants without a plan change; b-9's B3 work uses the existing `Io` variant.

## Test preservation

The baseline has 142 tests (`cargo test --manifest-path src-tauri/Cargo.toml`, `a18c724`; per module: cli 59, issues 31, attachments 23, updates 9, attachment_refs 6, logging 6, migration 5, config 2, polling 1). Each moving sprint carries its tests to the destination crate unchanged (module path prefix aside). b-7's acceptance gate compares the baseline test-name list with the workspace list:

```bash
git stash -q 2>/dev/null; git worktree add -q /tmp/btit-baseline origin/integrate/phase-b   # or the b-1 base
cargo test --manifest-path /tmp/btit-baseline/src-tauri/Cargo.toml -- --list 2>/dev/null | sed -nE 's/^.*::([a-z0-9_]+): test$/\1/p' | sort -u > /tmp/baseline-tests.txt
cargo test --workspace -- --list 2>/dev/null | sed -nE 's/^.*::([a-z0-9_]+): test$/\1/p' | sort -u > /tmp/after-tests.txt
comm -23 /tmp/baseline-tests.txt /tmp/after-tests.txt   # must print nothing until b-8/b-9 replace pinned tests by name
```

b-8 and b-9 each list, by name, the pinned tests they replace and their replacements, so the gate stays mechanical.

## Phase closure

phase-b closes when all of the following hold:

- b-1 to b-10 are merged to `integrate/phase-b`.
- `integrate/phase-b` is merged to `develop` through a single phase PR.
- `src-tauri/` no longer exists; `cargo tree --workspace -e normal` shows the dependency rules of the crate graph; `pnpm tauri:build` produces the bundles from `target/`; the release workflow's artifact globs match them.
- The 65 Tauri command names in `generate_handler!` are unchanged from `a18c724`.

**Not part of phase-b:**

- Implementing a beads-dolt SQL transport or a DoltHub transport (design headroom only, maintainer requirement 5).
- Splitting attachments, migration, polling, watcher, updates or probe out of `btit-app`.
- Changing the attachment folder layout (B8 is closed as documentation).
- The frontend double-warning question (B13, OQ-5).
- Structured-field logging call sites, `#[instrument]`, or any sc-observability-log API change.
- The "sc-lint" section, pending maintainer input.

## Open questions for the maintainer

| Id | Question | Default assumed by the sprint docs |
| --- | --- | --- |
| OQ-1 | Workspace `edition`: keep `2021` (today's app edition) for the phase-b moves, or adopt `2024` (the edition of `../atm-core`, `../sc-compose`, `../sc-observability` and the phase-a crates) now? | `2021`; a 2024 migration is a later change |
| OQ-2 | Package name of the Tauri crate: keep `beads-issue-tracker` (no artifact change), or rename to `btit-app` with `mainBinaryName = "beads-issue-tracker"` in `tauri.conf.json`? | keep `beads-issue-tracker`; directory `crates/btit-app` |
| OQ-3 | `sc-observability-log` dependency in b-1: crates.io version (preferred) or the git `rev` fallback? Depends on whether sc-observability has released when b-1 starts; the version/commit comes from `docs/plans/phase-a/handoff-a-6.md`. | crates.io if released, else git `rev` |
| OQ-4 | B7: does `br` support `list --all`? The doc comment (`cli.rs:409`) says no, the code (`cli.rs:416`) says yes; br source and binary are unavailable locally. | code unchanged; doc comment marked unverified |
| OQ-5 | B13 "double warning": should the frontend suppress the legacy banner when the version could not be parsed? Unverified in the UI. | out of scope for phase-b |
| OQ-6 | Gated logging: keep the `LOGGING_ENABLED`-gated `log_*!` macros (behaviour-preserving, moved to `btit-beads::logging`), or switch library crates to plain `log::` calls filtered by the bridge level? | keep the gate |
| OQ-7 | Should `bd_migrate_to_dolt`'s restore steps (`label add`, `dep add`, `comments add`, `migration.rs:944-1087`) keep their raw, un-JSON'd, un-locked invocations through `CliBackend::run_raw`, or go through `BeadsBackend::label_add`/`dep_add`/`comment_add` (which add `--json` and take the project lock)? | `run_raw`, byte-identical invocations |
