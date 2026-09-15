---
id: b-4
title: btit-cli crate — shared CLI transport, issue-operation bodies, and the btit-bd/btit-br skeletons
status: complete
branch: feature/sprint-b-4-btit-cli
worktree: ../beads-task-issue-tracker-worktrees/feature/sprint-b-4-btit-cli
target: integrate/phase-b
recommended_model: higher-effort (largest code motion; process, lock and PATH semantics must stay identical)
dependency_relations:
  - prerequisite: b-3
    dependent: b-4
    relation: must_follow
    rationale: "CliRunner returns BeadsError, uses parse/gates/detect and the log_*! macros from btit-beads; stack parent"
  - prerequisite: b-4
    dependent: b-5
    relation: must_follow
    rationale: "BdCli wraps CliRunner, ops and testing::RecordingInvoker and fills the btit-bd skeleton; group A forks from the b-4 head"
  - prerequisite: b-4
    dependent: b-6
    relation: must_follow
    rationale: "BrCli wraps the same and fills the btit-br skeleton; group A forks from the b-4 head"
---

# Sprint b-4 — `btit-cli`: shared CLI transport, issue-operation bodies, and the `btit-bd`/`btit-br` skeletons

## Recommended Agent / Model

Recommended model: higher-effort (largest code motion; process, lock and PATH semantics must stay identical).
Recommended agent: not set — the btit developer pane is still `tbd` in `.atm.toml`.
Planning advice; team-lead assigns from the active pool.

## Goal

- Create `crates/btit-cli`, the transport both CLIs share: extended `PATH`, `new_command`, the per-project lock, the JSON/raw invocation functions, the `--version` probe and auto-detection, and the issue-operation bodies that `issue_commands.rs`, `polling.rs`, `attachments.rs` and `migration.rs` execute through `execute_bd` today.
- Provide `CliRunner`, the per-binary instance that `BdCli` (b-5) and `BrCli` (b-6) wrap, and `testing::RecordingInvoker` for their tests.
- Pre-register the `btit-bd` and `btit-br` crates as empty skeletons with their final manifests, lockfile entries and CI rows, so that b-5 and b-6 (group A) never edit a common file.
- The app adopts the crate immediately through a transitional `AppInvoker` adapter over its existing statics, so no invocation logic exists twice. `execute_bd` becomes a thin wrapper.

## Hard Dependencies

- b-3 pushed (`BeadsError`, `capabilities_for`, `parse_issues_tolerant`, `detect::*`, `log_*!` macros).

## Dependency Relations

Trigger definitions, per-branch QA and fix-layer rules: `plan-phase-b.md` "Dependency relations" and "Parallel groups: fork and re-merge".

- b-3 → b-4 — `must_follow`.
- b-4 → b-5, b-4 → b-6 — `must_follow`; both, with b-9, form group A and fork from this branch's head once this sprint's acceptance criteria are met and QA-1 has no Blocking finding.

Stack: `phase-b-core` · layer 4.

## Exact Targets

Line numbers are at `a18c724` (`crates/btit-app/src/` after b-1).

