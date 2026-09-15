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
