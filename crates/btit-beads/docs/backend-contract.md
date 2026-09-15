# Beads backend contract

`btit-beads` defines the contract every beads backend implements: the traits in
`src/backend.rs`, the error type in `src/error.rs`, and the pure logic every
backend shares. The decision is recorded as ADR-008 in `docs/architecture.md`.
The public API is pinned by `tests/api_freeze.rs`.

Source citations (`file:line`) refer to the pre-split app sources at `a18c724`
(`src-tauri/src/`, now `crates/btit-app/src/`). "bd" and "br" name the
implementing crates `btit-bd` and `btit-br`.

## Shape

- `BeadsBackend` is transport-neutral. It has no notion of a binary, a process,
  a version or `--json`. A future SQL transport against a beads Dolt server, or a
  DoltHub client, implements this trait alone.
- Everything transport- or backend-specific is reached through optional
  accessors that default to `None`: `cli()`, `dolt()`, `close_suggestions()`.
  The app holds `Arc<dyn BeadsBackend>` and never downcasts. A caller that needs
  CLI facts calls `backend.cli()` and maps `None` to `BeadsError::Unsupported`.
- All four traits are object-safe and `Send + Sync`.

## Transport-neutral methods

### `BeadsBackend`

| Method | Today's source | Notes |
| --- | --- | --- |
| `fn project_uses_dolt(&self, project: &ProjectRef) -> bool` | `project_uses_dolt` / `project_uses_dolt_for`, `cli.rs:473-515`; every caller derives `beads_dir` as `<working_dir>/.beads` | bd: resolves the working dir, then the full check; br: `false`; an unresolvable `ProjectRef` → `false` |
| `fn list(&self, project: &ProjectRef, query: &ListQuery) -> Result<Vec<BdRawIssue>, BeadsError>` | `bd_list` arg building and `--all` fallback, `issue_commands.rs:18-69`; `bd_count` two-call form `:81-90`; `bd_poll_data` `polling.rs:43-56` | `--limit=0` always; two-call fallback when `!supports_list_all_flag` |
| `fn ready(&self, project: &ProjectRef) -> Result<Vec<BdRawIssue>, BeadsError>` | `bd_ready`, `issue_commands.rs:139-141` | |
| `fn status(&self, project: &ProjectRef) -> Result<serde_json::Value, BeadsError>` | `bd_status`, `:149-152` | |
| `fn show(&self, project: &ProjectRef, id: &str) -> Result<Option<BdRawIssue>, BeadsError>` | `bd_show`, `:162-205` | not-found stderr and empty stdout → `None`; array-or-object; strict deserialize error |
| `fn create(&self, project: &ProjectRef, payload: &CreatePayload) -> Result<BdRawIssue, BeadsError>` | `bd_create`, `:214-274` | flag mapping incl. `priority_to_number` |
| `fn update(&self, project: &ProjectRef, id: &str, updates: &UpdatePayload) -> Result<Option<BdRawIssue>, BeadsError>` | `bd_update`, `:285-394` | empty stdout → `show` fallback; lenient `.ok()` parse |
| `fn close(&self, project: &ProjectRef, id: &str) -> Result<serde_json::Value, BeadsError>` | `bd_close`, `:409-423` | br impl adds `--suggest-next` via `CloseSuggestions` |
| `fn search(&self, project: &ProjectRef, query: &str) -> Result<Vec<BdRawIssue>, BeadsError>` | `bd_search`, `:433-447` | empty / `[]` → empty vec; strict `Vec<BdRawIssue>` |
| `fn label_add(&self, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError>` | `:455-456` | |
| `fn label_remove(&self, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError>` | `:463-464` | |
| `fn delete(&self, project: &ProjectRef, id: &str) -> Result<(), BeadsError>` | `bd_delete` args `:470-475` | `--force` plus `--hard` when `supports_delete_hard_flag`; the caller removes the attachment folder (`:480-504`) |
| `fn comment_add(&self, project: &ProjectRef, id: &str, content: &str) -> Result<(), BeadsError>` | `bd_comments_add`, `:511-513` | |
| `fn dep_add(&self, project: &ProjectRef, issue_id: &str, depends_on_id: &str, relation_type: Option<&str>) -> Result<(), BeadsError>` | `bd_dep_add` `:520-522`, `bd_dep_add_relation` `:538-540` | `--type <t>` when `Some` |
| `fn dep_remove(&self, project: &ProjectRef, issue_id: &str, depends_on_id: &str) -> Result<(), BeadsError>` | `bd_dep_remove` `:529-531`, `bd_dep_remove_relation` `:547-549` | |
| `fn relation_types(&self) -> Vec<RelationType>` | `bd_available_relation_types`, `:555-581` | br: common 7; bd/unknown: common + `tracks`, `until`, `validates` |
| `fn sync(&self, project: &ProjectRef) -> Result<(), BeadsError>` | `sync_bd_database` spawn `migration.rs:222-232`; `bd_sync` `:278-288` | `sync [--no-daemon]`, no `--json`, no project lock (as today); non-zero exit → `Err(CommandFailed)` |
| `fn cli(&self) -> Option<&dyn CliBackend>` (default `None`) | new accessor | bd, br: `Some(self)`; a non-CLI transport: `None` |
| `fn dolt(&self) -> Option<&dyn DoltOperations>` (default `None`) | new accessor | bd: `Some(self)`; br: `None` |
| `fn close_suggestions(&self) -> Option<&dyn CloseSuggestions>` (default `None`) | new accessor | br: `Some(self)`; bd: `None` |

