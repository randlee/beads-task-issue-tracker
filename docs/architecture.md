# Architecture

> This file is a temporary home for architecture decision records (ADRs). They will move to a dedicated location later. The rest of the architecture documentation will be written after the phase-a/phase-b restructuring (sc-observability adoption, crate split).

## Architecture Decision Records

Status values:
- **Accepted:** the maintainer decided it.
- **Proposed:** written into the phase-a plan and waiting for maintainer confirmation.
- **Superseded:** replaced by a later ADR, which is named.

| ADR | Title | Status | Date |
|---|---|---|---|
| [ADR-001](#adr-001-errors-are-discriminated-unions-and-production-code-does-not-panic) | Errors are discriminated unions; production code does not panic | Accepted | 2026-09-13 |
| [ADR-002](#adr-002-structured-logging-through-sc-observability-via-a-log-bridge-and-tracing-compatible-macros) | Structured logging through sc-observability via a `log` bridge and tracing-compatible macros | Accepted | 2026-09-13 |
| [ADR-003](#adr-003-crate-layout-and-version-single-source-of-truth) | Crate layout and version single source of truth | Accepted | 2026-09-13 |
| [ADR-004](#adr-004-rust-toolchain-pin-and-msrv) | Rust toolchain pin and MSRV | Accepted | 2026-09-13 |
| [ADR-005](#adr-005-stacked-sprint-workflow-per-layer-worktrees-with-gh-stack-link) | Stacked sprint workflow: per-layer worktrees with `gh stack link` | Accepted | 2026-09-13 |
| [ADR-006](#adr-006-tracing-compatibility-policy) | tracing compatibility policy | Proposed | 2026-09-13 |
| [ADR-007](#adr-007-sc-observability-log-runtime-lifecycle) | sc-observability-log runtime lifecycle (global slot, panic containment, bounded shutdown) | Proposed | 2026-09-13 |

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
  - The phase-a crates (`sc-observability-log`, `sc-observability-log-macros`) follow this from the first sprint. btit adoption code in phase-a follows it for the functions it touches. The rest of `src-tauri` is migrated later.
  - This conflicts with sc-observability's opaque `error_wrapper!` structs. That is tracked in [randlee/sc-observability#92](https://github.com/randlee/sc-observability/issues/92), and it is a review item in phase-a sprint a-5.
- **Links:** `docs/plans/phase-a/plan-phase-a.md` (Engineering standards), `docs/plans/phase-a/sprint-a-1.md`.

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
  - After handoff, `../sc-observability` becomes the source of truth. Phase-b switches btit to the published crates.
  - The two crates share a hidden `__private` contract, so the runtime crate pins the macros crate with an exact `=` version, following the `serde` → `serde_core` precedent, and both release in lockstep.
- **Links:** `docs/plans/phase-a/`, PR #37.

### ADR-003: Crate layout and version single source of truth

- **Status:** Accepted, by maintainer direction on 2026-09-13.
- **Context:** Version numbers were duplicated across `package.json`, `src-tauri/Cargo.toml` and `tauri.conf.json`, and the release-time sync script was missing. The maintainer plans to split the Rust code into multiple crates, following `../atm-core` and `../sc-compose`.
- **Decision:**
  - **Version source.** `[workspace.package].version` in `src-tauri/Cargo.toml` is the single source for the app version. `tauri.conf.json` omits `version`, and Tauri falls back to Cargo. `package.json` mirrors the version.
  - **Enforcement.** `scripts/check_version_sync.py` checks all of these, and `--set` bumps them. CI runs it in the `version sync` job.
  - **Crate split.** Moving btit's backend into root `crates/` and splitting it into multiple crates is phase-b. Phase-a's `crates/` workspace is transitional and holds only the sc-observability-log crates.
- **Consequences:** The crates split out later use `version.workspace = true`.
- **Links:** PR #36, `docs/plans/phase-a/plan-phase-a.md` (issue inventory `b-crate-split`).

### ADR-004: Rust toolchain pin and MSRV

- **Status:** Accepted, by maintainer direction on 2026-09-13.
- **Context:** `sc-observability` 1.2.0 requires Rust ≥ 1.94.1, and btit's CI used floating `stable`.
- **Decision:**
  - **Toolchain pin.** `rust-toolchain.toml` pins stable 1.98.1, the latest stable at the time. CI and the release workflows pin the same version, and `check_version_sync.py` enforces that every workflow `toolchain:` matches.
  - **Phase-a crates MSRV.** The phase-a crates declare `rust-version = "1.94.1"`, matching sc-observability.
  - **MSRV checking in CI.** CI installs the MSRV toolchain with a `rustup` run step, not a workflow `toolchain:` pin, so the version-sync gate still passes.
- **Links:** PR #36, `docs/plans/phase-a/sprint-a-1.md`.

### ADR-005: Stacked sprint workflow: per-layer worktrees with `gh stack link`

- **Status:** Accepted, by maintainer direction on 2026-09-13. The constraint was verified locally.
- **Context:** The maintainer requires every sequence of sprints to use gh-stack with worktrees, and sprints to run in parallel where possible. A local test on gh-stack v0.1.0 (2026-09-13) found:
  - `gh stack rebase` cannot rebase a branch that is checked out in another worktree.
  - Local stack tracking (`init`, `add`, `rebase`, `sync`, `view`) is not visible from linked worktrees.
  - A branch that belongs to two stacks breaks non-interactive commands.
  - GitHub stacks are strictly linear.
- **Decision:**
  - **Worktrees.** Each layer lives in its own `/sc-git-worktree` worktree.
  - **Stacks on GitHub.** The stack is managed with `gh stack link` and `gh stack merge`. There is no local stack tracking.
  - **Rebasing.** Layers are rebased with `git rebase <parent>` inside each worktree, bottom to top, followed by `push --force-with-lease`.
  - **Parallel lanes.** A lane that has to branch from a layer already in a stack becomes a separate PR on `develop` after that layer merges. Phase-a's a-4 is an example.
  - **CI.** CI triggers on `feature/**` PR bases, so stacked PRs are validated.
- **Consequences:**
  - `gh stack init`, `add`, `rebase`, `sync` and `submit` are not used.
  - Evidence recorded across rebases (for example review fixes) uses annotated tags and tree hashes, not bare commit SHAs.
- **Links:** `docs/plans/phase-a/plan-phase-a.md` (gh-stack and worktree workflow).

### ADR-006: tracing compatibility policy

- **Status:** Proposed. It was written into the phase-a plan in QA fix round 2, derived from ADR-002's "import rename" goal, and is waiting for maintainer confirmation.
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

### ADR-007: sc-observability-log runtime lifecycle

- **Status:** Proposed. It was produced by phase-a QA fix rounds 1 and 2 and is waiting for maintainer confirmation.
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
  - The panic containment deliberately goes against the `rust-development` M-PANIC-IS-STOP guideline in order to meet ADR-001 while depending on third-party code that can panic. This is flagged for the a-5 review.
- **Links:** `docs/plans/phase-a/sprint-a-1.md`, `docs/plans/phase-a/sprint-a-4.md`.
