# Architecture

> This file is a temporary home for architecture decision records (ADRs). They will move to a dedicated location later. The rest of the architecture documentation will be written after the phase-a/phase-b restructuring (sc-observability adoption, crate split).

## Architecture Decision Records

Status values:
- **Accepted:** the maintainer decided it.
- **Proposed:** written into the phase-a plan and waiting for maintainer confirmation.
- **Superseded:** replaced by a later ADR, which is named.

| ADR | Title | Status | Date |
|---|---|---|---|
| [ADR-009](#adr-009-stacked-sprint-groups-fork-and-re-merge-per-branch-qa-1-fix-layers) | Stacked sprint groups: fork and re-merge, per-branch QA-1, fix layers | Accepted | 2026-09-13 |
| [ADR-008](#adr-008-beads-backend-contract) | Beads backend contract | Accepted | 2026-09-13 |
| [ADR-007](#adr-007-sc-observability-log-runtime-lifecycle) | sc-observability-log runtime lifecycle (global slot, panic containment, bounded shutdown) | Proposed | 2026-09-13 |
| [ADR-006](#adr-006-tracing-compatibility-policy) | tracing compatibility policy | Proposed | 2026-09-13 |
| [ADR-005](#adr-005-stacked-sprint-workflow-per-layer-worktrees-with-gh-stack-link) | Stacked sprint workflow: per-layer worktrees with `gh stack link` | Accepted; parallel-lane rule superseded by ADR-009 | 2026-09-13 |
| [ADR-004](#adr-004-rust-toolchain-pin-and-msrv) | Rust toolchain pin and MSRV | Accepted | 2026-09-13 |
| [ADR-003](#adr-003-crate-layout-and-version-single-source-of-truth) | Crate layout and version single source of truth | Accepted | 2026-09-13 |
| [ADR-002](#adr-002-structured-logging-through-sc-observability-via-a-log-bridge-and-tracing-compatible-macros) | Structured logging through sc-observability via a `log` bridge and tracing-compatible macros | Accepted | 2026-09-13 |
| [ADR-001](#adr-001-errors-are-discriminated-unions-and-production-code-does-not-panic) | Errors are discriminated unions; production code does not panic | Accepted | 2026-09-13 |

---

### ADR-009: Stacked sprint groups: fork and re-merge, per-branch QA-1, fix layers

- **Status:** Accepted, by maintainer direction on 2026-09-13.
- **Context:**
  - ADR-005 (stacked sprint workflow) records the gh-stack constraints verified in phase-a: GitHub stacks are strictly linear, so a branch has one parent and at most one stacked child. Phase-a met this by moving its parallel lane (a-4) out of the stack.
  - phase-b has two groups of sprints that can run in parallel on top of one stack layer (group A: b-5, b-6, b-9 on the b-4 head; group B: b-8, b-10 on the b-7 head). They need a rule that keeps the stack linear without serializing the group.
  - CI on stacked branches is slow, so a rebase cascade through lower layers is expensive.
- **Decision:**
  - **Four-step rule for a parallel group on layer k.**
    1. **Fork.** Every sprint in the group branches from the layer-k head, in its own worktree, and opens a draft PR with base = the layer-k branch. None is stack-linked at creation.
    2. **First closer becomes layer k+1.** The first sprint to reach closure (acceptance criteria met, QA with no Blocking finding) is linked into the stack with `gh stack link`.
    3. **Next layer from k+1.** The layer-(k+2) sprint, which `must_follow` the whole group, is created from the layer-(k+1) head, so the stack stays linear.
    4. **Late finishers merge into k+2.** Each remaining group member is merged into the layer-(k+2) branch with `git merge --no-ff` when it reaches closure, and its draft PR is closed with a comment naming the merge commit. The layer-(k+2) sprint merges every pushed member before each dev/fix round and closes only when all members are merged in.
  - **Per-branch QA.** Each parallel branch completes its own QA-1 on its draft PR before it is stacked as layer k+1 or merged into the layer-(k+2) branch. QA gating never waits on a sibling.
  - **Fix layers.** Once a sprint's layer is integrated (stack-linked, or merged into its join layer), later QA findings or review fixes for it land as a new fix layer on top of the current stack top (branch `fix/sprint-<id>-<slug>`, linked with `gh stack link`). Lower layers are not amended or force-pushed. The fix is documented as a "Fix layer" section appended to the sprint doc it fixes.
  - **Join layers are rebased only with `--rebase-merges`.** A layer that carries late-finisher merges is rebased with `git rebase --rebase-merges <parent>` so the merges survive, or not at all.
- **Consequences:**
  - Join layers (the layer-(k+2) branches) carry merge commits, so the stack is linear in branches but not in commits.
  - Group members have no fixed layer number. They are labelled "layer k+1 (a | b | c, first to close)", and downstream layer numbers are relative to the group.
  - Members of a group still need non-intersecting files (proven in the plan's ownership table), because the late merges must be conflict-free.
- **Links:** `docs/plans/phase-b/plan-phase-b.md` ("Parallel groups: fork and re-merge").

---

### ADR-008: Beads backend contract

- **Status:** Accepted, by maintainer direction on 2026-09-13.
- **Context:**
  - btit's Rust backend was one Tauri crate, and the CLI choice and its detected version were process globals: `AppConfig.cli_binary` and `static CLI_BINARY` (`config.rs:8-13`), `static BD_PROJECT_LOCKS` and `static CLI_CLIENT_INFO` (`cli.rs:14-20`), line numbers at `a18c724`. Every command read those globals, so one process could drive only one CLI for every project.
  - Maintainer requirements 2–5 for phase-b (`plan-phase-b.md` "Binding outcomes"): `btit-types` holds data types only (2); the backend traits live in `btit-beads`, not in `btit-types` (3); the trait design covers bd and br today and is shaped by requirements 3, 4 and 5, including a future SQL transport against a beads Dolt server or a DoltHub client that must fit without caller changes (5).
  - bd and br share most issue operations but differ in backend-specific ones: Dolt repair, migration, init and import exist for bd only; `close --suggest-next` exists for br only.
- **Decision:**
  - **Trait family** in `crates/btit-beads/src/backend.rs`, all object-safe and `Send + Sync`, synchronous like today's blocking `Command::output()` calls:
    - `BeadsBackend`: transport-neutral issue operations, `project_uses_dolt(&ProjectRef)`, `relation_types()`, `sync()`. No notion of a binary, a process, a version or `--json`.
    - `CliBackend: BeadsBackend`: what only a spawned CLI has: `binary()`, `probe()`, `client()`, `version()`, `capabilities()`, `run_raw()`, `release_source()`. The detected client kind, version and the five version gates live here, not on `BeadsBackend`, because a SQL transport has no equivalent.
    - `DoltOperations`: bd-only Dolt operations, transport-neutral: every method returns `btit_types::DoltOpResult { success, message, detail }`, never process output, so a SQL transport can implement it.
    - `CloseSuggestions`: br-only `close <id> --suggest-next`.
  - **Optional-accessor pattern.** `BeadsBackend` reaches everything transport- or backend-specific through default-`None` accessors of one shape: `cli() -> Option<&dyn CliBackend>`, `dolt() -> Option<&dyn DoltOperations>`, `close_suggestions() -> Option<&dyn CloseSuggestions>`. The app holds `Arc<dyn BeadsBackend>` and never downcasts; a caller that needs CLI facts calls `backend.cli()` and maps `None` to `BeadsError::Unsupported`.
  - **One error type.** `btit_beads::BeadsError` is the only error crossing crate boundaries: a `#[non_exhaustive]` enum with `code()` and `remediation()` per variant and a `Display` that reproduces today's command error strings. The Tauri commands keep `Result<_, String>` at the IPC edge.
  - **No process-global client state in library crates.** `btit-beads`, `btit-cli`, `btit-bd` and `btit-br` hold no `static` mutable state other than the two logging switches in `btit_beads::logging`. Backends are instances, so two can coexist for two projects. The app owns the single backend slot (`crates/btit-app/src/backend.rs`, b-7).
- **Alternatives considered:**
  - **Traits inside `btit-types`.** Rejected: it puts behaviour and an error type into the data-only crate (requirement 2) and makes every DTO consumer compile the contract.
  - **One fat trait whose unsupported methods return `Unsupported`.** Rejected: a non-CLI transport would have to implement every CLI-only and backend-specific method as an `Unsupported` stub, while the decision keeps `BeadsBackend` free of any notion of a binary or process.
  - **`Any` downcasting from `dyn BeadsBackend`.** Rejected: callers would have to name the concrete backend types (`BdCli`, `BrCli`) to reach CLI or Dolt operations; the accessors give the same reach through `&dyn` trait objects.
- **Consequences:**
  - A SQL or DoltHub transport implements `BeadsBackend` (and optionally `dolt()`) with no caller change.
  - Per-project backend selection becomes a change to the app's slot function (`backend::current()` to `backend::for_project(&ProjectRef)`), not to any trait or backend crate. Whether to add it is OQ-8.
  - Issue #51 (Dolt commit log, `AS OF` snapshots, `dolt_diff`) fits as a further bd-only trait behind an accessor like `dolt()`. No method for it is planned.
  - Adding a `BeadsError` variant or a trait method is a plan change: `crates/btit-beads/tests/api_freeze.rs` pins the public API.
- **Links:** `docs/plans/phase-b/sprint-b-3.md`, `crates/btit-beads/docs/backend-contract.md`, `docs/plans/phase-b/plan-phase-b.md` ("Trait design", "Trait inventory").

---

### ADR-007: sc-observability-log runtime lifecycle

- **Status:** Proposed. It was produced by phase-a QA fix rounds 1 and 2 and is waiting for maintainer confirmation. Every mechanism below is implemented in `crates/sc-observability-log` and was reviewed by the sc-observability team in sprint a-5.
- **Context:**
  - `sc_observability::Logger::flush` does a blocking send followed by an untimed `recv`, and `shutdown` ends with an unbounded `join` (`maintenance.rs:137-164`, `166-234`).
  - `try_log` can panic on a poisoned internal lock (`runtime.rs:392-398`).
  - The `log` facade logger can be installed only once per process.
  - ADR-001 forbids blocking forever or panicking.
- **Decision:**
  - **Global slot.** The installed logger lives in a global `RwLock<Option<Arc<Installed>>>` slot. The lock is held only to clone the `Arc`.
  - **Install once.** `compare_exchange` runs at `init` entry. A failed identity or logger init resets it. A foreign logger install leaves it set.
  - **Panic and reentrancy containment.** `try_log` runs under `catch_unwind(AssertUnwindSafe(..))` with a thread-local reentrancy guard. Reentrant emits are dropped and counted as `DropCause::ReentrantEmit`.
  - **Drop accounting.** Dropped events are counted per `DropCause` (7 variants).
  - **Bounded flush and shutdown.** Both run on a helper thread (`sc-observability-log-helper`) and wait with `recv_timeout`. Shutdown takes sole ownership by retrying `Arc::try_unwrap` until a deadline.
  - **`Log::flush`.** The `log::Log::flush` implementation is a no-op.
  - **Single level threshold.** The only level threshold is `LoggerConfig.level`.
  - **Process identity.** Process identity is resolved once, at `init`.
- **Consequences:**
  - `catch_unwind` has no effect under `panic = abort`. This residual is documented.
  - A helper that times out is detached.
  - The panic containment deliberately goes against the `rust-development` M-PANIC-IS-STOP guideline in order to meet ADR-001 while depending on third-party code that can panic. The a-5 review accepted it, and the hardening it asked for landed in that sprint.
- **Links:** `docs/plans/phase-a/sprint-a-1.md`, `docs/plans/phase-a/sprint-a-4.md`.

---

### ADR-006: tracing compatibility policy

- **Status:** Proposed. It was written into the phase-a plan in QA fix round 2, derived from ADR-002's "import rename" goal, and is waiting for maintainer confirmation. The policy is implemented: every supported form is covered by the compat tests in `crates/sc-observability-log/tests/compat_events.rs` and `compat_instrument.rs`, and every rejected form has a trybuild case in `crates/sc-observability-log/tests/ui/`.
- **Context:** Migrating from `tracing` should need only an import rename. Some tracing features have no equivalent in sc-observability, which has no span API and no deferred field recording.
- **Decision:**
  - **Scope.** Migration from tracing 0.1 is an import rename for every supported form. The rejected-forms table lists the tracing-valid forms that fail loudly at compile time.
  - **Supported event macro forms:**
    - brace field sets
    - non-literal `target:` / `name:`
    - `{ CONST } = v` keys
    - `event!(name:, Level)` and a const `event!` level
  - **Supported `#[instrument]` forms:**
    - `name` / `target` / `level = CONST`
    - `ret(level = ..)` and `err(level = ..)`, including the `Debug` / `Display` variants
    - dotted, const-key and raw-identifier `fields(..)`
    - `self` recorded via `Debug`
  - **Rejected, with a trybuild case naming each form:**
    - `parent:`
    - `follows_from`
    - deferred fields: value-less fields, `tracing::field::Empty`
    - the `""` key
    - reserved `sc_observability_log.*` keys
    - span macros
  - **Label sanitization.** Labels and dynamic keys are sanitized once at runtime by a single sanitizer, cached per callsite. The macro crate does no sanitization.
- **Links:** `docs/plans/phase-a/sprint-a-2.md`, `docs/plans/phase-a/sprint-a-3.md`.

---

### ADR-005: Stacked sprint workflow: per-layer worktrees with `gh stack link`

- **Status:** Accepted, by maintainer direction on 2026-09-13. The constraint was verified locally. The **Parallel lanes** decision below is superseded by [ADR-009](#adr-009-stacked-sprint-groups-fork-and-re-merge-per-branch-qa-1-fix-layers); the gh-stack constraints and the rest of the decision still hold.
- **Context:** The maintainer requires every sequence of sprints to use gh-stack with worktrees, and sprints to run in parallel where possible. A local test on gh-stack v0.1.0 (2026-09-13) found:
  - `gh stack rebase` cannot rebase a branch that is checked out in another worktree.
  - Local stack tracking (`init`, `add`, `rebase`, `sync`, `view`) is not visible from linked worktrees.
  - A branch that belongs to two stacks breaks non-interactive commands.
  - GitHub stacks are strictly linear.
- **Decision:**
  - **Worktrees.** Each layer lives in its own `/sc-git-worktree` worktree.
  - **Stacks on GitHub.** The stack is managed with `gh stack link` and `gh stack merge`. There is no local stack tracking.
  - **Rebasing.** Layers are rebased with `git rebase <parent>` inside each worktree, bottom to top, followed by `push --force-with-lease`.
  - **Parallel lanes.** ~~A lane that has to branch from a layer already in a stack becomes a separate PR on `develop` after that layer merges. Phase-a's a-4 is an example.~~ Superseded by ADR-009: a parallel group forks from one layer head, the first to close becomes the next layer, and the late finishers merge into the layer above it. Phase-b used this for groups A (b-5, b-6, b-9) and B (b-8, b-10).
  - **CI.** CI triggers on `feature/**` PR bases, so stacked PRs are validated.
- **Consequences:**
  - `gh stack init`, `add`, `rebase`, `sync` and `submit` are not used.
  - Evidence recorded across rebases (for example review fixes) uses annotated tags and tree hashes, not bare commit SHAs.
- **Links:** `docs/plans/phase-a/plan-phase-a.md` (gh-stack and worktree workflow).

---

### ADR-004: Rust toolchain pin and MSRV

- **Status:** Accepted, by maintainer direction on 2026-09-13.
- **Context:** `sc-observability` 1.2.0 requires Rust ≥ 1.94.1, and btit's CI used floating `stable`.
- **Decision:**
  - **Toolchain pin.** `rust-toolchain.toml` pins stable 1.98.1, the latest stable at the time. CI and the release workflows pin the same version, and `check_version_sync.py` enforces that every workflow `toolchain:` matches.
  - **Phase-a crates MSRV.** The phase-a crates declare `rust-version = "1.94.1"`, matching sc-observability.
  - **MSRV checking in CI.** CI installs the MSRV toolchain with a `rustup` run step, not a workflow `toolchain:` pin, so the version-sync gate still passes.
- **Links:** PR #36, `docs/plans/phase-a/sprint-a-1.md`.

---

### ADR-003: Crate layout and version single source of truth

- **Status:** Accepted, by maintainer direction on 2026-09-13.
- **Context:** Version numbers were duplicated across `package.json`, `src-tauri/Cargo.toml` and `tauri.conf.json`, and the release-time sync script was missing. The maintainer plans to split the Rust code into multiple crates, following `../atm-core` and `../sc-compose`.
- **Decision:**
  - **Version source.** `[workspace.package].version` in the workspace root `Cargo.toml` is the single source for the app version. It lived in `src-tauri/Cargo.toml` until phase-b b-1 moved the workspace to the repository root and the Tauri crate to `crates/btit-app`. `tauri.conf.json` omits `version`, and Tauri falls back to Cargo. `package.json` mirrors the version.
  - **Enforcement.** `scripts/check_version_sync.py` checks all of these, and `--set` bumps them. CI runs it in the `version sync` job.
  - **Crate split.** Moving btit's backend into root `crates/` and splitting it into multiple crates is phase-b. Phase-a's `crates/` workspace was transitional and held only the sc-observability-log crates.
- **Consequences:**
  - Phase-b completed the split: one root workspace whose members are `crates/btit-app` (Tauri), `crates/btit-types`, `crates/btit-beads`, `crates/btit-cli`, `crates/btit-bd`, `crates/btit-br` and the three in-tree `sc-observability-log*` crates. `src-tauri/` no longer exists.
  - The btit crates use `version.workspace = true`. The `sc-observability-log*` crates version independently (0.1.0, their own MSRV), and `scripts/check_version_sync.py` exempts them through `INDEPENDENT_VERSION_MEMBERS`.
- **Links:** PR #36, `docs/plans/phase-a/plan-phase-a.md` (issue inventory `b-crate-split`).

---

### ADR-002: Structured logging through sc-observability via a `log` bridge and tracing-compatible macros

- **Status:** Accepted, by maintainer direction on 2026-09-13.
- **Context:**
  - btit logs plain text through `log` 0.4 and `tauri-plugin-log`.
  - The project is joining the sc ecosystem, whose logging library is `sc-observability` 1.2.0 (structured JSONL).
  - `sc-observability` 1.2.0 has no bridge from `log` or `tracing`, no event macros, no attributes and no compile-time tooling.
  - The maintainer wants future migrations to need no more than adding a couple of crates and renaming imports and attributes.
- **Decision:**
  - **New crates.** Build two crates:
    - `sc-observability-log`: the `log::Log` bridge, runtime, and `#[doc(hidden)] __private` emit path.
    - `sc-observability-log-macros`: tracing-compatible `trace!`, `debug!`, `info!`, `warn!`, `error!` and `event!` macros, plus `#[instrument]`.
  - **Built in btit, published from sc-observability.** They are developed in btit phase-a in a transitional `crates/` workspace, to sc-observability's standards: edition 2024, MSRV 1.94.1, sc-observability's lint set. After a critical review by the sc-observability team (coordinated over ATM with `cobs`), they are copied into `../sc-observability` and published from there.
  - **btit adoption.** btit replaces `tauri-plugin-log` with the bridge without changing call sites. Logs become `<app_log_dir>/logs/beads-task-issue-tracker.log.jsonl`.
- **Consequences:**
  - The crates remain in-tree in btit for now. Phase-b kept them as workspace members at 0.1.0 (open question OQ-3) rather than depending on a published release, so the crate split did not block on publishing.
  - The copy into `../sc-observability` and the switch to the published crates are still to come; `../sc-observability` becomes the source of truth once that lands.
  - The two crates share a hidden `__private` contract, so the runtime crate pins the macros crate with an exact `=` version, following the `serde` → `serde_core` precedent, and both release in lockstep.
- **Links:** `docs/plans/phase-a/`, PR #37.

---

### ADR-001: Errors are discriminated unions and production code does not panic

- **Status:** Accepted, by maintainer direction on 2026-09-13.
- **Context:** The maintainer's position is that discriminated-union errors and panic-free code give a more maintainable codebase and higher reliability. This standard will be applied across the whole btit codebase. The existing code does not meet it yet.
- **Decision:**
  - **Errors.** Every Rust error type is an `enum` with typed, structured variants that callers can `match` on. Opaque boxed wrappers and string errors are not allowed.
  - **Diagnostic metadata.** Where metadata is needed (for example sc-observability `ErrorCode` / `Remediation`), it is attached per variant, e.g. with `code()` and `remediation()` accessors.
  - **No panics.** Production code does not panic. `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` and panicking indexing are forbidden.
  - **Poisoned locks** are handled explicitly, for example with `PoisonError::into_inner`, never with `.lock().unwrap()`.
  - **Enforcement.** New crates set clippy `unwrap_used`, `expect_used`, `panic`, `unreachable`, `todo`, `unimplemented` and `indexing_slicing` to `deny`, alongside the pedantic set. Tests are exempt through `clippy.toml` `allow-*-in-tests` or a file-level `#![allow]` in `tests/*.rs`.
  - **Allowed exceptions.** Re-raising a user function's panic, as `#[instrument]` does, is propagation and not a new panic. Proc-macro compile errors via `syn::Error` / `compile_error!` are allowed.
- **Consequences:**
  - The phase-a crates (`sc-observability-log`, `sc-observability-log-macros`) follow this from the first sprint. btit adoption code in phase-a follows it for the functions it touches.
  - The rest of the backend followed in phase-b: the new `btit-*` crates are held to the standard from creation, and b-12 put the app crate (`crates/btit-app`, formerly `src-tauri`) under the workspace lints with its panic sites rewritten, so `cargo clippy --workspace --all-targets --all-features -- -D warnings` is a CI gate for the whole workspace.
  - This conflicts with sc-observability's opaque `error_wrapper!` structs. That is tracked in [randlee/sc-observability#92](https://github.com/randlee/sc-observability/issues/92), and it is a review item in phase-a sprint a-5.
- **Links:** `docs/plans/phase-a/plan-phase-a.md` (Engineering standards), `docs/plans/phase-a/sprint-a-1.md`.