### `DoltOperations` (bd only today)

Transport-neutral: every method returns `btit_types::DoltOpResult { success, message, detail }`,
which is what today's callers read from the process (`status.success()`,
`stdout.trim()`, `stderr.trim()`), never raw process output.

| Method | Today's source | What callers read (→ `DoltOpResult` field) |
| --- | --- | --- |
| `fn doctor_fix(&self, project: &ProjectRef) -> Result<DoltOpResult, BeadsError>` | `bd_repair_database` Dolt path, `migration.rs:333-339` (`doctor --fix --yes`) | `status.success()` → `success`; `stdout.trim()` → `message` (`:342-346`); `stderr.trim()` → `detail` (`:350-352`) |
| `fn migrate_to_dolt(&self, project: &ProjectRef) -> Result<DoltOpResult, BeadsError>` | `bd_migrate_to_dolt`, `:623-629` (`migrate --to-dolt --yes`) | `success`; `stdout.trim()` → `message` (`:632-636`); `stderr.trim()` → `detail`, reused in the init-fallback error (`:642-643,680-682`) |
| `fn init(&self, project: &ProjectRef, prefix: &str) -> Result<DoltOpResult, BeadsError>` | `:665-671`, `:771-777` (`init --prefix <p>`) | `success`; `stderr.trim()` → `detail` (`:679-682,779-781`) |
| `fn import_jsonl(&self, project: &ProjectRef, file: &Path) -> Result<DoltOpResult, BeadsError>` | `:879-885` (`import -i <file>`) | `success`; `stderr.trim()` → `detail` (`:890-900`); `stdout.trim()` → `message` (`:903-904`) |

Spawn failures are `Err(BeadsError::Spawn { operation: Some(..), .. })`, so the app keeps
`Failed to run bd doctor|migrate|init|import: {e}` (`:339,629,671,885`).

## CLI-only methods

### `CliBackend: BeadsBackend` (bd and br)

| Method | Today's source | Notes |
| --- | --- | --- |
| `fn binary(&self) -> String` | `get_cli_binary`, `config.rs:58-60` | the configured name/path (owned, as `get_cli_binary` returns today) |
| `fn probe(&self) -> Option<CliProbe>` | `probe_cli_binary`, `cli.rs:185-196` | fresh `--version` run from the temp dir |
| `fn client(&self) -> CliClient` | `get_cli_client_info` client part, `cli.rs:326-365` | from the cached probe; `Unknown` when the probe failed |
| `fn version(&self) -> Option<CliVersion>` | `get_cli_client_info` tuple part | same |
| `fn capabilities(&self) -> BackendCapabilities` | `supports_daemon_flag`, `uses_jsonl_files`, `supports_list_all_flag`, `supports_delete_hard_flag`, `uses_dolt_backend` wrappers, `cli.rs:380-385,400-405,421-426,441-446,461-466` | computed with `btit_beads::gates::capabilities_for` from the cached probe |
| `fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError>` | `new_command(..).args(..).current_dir(..).env("PATH",..).env("BEADS_PATH",..).output()` pattern, `migration.rs:227-232,282-288,333-339,403-408,623-629,665-671,771-777,879-885,944-950,1003-1009,1081-1087` | no `--json`, no lock; returns status, stdout, stderr |
| `fn release_source(&self) -> ReleaseSource` | `check_bd_cli_update`, `updates.rs:285-292` | bd/unknown: `steveyegge/beads`; br: `Dicklesworthstone/beads_rust`. No app caller in phase-b; contract headroom |

The client kind, version and version gates are on `CliBackend`, not `BeadsBackend`,
because a SQL transport has no equivalent.

### `CloseSuggestions` (br only)

| Method | Today's source |
| --- | --- |
| `fn close_suggesting_next(&self, project: &ProjectRef, id: &str) -> Result<serde_json::Value, BeadsError>` | `bd_close` br branch, `issue_commands.rs:410-413` (`close <id> --suggest-next`) |

## Errors

`BeadsError` is the only error type that crosses crate boundaries. It is a
`#[non_exhaustive]` enum; every variant is `Debug`, and `source()` returns the inner
`io::Error`/`serde_json::Error` where present. `Display` reproduces today's command
error strings byte for byte, so the Tauri commands keep `Result<_, String>` at the
IPC edge with `map_err(|e| e.to_string())`. Adding a variant is a plan change.