- `Cargo.toml` (root): members `crates/btit-cli`, `crates/btit-bd`, `crates/btit-br`
- `Cargo.lock` (root): refreshed with the three new packages and the skeletons' full dependency sets
- `crates/btit-cli/Cargo.toml` (with `[features] test-support = []`), `clippy.toml`, `src/lib.rs`, `src/path.rs`, `src/command.rs`, `src/locks.rs`, `src/run.rs`, `src/probe.rs`, `src/runner.rs`, `src/ops.rs`, `src/testing.rs` (`#[cfg(feature = "test-support")]`), `tests/api_freeze.rs`
- `crates/btit-bd/Cargo.toml`, `crates/btit-bd/clippy.toml`, `crates/btit-bd/src/lib.rs` (skeleton); `crates/btit-br/Cargo.toml`, `crates/btit-br/clippy.toml`, `crates/btit-br/src/lib.rs` (skeleton)
- Moved out of `crates/btit-app/src/cli.rs`: 14-15 (`BD_PROJECT_LOCKS` → `ProjectLocks`), 22-59 (`get_extended_path`), 63-72 (`new_command`), 185-196 (`probe_cli_binary`), 200-207 (`extended_path_entries`), 255-281 (`default_cli_binary`), 523-592 (`execute_bd` body → `run_json`); tests 754-758, 804-817, 846-874, 1116-1161, 1222-1238
- Pattern copied (not moved) from `crates/btit-app/src/config.rs:65-69,129-133` (the `--version` spawn) → `probe_version_output`; `config.rs` itself is rewired in b-7
- Moved out of `crates/btit-app/src/issue_commands.rs`: the bodies of `bd_list` (18-72), `bd_count` fetch (81-90), `bd_ready` (139-141), `bd_status` (149-152), `bd_show` (162-208), `bd_create` (214-276), `bd_update` (285-402), `bd_close` (409-426), `bd_search` (433-449), `bd_label_add`/`bd_label_remove` (455-456, 463-464), `bd_delete` CLI part (470-475), `bd_comments_add` (511-513), `bd_dep_add`/`bd_dep_remove`/`bd_dep_add_relation`/`bd_dep_remove_relation` (520-551), `bd_available_relation_types` table (556-578) → `ops.rs`
- Moved out of `crates/btit-app/src/migration.rs`: the sync invocation 222-232 → `ops::sync`
- `crates/btit-app/src/cli.rs`: `execute_bd` becomes a wrapper; new `pub(crate) struct AppInvoker;` implementing `btit_cli::CliInvoker` over `config::CLI_BINARY`, `CLI_CLIENT_INFO` and a `static PROJECT_LOCKS: LazyLock<Arc<ProjectLocks>>`
- `crates/btit-app/src/issue_commands.rs`, `polling.rs:43-60`, `attachments.rs:189-191`, `migration.rs:222-250`: call `btit_cli::ops::*` through `AppInvoker`
- `crates/btit-app/src/attachments.rs:50` (`crate::cli::new_command("cmd")`) and `crates/btit-app/src/updates.rs:1,75` (`use crate::cli::{.., new_command, ..}`, `new_command("gh")`): rewired to `btit_cli::command::new_command`, keeping `CREATE_NO_WINDOW` on Windows
- `crates/btit-app/src/config.rs:1` (`use crate::cli::{default_cli_binary, get_extended_path, new_command, reset_bd_version_cache}`) and `crates/btit-app/src/migration.rs:4` (`use crate::cli::{get_cli_client_info, get_extended_path, new_command, project_uses_dolt, supports_daemon_flag, uses_jsonl_files}`): the `new_command`/`get_extended_path` names now come from `btit_cli::{command::new_command, path::get_extended_path}` (and `default_cli_binary` from `btit_cli::probe`); the remaining names stay `crate::cli::…`; call sites unchanged
- `crates/btit-app/src/lib.rs:41,53,64` (`cli::get_extended_path()`, `cli::probe_cli_binary(&binary)`, `cli::extended_path_entries()`): rewired to `btit_cli::path::get_extended_path()`, `btit_cli::probe::probe_cli_binary(&binary)`, `btit_cli::path::extended_path_entries()`
- `crates/btit-app/src/config.rs:12` (`#[serde(default = "crate::cli::default_cli_binary")]`): rewired to a local `fn default_cli_binary() -> String { btit_cli::probe::default_cli_binary() }` in `config.rs` and `#[serde(default = "default_cli_binary")]` (serde's `default` takes a path, so the local wrapper keeps the attribute valid after the move)
- `.github/workflows/ci.yml`: `rust-quality` gains `btit-cli`, `btit-bd`, `btit-br` in every step (fmt, clippy, rustdoc, `cargo tree` gates)
- `docs/plans/phase-b/sprint-b-4.md` (`status:` frontmatter only)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for the scope this sprint claims. If that cannot be done cleanly in one sprint, the sprint must be split before implementation begins. No deliverable may be silently dropped or partially deferred.

1. **Crate.** `crates/btit-cli`, `[lints] workspace = true`, `#![deny(missing_docs)]`, `publish = false`. Dependencies: `btit-types`, `btit-beads`, `serde_json`, `log`. No Tauri, no `sc-observability-log`. Cargo feature `test-support` (no dependencies) gates `pub mod testing`.
2. **`path.rs`.** `get_extended_path` and `extended_path_entries` moved verbatim (`cli.rs:22-59, 200-207`), including the per-OS extra directories and separators. `command.rs`: `new_command` verbatim (`CREATE_NO_WINDOW` on Windows, `cli.rs:63-72`).
3. **`locks.rs`.** `pub struct ProjectLocks` wrapping today's `Mutex<HashMap<String, Arc<Mutex<()>>>>` (`cli.rs:14-15`) with `fn guard(&self, working_dir: &str) -> Arc<Mutex<()>>`. Poisoning is recovered with `PoisonError::into_inner` where today's code calls `.unwrap()` (`cli.rs:548,553`); this is the one behaviour delta (no panic on a poisoned lock) and is listed in the PR.
4. **`run.rs`.** `resolve_working_dir(project: &ProjectRef, client: CliClient) -> Result<String, BeadsError>` (cwd → `BEADS_PATH` → `current_dir()` → `"."`, `cli.rs:524-531`; the `#[non_exhaustive]` wildcard arm returns `BeadsError::Unsupported { operation: "non-local project reference", client }`). `run_json(binary, no_daemon: bool, locks: &ProjectLocks, working_dir: &str, command: &str, args: &[String]) -> Result<String, BeadsError>` is `execute_bd`'s body (`cli.rs:533-591`): subcommand split, `--no-daemon` when `no_daemon`, `--json`, the `[bd] …` log lines, the project lock, `PATH`/`BEADS_PATH` env, `SchemaMigration` on `no such column: spec_id`, `CommandFailed` with stderr, the `VERBOSE_LOGGING` preview. `run_raw(binary, working_dir, args: &[&str]) -> Result<CliOutput, BeadsError>`: `new_command(binary).args(args).current_dir(working_dir).env("PATH", ..).env("BEADS_PATH", working_dir).output()`, mapping the spawn error to `Spawn { operation: args.first().map(ToString::to_string) }` and any exit status to `Ok(CliOutput)` (the caller decides, as `migration.rs` does today). **`json_argv(command: &str, args: &[String], no_daemon: bool) -> Vec<String>`** is the single pure assembler of a JSON invocation's argv (`command.split_whitespace()`, then `args`, then `--no-daemon` if `no_daemon`, then `--json`, `cli.rs:534-541`), and **`json_invocation(info: Option<&CliProbe>, command: &str, args: &[String]) -> (CliClient, Vec<String>)`** is the single pure step from a cached probe to an invocation: it returns the client for `resolve_working_dir` (`Unknown` for `None`) and `json_argv(command, args, capabilities_for(p.client, p.version).supports_daemon_flag)` (`false` for `None`). `CliRunner::run_json` spawns exactly the argv `json_invocation` returns and `RecordingInvoker::run_json` records exactly it, so the argv table below is asserted above the spawn seam. A unit test `json_invocation_literal_outputs` pins `Some(Bd 0.49.6)` → `(Bd, ["ready", "--no-daemon", "--json"])`, `Some(Br 0.1.33)` → `(Br, ["ready", "--json"])`, `None` → `(Unknown, ["ready", "--json"])` for `("ready", &[])`. `probe_version_output(binary) -> Result<CliOutput, BeadsError>`: `--version` from `std::env::temp_dir()` with the extended `PATH` (`config.rs:65-69,129-133`, `cli.rs:186-190`).
5. **`probe.rs`.** `probe_cli_binary(binary) -> Option<CliProbe>` (`cli.rs:185-196`, via `probe_version_output` + `parse_cli_probe`) and `default_cli_binary() -> String` (`cli.rs:255-281`, ungated `log::info!/warn!` kept).
6. **`runner.rs`.** The `CliInvoker` trait (`binary`, `client_info`, `probe`, `run_json`, `run_raw`, with the provided `capabilities`/`client`/`version`) and `CliRunner` exactly as in the code samples. `CliRunner::client_info()` reproduces `get_cli_client_info` (`cli.rs:326-365`): probe lazily, cache only a parsed success, log the same lines. `CliRunner::capabilities()` = `capabilities_for(client, version)`. `CliRunner::with_probe(binary, locks, probe)` pre-seeds the cache; it is production API (b-7's factory and `check_bd_compatibility` seed the rebuilt runner from the fresh probe instead of spawning `--version` twice) and also serves tests.
7. **`ops.rs`.** One function per operation, generic over `&dyn CliInvoker`, with today's bodies, where every former global read is replaced by the invoker accessor (`supports_list_all_flag()` → `inv.capabilities().supports_list_all_flag`, `supports_delete_hard_flag()` → the `hard` argument, `get_cli_client_info()` client match → `inv.client()`, `supports_daemon_flag()` inside `execute_bd` → `run_json`'s `no_daemon`): `list` (with the `--all` two-call fallback when `!supports_list_all_flag`, `issue_commands.rs:22-39`), `ready`, `status`, `show` (not-found via `e.to_string().to_lowercase()` containing `no issue found`/`not found`, empty stdout, array-or-object, strict deserialize → `ParseFailed { target: Issue, id: Some(id) }`), `create`, `update` (empty stdout → `show` fallback with lenient `.ok()`), `close(inv, project, id, suggest_next: bool)`, `search`, `label_add`, `label_remove`, `delete(inv, project, id, hard: bool)`, `comment_add`, `dep_add(.., relation_type: Option<&str>)`, `dep_remove`, `relation_types(client) -> Vec<RelationType>` (`issue_commands.rs:556-578`), `sync(inv, project, no_daemon) -> Result<(), BeadsError>` (`migration.rs:222-232`, non-zero exit → `CommandFailed`). Log lines keep their text; the `context` label of `parse_issues_tolerant` calls may become the op name (plan "Behaviour preserved").
8. **`testing.rs` (feature `test-support`).** `pub struct RecordingInvoker` implementing `CliInvoker` with a seeded `Option<CliProbe>` (`None` is the no-probe column: `client_info()` returns `None`, so `client()` is `Unknown` and `capabilities()` all `false`), a queue of scripted `Result<String, BeadsError>`/`Result<CliOutput, BeadsError>` replies, a recorded `Vec<Vec<String>>` of the **full argv** of every call (the argv assembled by `run::json_invocation` for `run_json`), and an `AtomicUsize` count of `probe()` calls exposed as `probe_calls()`: `run_json` records the argv half of `run::json_invocation(self.probe.as_ref(), command, args)` (the same step `CliRunner::run_json` spawns), `run_raw` records `args` verbatim. Used by this crate's own tests and, through `btit-cli = { features = ["test-support"] }` under `[dev-dependencies]`, by `btit-bd` and `btit-br`.
9. **Skeleton crates `btit-bd` and `btit-br`.** Each: `Cargo.toml` with the final `[dependencies]` (`btit-types`, `btit-beads`, `btit-cli`, `serde_json`, `log`), `[features] test-support = ["btit-cli/test-support"]` (a crate feature, not `#[cfg(test)]`, so `with_invoker` is usable from the app's tests via `btit-bd = { features = ["test-support"] }` under the app's `[dev-dependencies]` in b-8), `[dev-dependencies] btit-cli = { path = "../btit-cli", features = ["test-support"] }`, `[lints] workspace = true`, `publish = false`; `clippy.toml` with the four `allow-*-in-tests` keys; `src/lib.rs` containing only `#![deny(missing_docs)]` and the crate doc comment. Both are workspace members; `cargo check --workspace` succeeds; the root `Cargo.lock` carries their entries and dependency edges (a feature adds no package, so the `cargo tree -e normal` expectations are unchanged). b-5 and b-6 add code inside these directories only and never touch the root manifests, the lockfile or the workflow.
10. **App adoption, no duplicated logic.** `execute_bd` = `run_json(&get_cli_binary(), supports_daemon_flag(), &PROJECT_LOCKS, &wd, command, args).map_err(|e| e.to_string())` with `wd` from `resolve_working_dir`. The import lines `config.rs:1` and `migration.rs:4` take `new_command`/`get_extended_path` (and `default_cli_binary`) from `btit-cli`; `lib.rs:41,53,64` call `btit_cli::path::get_extended_path`, `btit_cli::probe::probe_cli_binary`, `btit_cli::path::extended_path_entries`; `config.rs:12`'s serde default points at the local `default_cli_binary()` wrapper over `btit_cli::probe::default_cli_binary`; their call sites (`config.rs:65-69,129-133`, `migration.rs:227-232,282-288,333-339,403-408,623-629,665-671,771-777,879-885,944-950,1003-1009,1081-1087`) are unchanged. `AppInvoker` implements `CliInvoker` over the statics (transitional; deleted by b-7). The two non-beads spawns that use `crate::cli::new_command` today, `attachments.rs:50` (`cmd /C start`) and `updates.rs:75` (`gh auth token`, imported at `updates.rs:1`), switch to `btit_cli::command::new_command`; `CREATE_NO_WINDOW` behaviour on Windows is unchanged because the function moved verbatim. Every `#[tauri::command]` in `issue_commands.rs` keeps its signature and calls the `ops` function, keeping its own log lines, `transform_issue` mapping and `map_err(|e| e.to_string())`; `bd_delete` keeps the attachment-folder cleanup; `bd_available_relation_types` maps `RelationType` to the same `{"value","label"}` JSON. `polling.rs` `bd_poll_data` uses `ops::list` with `include_all: true` and partitions on `status != "closed"` (today's `--all` path; for the two-call fallback the merged list contains the same issues, `polling.rs:43-56`). `attachments.rs` `purge_orphan_attachments` uses `ops::list(include_all: true)` (today an unconditional `--all` call, `attachments.rs:189`; on bd < 0.55 this now takes the fallback — legacy-only delta listed in the PR). `migration.rs` `sync_bd_database`/`bd_sync` call `ops::sync` and map the result to today's log/return text (table in Required Work).
11. **Tests moved and added.** The cli.rs tests in Exact Targets move to `btit-cli`. New unit tests cover the pure pieces this sprint introduces: `resolve_working_dir` precedence (cwd, `BEADS_PATH`, current dir), the arg vectors built by each `ops` function through `RecordingInvoker` (asserting, for example, `list` with `include_all` on a `supports_list_all_flag = false` invoker issues `list --limit=0` then `list --limit=0 --status=closed`, `issue_commands.rs:25-33`), `show`'s not-found and shape handling, `update`'s empty-output fallback, `close` with and without `--suggest-next`, `delete` with and without `--hard`, `relation_types` for `Br` vs others.
12. **API freeze and CI.** `crates/btit-cli/tests/api_freeze.rs` pins `CliInvoker` (all five required methods including `probe`), `CliRunner`, `run::json_argv`, `run::json_invocation`, `RecordingInvoker::new(Option<CliProbe>)`, `RecordingInvoker::calls() -> Vec<Vec<String>>` and every `ops` signature; `rust-quality` covers `btit-cli`, `btit-bd`, `btit-br` (the skeletons pass fmt/clippy/rustdoc trivially and their `cargo tree` gates already assert the final dependency sets). The command-signature gate (1) and the frontend invoke-subset gate (2) from the plan ("Command contract gates") pass.

## Required Work

- **`sync` result mapping in the app** (byte-exact with `migration.rs:234-250, 290-300`):

  | `ops::sync` result | `sync_bd_database` | `bd_sync` |
  |---|---|---|
  | `Ok(())` | `log_info!("[sync] Sync completed successfully")`; cooldown updated | same log; cooldown updated; `Ok(())` |
  | `Err(CommandFailed { stderr, .. })` | `log_warn!("[sync] {} sync failed: {}", binary, stderr)` | `log_error!("[bd_sync] Sync failed: {}", stderr.trim())`; `Err(format!("Sync failed: {}", stderr.trim()))` |
  | `Err(Spawn { source, .. })` | `log_error!("[sync] Failed to run {} sync: {}", binary, source)` | `Err(format!("Failed to run {} sync: {}", binary, source))` |

- `ops` functions take `project: &ProjectRef`; the app constructs `ProjectRef::local(options.cwd)`.
- **Authoritative argv table** (every cell is a `RecordingInvoker` assertion in `crates/btit-cli/tests/ops_argv.rs`; the recorded argv is `run::json_argv(command, args, no_daemon)`, i.e. the subcommand words, the op's args, `--no-daemon` only when `supports_daemon_flag` — bd < 0.50 — and `--json` last, `cli.rs:534-541`). Columns are the seeded probes `Bd 1.0.4`, `Bd 0.54.0`, `Bd 0.49.6`, `Br 0.1.33`, `Unknown 9.9.9`, and *no probe* (`RecordingInvoker::new(None)`: `client_info() == None`, all gates `false`, client `Unknown`). The `--all` vs two-call expectation and every capability-driven flag in the tests are computed from `btit_beads::gates::capabilities_for(client, version)` for the column's probe, not written as literals, so a later change to a gate (b-9, B4/B5/B7) moves the expectation with it; the table shows the values those gates return at `a18c724`. Three cells in columns b-9 cannot move are additionally asserted as literals in `ops_argv.rs`, so a derivation bug cannot pass every test: `ready` for Bd 0.49.6 records exactly `["ready", "--no-daemon", "--json"]`; `list(include_all)` for Bd 0.54.0 records exactly the two calls `["list", "--limit=0", "--json"]` then `["list", "--limit=0", "--status=closed", "--json"]`; `delete(id, hard)` for Bd 0.49.6 records exactly `["delete", id, "--force", "--hard", "--no-daemon", "--json"]`.

  | `ops` call | Bd 1.0.4 | Bd 0.54.0 | Bd 0.49.6 | Br 0.1.33 | Unknown 9.9.9 / no probe |
  |---|---|---|---|---|---|
  | `list(q)` with `status=[open]`, `type=[bug]`, `priority=[p1]`, `assignee=a`, `include_all=None` | `list --status=open --type=bug --priority=1 --assignee=a --limit=0 --json` | same | same + `--no-daemon` before `--json` | same as Bd 1.0.4 | same as Bd 1.0.4 |
  | `list(q)` with `include_all=Some(true)`, no filters (`issue_commands.rs:20-43`) | `list --all --limit=0 --json` | two calls: `list --limit=0 --json`, `list --limit=0 --status=closed --json` | two calls, each `… --no-daemon --json` | `list --all --limit=0 --json` (B7/OQ-4) | two calls |
  | `ready()` | `ready --json` | same | `ready --no-daemon --json` | `ready --json` | `ready --json` |
  | `status()` | `status --json` | same | `status --no-daemon --json` | `status --json` | `status --json` |
  | `show(id)` | `show <id> --json` | same | `show <id> --no-daemon --json` | `show <id> --json` | `show <id> --json` |
  | `create(p)` (all payload fields set, `issue_commands.rs:214-269`) | `create <title> --description <d> --type <t> --priority <n> --assignee <a> --labels <l1,l2> --external-ref <e> --estimate <m> --design <d> --acceptance <a> --notes <n> --parent <p> --spec-id <s> --json` | same | same + `--no-daemon` | same | same |
  | `update(id, u)` (all fields set, `:285-346`) | `update <id> --title <t> --description <d> --type <t> --status <s> --priority <n> --assignee <a> --set-labels <l> --external-ref <e> --estimate <m> --design <d> --acceptance <a> --notes <n> --metadata <j> --spec-id <s> --parent <p> --json` | same | same + `--no-daemon` | same | same |
  | `update(id, u)` when stdout is empty (`:354-373`) | second call `show <id> --json` | same | `show <id> --no-daemon --json` | `show <id> --json` | `show <id> --json` |
  | `close(id, suggest_next)` | `close <id> --json` (`false`) | same | `close <id> --no-daemon --json` | `close <id> --suggest-next --json` (`true`) | `close <id> --json` |
  | `search(q)` | `search <q> --json` | same | `search <q> --no-daemon --json` | `search <q> --json` | `search <q> --json` |
  | `label_add` / `label_remove` | `label add <id> <label> --json` / `label remove <id> <label> --json` | same | + `--no-daemon` | same | same |
  | `delete(id, hard)` (`:470-475`) | `delete <id> --force --json` (`hard=false`) | same | `delete <id> --force --hard --no-daemon --json` (`hard=true`) | `delete <id> --force --json` | `delete <id> --force --json` |
  | `comment_add` | `comments add <id> <text> --json` | same | + `--no-daemon` | same | same |
  | `dep_add(a, b, None)` / `dep_add(a, b, Some(t))` | `dep add <a> <b> --json` / `dep add <a> <b> --type <t> --json` | same | + `--no-daemon` | same | same |
  | `dep_remove(a, b)` | `dep remove <a> <b> --json` | same | + `--no-daemon` | same | same |
  | `sync(no_daemon)` (`run_raw`, `migration.rs:222-232`) | `sync` | `sync` | `sync --no-daemon` | `sync` | `sync` |
  | `relation_types(client)` | common 7 + `tracks`, `until`, `validates` | same | same | common 7 | common 7 + 3 |

  The `hard`/`suggest_next`/`no_daemon` arguments are supplied by the backends from `capabilities()`/client kind (b-5, b-6), so the per-version columns are what those backends produce end-to-end; b-5 and b-6 repeat the table as full-method parity tests through `with_invoker(Box::new(RecordingInvoker::new(..)))`. Footnote: the *no probe* column above is the generic (`BdCli`) behaviour; for a `BrCli` built over a no-probe invoker (b-6 D5) `close` still records `--suggest-next` (the flag is br's, not version-gated) and `list(include_all)` takes the two-call path because `capabilities_for(Unknown, None)` has `supports_list_all_flag == false`.
- Changelog lines (collated by b-12): "New crate `btit-cli`: the process transport shared by bd and br (extended PATH, per-project lock, `--json` invocation) and the shared issue-operation bodies."

## Explicit Code Samples

```rust
// crates/btit-cli/src/runner.rs
use std::sync::{Arc, Mutex, PoisonError};
use btit_beads::{error::BeadsError, gates::capabilities_for};
use btit_types::{BackendCapabilities, CliClient, CliOutput, CliProbe, CliVersion, ProjectRef};
use crate::locks::ProjectLocks;

/// What `ops` needs from a CLI: identity, cached version info and the two invocation forms.
/// Implemented by `CliRunner`, by `testing::RecordingInvoker`, and, until b-7, by the app's transitional `AppInvoker`.
pub trait CliInvoker: Send + Sync {
    fn binary(&self) -> String;
    /// Cached `(client, version)`; probes lazily like `get_cli_client_info` (cli.rs:326-365).
    fn client_info(&self) -> Option<CliProbe>;
    fn capabilities(&self) -> BackendCapabilities {
        match self.client_info() {
            Some(p) => capabilities_for(p.client, p.version),
            None => BackendCapabilities::default(),
        }
    }
    fn client(&self) -> CliClient { self.client_info().map_or(CliClient::Unknown, |p| p.client) }
    fn version(&self) -> Option<CliVersion> { self.client_info().and_then(|p| p.version) }
    /// Fresh `<binary> --version` (probe_cli_binary, cli.rs:185-196); never touches the cache.
    fn probe(&self) -> Option<CliProbe>;
    /// `<binary> <command…> <args…> [--no-daemon] --json` under the project lock (execute_bd, cli.rs:523-592).
    fn run_json(&self, project: &ProjectRef, command: &str, args: &[String]) -> Result<String, BeadsError>;
    /// `<binary> <args…>` with PATH/BEADS_PATH, no `--json`, no lock.
    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError>;
}

/// One configured binary. Owned by `BdCli`/`BrCli`; the app holds one at a time.
#[derive(Debug)]
pub struct CliRunner {
    binary: String,
    locks: Arc<ProjectLocks>,
    probe: Mutex<Option<CliProbe>>,
}

impl CliRunner {
    pub fn new(binary: impl Into<String>, locks: Arc<ProjectLocks>) -> Self { /* probe: None */ }
    pub fn with_probe(binary: impl Into<String>, locks: Arc<ProjectLocks>, probe: CliProbe) -> Self { /* probe: Some(probe) */ }   // production: seeded rebuild (b-7)
    fn cached(&self) -> std::sync::MutexGuard<'_, Option<CliProbe>> {
        self.probe.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl CliInvoker for CliRunner {
    fn binary(&self) -> String { self.binary.clone() }
    fn probe(&self) -> Option<CliProbe> { crate::probe::probe_cli_binary(&self.binary) }
    fn client_info(&self) -> Option<CliProbe> {
        let mut cached = self.cached();
        if let Some(p) = cached.as_ref() { return Some(p.clone()); }
        let output = crate::run::probe_version_output(&self.binary).ok()?;      // spawn failure → None
        if !output.success { log_warn!("[cli_detect] Failed to get version from {}", self.binary); return None; }
        let probe = btit_beads::detect::parse_cli_probe(&output.stdout);
        match probe.version {
            Some(v) => {                                                          // cache parsed successes only (cli.rs:351-360); b-11 (B13) caches failures too
                log_info!("[cli_detect] Detected {} client v{}", btit_beads::detect::cli_client_name(probe.client), v);
                *cached = Some(probe.clone());
                Some(probe)
            }
            None => { log_warn!("[cli_detect] Could not parse version from: {}", probe.raw); None }
        }
    }
    fn run_json(&self, project: &ProjectRef, command: &str, args: &[String]) -> Result<String, BeadsError> {
        let info = self.client_info();                                        // one cache read (or one probe) per invocation
        let (client, argv) = crate::run::json_invocation(info.as_ref(), command, args);
        let wd = crate::run::resolve_working_dir(project, client)?;
        crate::run::spawn_json(&self.binary, &self.locks, &wd, &argv)         // spawns exactly `argv`; the log/lock/SchemaMigration body of execute_bd
    }
    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> {
        let client = self.client_info().map_or(CliClient::Unknown, |p| p.client);
        let wd = crate::run::resolve_working_dir(project, client)?;
        crate::run::run_raw(&self.binary, &wd, args)
    }
}
```

```rust
// crates/btit-cli/src/run.rs — the single argv assembler (execute_bd, cli.rs:534-541) and the probe → invocation step
pub fn json_argv(command: &str, args: &[String], no_daemon: bool) -> Vec<String> {
    let mut argv: Vec<String> = command.split_whitespace().map(str::to_owned).collect();
    argv.extend(args.iter().cloned());
    if no_daemon { argv.push("--no-daemon".to_owned()); }
    argv.push("--json".to_owned());
    argv
}
/// Client (for resolve_working_dir) and argv for one JSON invocation, from the cached probe. Pure; shared by CliRunner and RecordingInvoker.
pub fn json_invocation(info: Option<&CliProbe>, command: &str, args: &[String]) -> (CliClient, Vec<String>) {
    let client = info.map_or(CliClient::Unknown, |p| p.client);
    let no_daemon = info.map_or(false, |p| capabilities_for(p.client, p.version).supports_daemon_flag);
    (client, json_argv(command, args, no_daemon))
}
/// execute_bd's body after argv assembly (cli.rs:543-591): log line, project lock, spawn of exactly `argv`, SchemaMigration/CommandFailed mapping, verbose preview.
pub fn spawn_json(binary: &str, locks: &ProjectLocks, working_dir: &str, argv: &[String]) -> Result<String, BeadsError> { /* … */ }
pub fn run_json(binary: &str, no_daemon: bool, locks: &ProjectLocks, working_dir: &str, command: &str, args: &[String]) -> Result<String, BeadsError> {
    spawn_json(binary, locks, working_dir, &json_argv(command, args, no_daemon))   // kept for the transitional app wrapper `execute_bd`
}
```

```rust
// crates/btit-cli/src/testing.rs  (cfg(feature = "test-support"))
/// Scripted `CliInvoker` for unit tests in btit-cli, btit-bd, btit-br and the app: no process is spawned.
#[derive(Debug)]
pub struct RecordingInvoker { /* binary: String, probe: Option<CliProbe>, json_replies: Mutex<VecDeque<Result<String, BeadsError>>>, raw_replies: Mutex<VecDeque<Result<CliOutput, BeadsError>>>, calls: Mutex<Vec<Vec<String>>> */ }
impl RecordingInvoker {
    pub fn new(probe: Option<CliProbe>) -> Self;                          // None = the no-probe column
    pub fn reply_json(self, reply: Result<String, BeadsError>) -> Self;   // builder; replies are consumed in order
    pub fn reply_raw(self, reply: Result<CliOutput, BeadsError>) -> Self;
    pub fn calls(&self) -> Vec<Vec<String>>;                              // full argv per call, e.g. ["list", "--limit=0", "--json"]
    pub fn probe_calls(&self) -> usize;                                   // number of `probe()` calls (b-7 asserts startup adds none)
}
impl CliInvoker for RecordingInvoker {
    fn binary(&self) -> String { self.binary.clone() }                    // "bd" unless set with `.binary(..)`
    fn client_info(&self) -> Option<CliProbe> { self.probe.clone() }
    fn probe(&self) -> Option<CliProbe> { self.probe_calls.fetch_add(1, Ordering::SeqCst); self.probe.clone() }   // probe_calls: AtomicUsize
    fn run_json(&self, _p: &ProjectRef, command: &str, args: &[String]) -> Result<String, BeadsError> {
        let (_client, argv) = crate::run::json_invocation(self.probe.as_ref(), command, args);   // the same step CliRunner spawns
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).push(argv);
        self.json_replies.lock().unwrap_or_else(PoisonError::into_inner).pop_front().unwrap_or_else(|| Ok(String::new()))
    }
    fn run_raw(&self, _p: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> { /* records args verbatim, pops raw reply */ }
}
```

```rust
// crates/btit-cli/src/ops.rs (signatures; bodies are the a18c724 command bodies)
pub fn list(inv: &dyn CliInvoker, project: &ProjectRef, query: &ListQuery) -> Result<Vec<BdRawIssue>, BeadsError>;
pub fn ready(inv: &dyn CliInvoker, project: &ProjectRef) -> Result<Vec<BdRawIssue>, BeadsError>;
pub fn status(inv: &dyn CliInvoker, project: &ProjectRef) -> Result<serde_json::Value, BeadsError>;
pub fn show(inv: &dyn CliInvoker, project: &ProjectRef, id: &str) -> Result<Option<BdRawIssue>, BeadsError>;
pub fn create(inv: &dyn CliInvoker, project: &ProjectRef, payload: &CreatePayload) -> Result<BdRawIssue, BeadsError>;
pub fn update(inv: &dyn CliInvoker, project: &ProjectRef, id: &str, updates: &UpdatePayload) -> Result<Option<BdRawIssue>, BeadsError>;
pub fn close(inv: &dyn CliInvoker, project: &ProjectRef, id: &str, suggest_next: bool) -> Result<serde_json::Value, BeadsError>;
pub fn search(inv: &dyn CliInvoker, project: &ProjectRef, query: &str) -> Result<Vec<BdRawIssue>, BeadsError>;
pub fn label_add(inv: &dyn CliInvoker, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError>;
pub fn label_remove(inv: &dyn CliInvoker, project: &ProjectRef, id: &str, label: &str) -> Result<(), BeadsError>;
pub fn delete(inv: &dyn CliInvoker, project: &ProjectRef, id: &str, hard: bool) -> Result<(), BeadsError>;
pub fn comment_add(inv: &dyn CliInvoker, project: &ProjectRef, id: &str, content: &str) -> Result<(), BeadsError>;
pub fn dep_add(inv: &dyn CliInvoker, project: &ProjectRef, issue_id: &str, depends_on_id: &str, relation_type: Option<&str>) -> Result<(), BeadsError>;
pub fn dep_remove(inv: &dyn CliInvoker, project: &ProjectRef, issue_id: &str, depends_on_id: &str) -> Result<(), BeadsError>;
pub fn relation_types(client: CliClient) -> Vec<RelationType>;   // Br → common 7; Bd/Unknown → common + tracks, until, validates
pub fn sync(inv: &dyn CliInvoker, project: &ProjectRef, no_daemon: bool) -> Result<(), BeadsError>;
```

```toml
# crates/btit-bd/Cargo.toml (skeleton written by b-4; b-5 changes nothing in it) — btit-br identical with its own name/description
[package]
name = "btit-bd"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "bd (steveyegge/beads) backend: BeadsBackend, CliBackend, DoltOperations."
publish = false

[dependencies]
btit-types = { path = "../btit-types" }
btit-beads = { path = "../btit-beads" }
btit-cli = { path = "../btit-cli" }
serde_json.workspace = true
log.workspace = true

[features]
test-support = ["btit-cli/test-support"]   # enables BdCli::with_invoker for this crate's and the app's tests

[dev-dependencies]
btit-cli = { path = "../btit-cli", features = ["test-support"] }

[lints]
workspace = true
```

```rust
// crates/btit-app/src/cli.rs (transitional; AppInvoker and execute_bd are deleted in b-7, the file in b-8)
pub(crate) static PROJECT_LOCKS: LazyLock<Arc<ProjectLocks>> = LazyLock::new(|| Arc::new(ProjectLocks::default()));

pub(crate) struct AppInvoker;
impl CliInvoker for AppInvoker {
    fn binary(&self) -> String { config::get_cli_binary() }
    fn probe(&self) -> Option<CliProbe> { btit_cli::probe::probe_cli_binary(&config::get_cli_binary()) }
    fn client_info(&self) -> Option<CliProbe> { get_cli_client_info().map(|(c, a, b, d)| CliProbe { client: c, version: Some((a, b, d).into()), raw: String::new() }) }
    fn run_json(&self, project: &ProjectRef, command: &str, args: &[String]) -> Result<String, BeadsError> { /* resolve_working_dir + run_json with supports_daemon_flag() and &PROJECT_LOCKS */ }
    fn run_raw(&self, project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> { /* resolve_working_dir + run_raw */ }
}
```

## This Sprint Does Not Close

- `impl BeadsBackend`/`CliBackend` (b-5, b-6); the app's backend slot and deletion of the statics (b-7).
- `migration.rs` Dolt operations and raw invocations other than `sync` (b-8 via `DoltOperations`/`run_raw`).
- Caching of failed probes (B13, b-11).
- Subprocess timeouts. `run_json`, `run_raw` and `probe_version_output` keep today's `Command::output()` with no timeout (`cli.rs:190,338,560`); this sprint is a behaviour-preserving move of `cli.rs:533-591`, and the `migration.rs` raw calls stay as they are until b-8 moves them onto `run_raw`, also without a timeout. Later fix location: `btit_cli::run`; tracked in issue #55 (plan QA RSH-001). Do not add timeout logic in this sprint.
- `ProjectLocks` eviction: `locks.rs` keeps today's unbounded per-working-dir map (`cli.rs:14-15`); no entry is ever removed, as today. Tracked in issue #55 (plan QA RSH-005); no design change in phase-b.
- Size limits on frontend-supplied strings that become argv elements (`create`, `update`, `comment_add`, `label_add` in the argv table): none today, none added; issue #55 (related: #49 `log_frontend` size) (RSH-003).

## Acceptance Criteria

1. `cargo tree -e normal -p btit-cli --depth 1` lists exactly `btit-beads`, `btit-types`, `log`, `serde_json`; `! grep -rn 'tauri\|sc_observability' crates/btit-cli/src`; `! grep -rnE '^\s*(pub(\(crate\))? )?static ' crates/btit-cli/src` (no process-global client state: `CliRunner` owns binary and probe cache, `ProjectLocks` is passed in).
2. `cargo metadata --format-version 1 --no-deps` lists `btit-bd` and `btit-br` as members; `cargo tree -e normal -p btit-bd --depth 1` and `-p btit-br --depth 1` each list exactly `btit-beads`, `btit-cli`, `btit-types`, `log`, `serde_json`; `wc -l crates/btit-bd/src/lib.rs crates/btit-br/src/lib.rs` shows doc-only files (≤ 5 lines each).
3. `crates/btit-app/src/cli.rs` contains none of the moved functions from Exact Targets; `execute_bd` is a ≤ 10-line wrapper over `btit_cli::run_json`; no `#[tauri::command]` body in `issue_commands.rs` builds CLI args itself (`! grep -nE '"--(status|type|priority|assignee|all|limit|force|hard|suggest-next|title|description|set-labels|external-ref|estimate|design|acceptance|notes|metadata|spec-id|parent)' crates/btit-app/src/issue_commands.rs`); `! grep -rnE 'crate::cli::(new_command|get_extended_path|probe_cli_binary|extended_path_entries|default_cli_binary)|use crate::cli::\{[^}]*(new_command|get_extended_path|default_cli_binary)' crates/btit-app/src` (the `cmd`/`gh` spawns, the startup lines and the serde default use `btit_cli` paths); `grep -c 'fn default_cli_binary() -> String' crates/btit-app/src/config.rs` is `1`.
4. `git diff -M origin/integrate/phase-b...HEAD` shows the op bodies as moves; deltas inside them are limited to `execute_bd(..)?` → `inv.run_json(..)?`, `Err(String)` → `BeadsError` constructors from the b-3 error table, `context` labels, and the accessor adaptations of Deliverable 7 (`supports_*()`/`uses_*()` wrappers → `inv.capabilities().<field>`, `get_cli_client_info()` → `inv.client()`); listed in the PR description.
5. The `RecordingInvoker` tests from Deliverable 11 pass and cover every `ops` function; every cell of the Required Work argv table is an assertion in `crates/btit-cli/tests/ops_argv.rs`; `cargo test -p btit-cli --features test-support` and `cargo test -p btit-cli` both pass. The plan's command-signature gate (1) diffs empty and the frontend invoke-subset gate (2) prints nothing.
6. `cargo test --workspace` passes; the test-preservation gate prints nothing.
7. `pnpm tauri:dev` manual check: list, show, create, update, close, search, label add/remove, delete, dependency add/remove, sync and the debug panel behave as before; the `[bd] <binary> <args> | cwd: <dir>` log lines are unchanged in shape. Evidence (three log lines) in the PR description.
8. `cargo clippy -p btit-cli -p btit-bd -p btit-br --all-targets --all-features -- -D warnings`, `cargo rustdoc -p btit-cli -- -D missing-docs` pass; `! grep -rnE 'allow\(clippy::(unwrap_used|expect_used|panic|unreachable|todo|unimplemented|indexing_slicing)' crates/btit-cli/src`.
9. `git diff --exit-code feature/sprint-b-3-btit-beads...HEAD -- crates/btit-beads crates/btit-types` is empty.
10. CI green; every command in Required Validation passes.

## Required Validation

- `cargo fmt --check -p btit-types -p btit-beads -p btit-cli -p btit-bd -p btit-br`
- `cargo clippy -p btit-cli -p btit-bd -p btit-br --all-targets --all-features -- -D warnings`
- `cargo rustdoc -p btit-cli -- -D missing-docs`
- `cargo test --workspace`
- `cargo test -p btit-cli --features test-support`
- `cargo check --workspace --all-targets`
- `for c in btit-cli btit-bd btit-br; do cargo tree -e normal -p $c --depth 1 --prefix none --format '{p}' | sed -E 's/ v.*//' | sort | diff - <(printf "btit-beads\n$c\nbtit-types\nlog\nserde_json\n" | sort); done` (for `btit-bd`/`btit-br` the expected list also contains `btit-cli`)
- `git diff --exit-code feature/sprint-b-3-btit-beads...HEAD -- crates/btit-beads crates/btit-types`
- command-signature gate (1) and frontend invoke-subset gate (2) from `plan-phase-b.md` "Command contract gates"
- `python3 scripts/check_version_sync.py`
- `PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets`
- `git diff --check`
- test-preservation gate
- `pnpm tauri:dev` (manual, Acceptance Criterion 7)

## Implementation Notes

Base: `feature/sprint-b-3-btit-beads@a226ab7`. Gate inputs from `sprint-b-1.md`: `IMPLEMENTATION_BASELINE=94e44d3`,
`BASELINE_TEST_COUNT=155`, baseline worktree `/tmp/btit-baseline-94e44d3`. `origin/integrate/phase-b` is still at `eca588b`
(pre-b-2), so the layer diffs (AC4, AC9) are taken against `origin/feature/sprint-b-3-btit-beads`.

### Cite re-verification

The Exact Targets cite `a18c724`. After b-2 and b-3, `crates/btit-app/src/cli.rs` shifted, so the moved items were found by name
(at the b-3 head: `BD_PROJECT_LOCKS` 24-25, `get_extended_path` 32-69, `new_command` 73-82, `probe_cli_binary` 89-100,
`extended_path_entries` 104-111, `default_cli_binary` 113-139, `execute_bd` 279-348; the seven moved tests 402-503). `lib.rs:41,53,64`
read `lib.rs:38,50,61` at the b-3 head (b-3 added the `extern crate btit_beads` lines). The other cites were unchanged:
`issue_commands.rs` (18-72 … 556-578), `polling.rs:43-60`, `attachments.rs:50,189`, `migration.rs:4,222-250,275-300`,
`config.rs:1,12`, `updates.rs:1,75`.

### What moved where

- `path.rs`: `get_extended_path`, `extended_path_entries`, and the four `extended_path_*` tests. `command.rs`: `new_command`.
  `probe.rs`: `probe_cli_binary` (now through `run::probe_version_output`), `default_cli_binary`, and the three `probe_returns_none_*` tests.
- `locks.rs`: `ProjectLocks` (`new`, `Default`, `guard`). `run.rs`: `resolve_working_dir`, `json_argv`, `json_invocation`, `spawn_json`
  (the `execute_bd` body after argv assembly), `run_json`, `run_raw`, `probe_version_output`. `runner.rs`: `CliInvoker`, `CliRunner`.
- `ops.rs`: `list`, `ready`, `status`, `show`, `create`, `update`, `close`, `search`, `label_add`, `label_remove`, `delete`,
  `comment_add`, `dep_add`, `dep_remove`, `relation_types`, `sync`. `testing.rs` (feature `test-support`): `RecordingInvoker`.
- App: `cli.rs` keeps the statics, `get_cli_client_info`, the wrappers, `project_uses_dolt(_for)`, `reset_bd_version_cache` and
  `check_bd_compatibility`; it adds `PROJECT_LOCKS`, `AppInvoker` and the `execute_bd` wrapper, and drops the re-exports only the
  moved functions used (`parse_cli_probe`, `select_default_binary`, `CLI_CANDIDATES`, `CLI_FALLBACK`).
- Tests: `app_lib` 77 → 70 (7 moved). `btit-cli`: 19 unit tests, `tests/api_freeze.rs` 5, `tests/ops_argv.rs` 20 (every cell of the
  argv table, plus the three literal cells), `tests/ops_behaviour.rs` 19, `tests/resolve_working_dir.rs` 1.

### Gates run

- Required Validation, all pass: `cargo fmt --check -p btit-types -p btit-beads -p btit-cli -p btit-bd -p btit-br`;
  `cargo clippy -p btit-cli -p btit-bd -p btit-br --all-targets --all-features -- -D warnings` (also without `--all-features`, and for
  `x86_64-pc-windows-msvc` through `cargo xwin clippy`); `cargo rustdoc -p btit-cli -- -D missing-docs`; `cargo test --workspace`;
  `cargo test -p btit-cli` and `cargo test -p btit-cli --features test-support`; `cargo check --workspace --all-targets`; the three
  `cargo tree` diffs (empty); `git diff --exit-code origin/feature/sprint-b-3-btit-beads...HEAD -- crates/btit-beads crates/btit-types`
  (empty); `python3 scripts/check_version_sync.py` (OK line now lists `btit-cli, btit-bd, btit-br`, no script change);
  `cargo xwin check --workspace --target x86_64-pc-windows-msvc --all-targets` (only the three pre-existing Windows-only warnings
  b-3 recorded: `updates.rs:5`, `attachments.rs:2`, `updates.rs:444`); `git diff --check`; `pnpm test` (366 passed); `npx vue-tsc --noEmit`.
- Test preservation: baseline list 155 lines, after list 340, `comm -23` prints nothing.
- Command contract gates: signature diff empty (65 headers); frontend invoke subset prints nothing.
- AC1/AC3/AC8 greps print nothing; `grep -c 'fn default_cli_binary() -> String' crates/btit-app/src/config.rs` is 1;
  `cargo metadata` lists `btit-bd` and `btit-br`; both skeleton `lib.rs` files are 5 lines;
  `cargo tree -e normal,features -p beads-issue-tracker | grep -c test-support` is 0. The static grep was also run with perl, because BSD
  grep does not accept the `\(crate\)` group (CI runs the GNU form).
- AC4: each moved op body was compared token by token with its b-3-head original. The deltas are `execute_bd(.., cwd)?` →
  `inv.run_json(project, ..)?`; `format!(..)` errors → `BeadsError::ParseFailed { target, id, source }` (b-3 table);
  `parse_issues_tolerant` context labels (`list`, `list_open`, `list_closed`, `ready`) and the dropped IPC-edge `.map_err(|e| e.to_string())`;
  `supports_list_all_flag()` → `inv.capabilities().supports_list_all_flag`, `supports_delete_hard_flag()` → `hard`, the `get_cli_client_info()`
  Br match → `suggest_next`; `options.query.<f>` → `query.<f>`; `&str` parameters (`id.clone()` → `id.to_string()`,
  `std::slice::from_ref(&id)` → `&[id.to_string()]`); and `transform_issue`/`json!({"success": true})` staying in the commands.
  rustfmt reformatted the moved bodies (Required Validation runs `fmt --check -p btit-cli`).

### Deviations (minimal, justified)

1. **`RecordingInvoker::with_binary(..)`, not `.binary(..)`.** An inherent `binary(self, ..)` builder shadows `CliInvoker::binary(&self)`
   in method resolution, so `rec.binary()` on a concrete `RecordingInvoker` would not compile. Only the builder name changed. Default `"bd"`.
2. **`AppInvoker` passes `CliClient::Unknown` to `resolve_working_dir`.** The client only labels the `Unsupported` error of a non-local
   `ProjectRef`, and none exists. Reading `get_cli_client_info()` for it would add a `--version` spawn per call whenever the probe fails
   (only parsed probes are cached). `run_json` keeps the one `supports_daemon_flag()` read `execute_bd` made. `execute_bd` is a 3-line
   wrapper, `AppInvoker.run_json(&ProjectRef::local(cwd), ..).map_err(|e| e.to_string())`, which is `run::run_json(&get_cli_binary(),
   supports_daemon_flag(), &PROJECT_LOCKS, &wd, ..)`. It carries `#[expect(dead_code, ..)]` because every former caller now goes through `ops`.
3. **Log lines from `ops::list` on more paths (log text only).** The moved `bd_list` range (18-72) contains the `[bd_list] --all requested …`,
   `[bd_list] Found {} issues (fallback)` and `[bd_list] Found {} issues` lines. `bd_count`, `bd_poll_data` and `purge_orphan_attachments`
   now call `ops::list`, so they also emit those lines. The log target of moved lines is `btit_cli::ops`/`btit_cli::run` (plan "Behaviour preserved" (2)).
4. **Legacy-only argv order in `bd_count` and `bd_poll_data`.** On bd < 0.55 (and unknown/no probe), their closed-issues call was
   `list --status=closed --limit=0`. Through `ops::list` it is `list --limit=0 --status=closed`, the argv table's two-call form. The set
   of issues is the same. The open list is now parsed before the closed call runs, so invalid open-list JSON fails one call earlier. The
   `--all` path (bd ≥ 0.55, br) is byte-identical.
5. **`ops::sync` `CommandFailed.status_display`.** `CliOutput` carries only the exit code, so the text is rebuilt as
   `std::process::ExitStatus`'s `Display` (`exit status: N`, or `exit code: N` on Windows; `terminated by signal` for `None`). A unit test
   checks it against std for exit codes. The app's sync mapping reads only `stderr`/`source`, so no app string depends on it. An `Err(e)` arm
   covers the other `BeadsError` variants, which a local project cannot produce: `sync_bd_database` logs `[sync] Failed to run {} sync: {e}`
   and `bd_sync` returns `e.to_string()`.
6. **Lint handling.** `#[expect(.., reason)]`, never `#[allow]`: `get_extended_path` `uninlined_format_args` (non-Windows only; the Windows
   raw-string branch does not trip it), `new_command` `unused_mut` (non-Windows) and `unreadable_literal` (Windows, `0x08000000`), the moved
   Windows test `extended_path_entries_precede_ambient_path` `map_unwrap_or`, and the `ops` functions `list`/`update`/`delete`
   (`uninlined_format_args`; `update` also `too_many_lines`). Otherwise-verbatim moved code: `resolve_working_dir` uses
   `map_or_else` for the `current_dir()` fallback (`clippy::map_unwrap_or`); `relation_types` uses `if client == Br` (clippy rejects the adapted
   `match` as an equality check); `CliRunner::client_info` uses `if let` where the sample has `match` (`single_match_else`); local `argv` bindings are
   `full_args` (`similar_names` against `args`, the name `execute_bd` used).
7. **Manifests.** `btit-cli` takes `btit-types`/`btit-beads` as `.workspace = true`, and the root `[workspace.dependencies]` gains
   `btit-cli = { path = "crates/btit-cli" }` for the app (the b-3 convention, b-3 Deviation 3). The skeletons use the path dependencies
   exactly as in the code sample. `btit-cli` declares `[[test]] required-features = ["test-support"]` for `ops_argv` and `ops_behaviour`,
   so `cargo test -p btit-cli` passes without the feature. `cargo test --workspace` still runs them, because the skeletons' dev-dependency
   unifies the feature in.
8. **Small API additions.** `ProjectLocks::new()` besides `Default` (C-CTOR). `probe_version_output`'s `Spawn.operation` is
   `Some("--version")`. `RecordingInvoker` replies `Ok("")` for `run_json` and a successful empty `CliOutput` for `run_raw` when its
   queue is empty (the sample's `run_json` default, applied to both).
9. **Test layout.** The `resolve_working_dir` precedence test mutates `BEADS_PATH`, so it is the only test in its own binary
   (the unit tests read `PATH`/`HOME` and spawn processes concurrently). `show`/`update`/`close`/`delete`/`relation_types` behaviour
   tests are in `tests/ops_behaviour.rs`, and the argv table is in `tests/ops_argv.rs`.
10. **CI.** Besides the `rust-quality` fmt/clippy/rustdoc/`cargo tree` rows for the three crates, the Dependency graph step asserts that
    `test-support` is absent from the app's normal feature tree. A new Linux step runs the AC1/AC8 source greps for `btit-cli`, `btit-bd`
    and `btit-br`, as b-3 did for `btit-beads`.

### Behaviour deltas (PR description)

- `ProjectLocks` recovers a poisoned map or project mutex with `PoisonError::into_inner` where `execute_bd` called `.unwrap()` (Deliverable 3).
- `purge_orphan_attachments` on bd < 0.55 takes the two-call fallback instead of an unconditional `--all` (Deliverable 10).
- Deviations 3 and 4 above (log lines; legacy argv order in `bd_count`/`bd_poll_data`).

### Shared files pre-registered for group A (b-5, b-6, b-9)

Root `Cargo.toml` (members `crates/btit-cli`, `crates/btit-bd`, `crates/btit-br`; `[workspace.dependencies] btit-cli`), root `Cargo.lock`
(`btit-cli`, `btit-bd`, `btit-br` with their dependency edges), `crates/btit-bd/{Cargo.toml,clippy.toml,src/lib.rs}`,
`crates/btit-br/{Cargo.toml,clippy.toml,src/lib.rs}` (final dependency sets and the `test-support` feature), and the
`.github/workflows/ci.yml` `rust-quality` rows (fmt, clippy, rustdoc, `cargo tree`, source gates) for all three crates.

### Open items before `status: complete`

- AC7: the `pnpm tauri:dev` manual check (list, show, create, update, close, search, label add/remove, delete, dependency add/remove, sync,
  debug panel; three `[bd] <binary> <args> | cwd: <dir>` log lines as evidence in the PR) is for the team-lead; the developer agent did not launch the app.
- AC10: CI green on the PR.