| Variant | Fields | `Display` | `code()` | `remediation()` |
| --- | --- | --- | --- | --- |
| `Spawn` | `binary: String`, `operation: Option<String>`, `source: io::Error` | `None`: `Failed to execute {binary}: {source}`; `Some(op)`: `Failed to run {binary} {op}: {source}` | `BTIT_BEADS_SPAWN` | Install the CLI or point Settings at its path; the searched directories are in check_bd_compatibility.searchedPaths. |
| `CommandFailed` | `binary: String`, `status: Option<i32>`, `status_display: String`, `stderr: String` | `stderr` when non-empty; else `bd command failed with status: {status_display}` | `BTIT_BEADS_COMMAND_FAILED` | Read stderr; run the same command in a terminal from the project directory. |
| `SchemaMigration` | `binary: String` | `SCHEMA_MIGRATION_ERROR: Database schema is incompatible. Please use the repair function to fix this issue.` | `BTIT_BEADS_SCHEMA_MIGRATION` | Use Repair database (bd_repair_database). |
| `InvalidJson` | `context: String`, `source: serde_json::Error` | `Invalid JSON: {source}` | `BTIT_BEADS_INVALID_JSON` | Upgrade the CLI; the output is not JSON even with --json. |
| `UnexpectedShape` | `context: String`, `expected: ExpectedShape` | `Array` → `Expected JSON array`; `ArrayOrEnvelope` → `Expected JSON array or paginated envelope` | `BTIT_BEADS_UNEXPECTED_SHAPE` | Upgrade the CLI; the JSON shape is not one btit knows. |
| `ParseFailed` | `target: ParseTarget`, `id: Option<String>`, `source: serde_json::Error` | `Status` → `Failed to parse status: {e}`; `Issue` → `Failed to parse issue: {e}` or, with an id, `Failed to parse issue {id}: {e}`; `CreatedIssue` → `Failed to parse created issue: {e}`; `UpdatedIssueFetch` → `Failed to fetch updated issue: {e}`; `UpdatedIssue` → `Failed to parse updated issue: {e}`; `CloseResult` → `Failed to parse close result: {e}`; `SearchResults` → `Failed to parse search results: {e}` | `BTIT_BEADS_PARSE_FAILED` | Report the CLI version and the raw output from the log. |
| `Unsupported` | `operation: &'static str`, `client: CliClient` | `{operation} is not supported by the {client} client` (`bd`, `br` or `unknown`) | `BTIT_BEADS_UNSUPPORTED` | Switch the CLI binary in Settings. |

## `ProjectRef` headroom

`btit_types::ProjectRef` is a `#[non_exhaustive]` enum with the single variant
`Local { cwd: Option<String> }` (today's `cwd: Option<&str>`). A remote transport
adds a variant; callers that only pass a `ProjectRef` through do not change.

## Synchronous traits, blocking calls

The traits are synchronous, matching today's blocking `Command::output()` calls
inside `async` Tauri commands (`cli.rs:555-560`). A `#[tauri::command] async fn`
calling a backend therefore blocks the async runtime for the child's lifetime, as
it does today. An async twin is not planned; moving the calls to `spawn_blocking`
at the `btit-app` command boundary is tracked in issue #55.

## Dependency rules

- `btit-beads` depends only on `btit-types`, `log` and `serde_json`.
- Only `btit-app` depends on `tauri` and on `sc-observability-log`. Library crates
  log through the `log` facade; `btit_beads::logging` gates those calls on
  `LOGGING_ENABLED`/`VERBOSE_LOGGING` and does not wrap the bridge.

## No process-global client state in library crates

`btit-beads`, `btit-cli`, `btit-bd` and `btit-br` declare no `static` mutable state
other than the two logging switches `btit_beads::logging::{LOGGING_ENABLED,
VERBOSE_LOGGING}`. Backends are instances that own their binary and probe cache,
so two backends (for example bd for one project and br for another) can coexist
in one process. The only backend global is the app's slot, reached through a
single function in `btit-app`; per-project selection would replace that
function's body without touching any trait or backend crate.

## Future backend-specific traits

Backend-specific capabilities are added as further traits behind an optional
accessor with the shape of `dolt()`, never as `Unsupported` methods on
`BeadsBackend`. Issue #51 (Dolt commit log, `AS OF` snapshots and `dolt_diff`
over the `issues`, `dependencies`, `labels` and `comments` tables) is bd-only
headroom of this kind: a trait next to `DoltOperations`, implemented by `btit-bd`
only and `None` for `btit-br`. No method, type or sprint for it is planned.

A SQL transport can implement `dolt()` as well as `BeadsBackend`, because
`DoltOpResult` carries no process output: only `success`, `message` and `detail`.
