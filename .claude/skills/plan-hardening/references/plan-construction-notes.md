# Plan Construction Notes

Use this reference when creating missing sprint docs or normalizing an
existing phase plan into a hardened form, in the btit repo.

## Plan Shape

The planning set should have:

- one top-level project plan entry that names the phase
- one phase plan document that lists the whole remaining phase
- one sprint doc per sprint in the phase

The phase plan explains sequencing and closure intent.
The sprint docs are the authoritative execution artifacts.

## Naming Convention

Use consecutive capital letters for phases and `<Phase>.<number>` for sprint
identifiers.

Examples:

- `Phase A`
- `A.1`
- `A.2`
- `Phase B`
- `B.1`

Recommended doc naming, under btit's existing `docs/` directory (see
`docs/azure-devops-mapping.md`, `docs/attachments.md` for existing doc
conventions):

- `docs/plans/phase-A/plan-phase-A.md`
- `docs/plans/phase-A/sprint-A1.md`
- `docs/plans/phase-A/sprint-A2.md`

This is a deliberate adaptation from the atm-core source skill, which used
`docs/phase-A/...` — btit already has a flat `docs/` directory with several
unrelated docs, so phase/sprint plans are namespaced under `docs/plans/` to
avoid clutter. See also the CLAUDE.md `## Plan Mode` note: `.claude/plans/`
is for ephemeral, single-session Plan Mode output (via `ExitPlanMode`), not
for the durable, reviewed, checked-in phase/sprint docs this skill produces
— those belong in `docs/plans/` per this skill's convention.

## Branch And Worktree Model

Use this default model unless the user explicitly chooses something else
(see `SKILL.md` → `## Branch Model` for the full rationale):

- `develop` is the baseline integration branch
- `plan/<phase>` is the planning branch or worktree, created from `develop`
- `feature/<phase>-<sprint>-...` branches are sprint implementation
  branches, created from `develop`

Branching rule:

- harden the phase plan on `plan/<phase>`, branched from `develop`
- create sprint implementation worktrees from `develop`
- merge accepted sprint branches back into `develop` via PR
- `main` only receives merges at release time — see CLAUDE.md `## GitHub —
  Account: w3dev33` → `### Releases`

Every worktree created for this skill's work — the planning worktree and
every sprint worktree — must be recorded in
`../beads-task-issue-tracker-worktrees/worktree-tracking.md`, following the
existing table format (columns: Branch, Path, Base, Purpose, Owner, Created,
Status).

## Authority Rules

- the sprint doc is authoritative for implementation scope
- the phase plan is authoritative for sprint ordering and phase-level
  closure
- downstream QA or task prompts may summarize scope, but they must not
  replace the sprint doc

## Sprint Doc Minimum Shape

Every sprint doc should contain:

- Goal
- Hard Dependencies
- Exact Targets
- Deliverables
- Required Work
- Explicit Code Samples
- This Sprint Does Not Close
- Acceptance Criteria
- Required Validation

For "Required Validation" in btit, the applicable gates are (see
CLAUDE.md → `### Session Completion` and `### Testing`):

- `pnpm test`
- `npx vue-tsc --noEmit`
- `cargo test` (run with `--manifest-path src-tauri/Cargo.toml` outside
  `src-tauri/`)
- `cargo clippy --all-targets` (run from `src-tauri/`, or with
  `--manifest-path src-tauri/Cargo.toml`)

Only list the gates actually relevant to a sprint's changes (e.g. a
docs-only sprint lists none of these; a Rust-only sprint skips `pnpm test`
and `vue-tsc`).

## Split Rule

Split a sprint before implementation if:

- multiple closure types are mixed together
- one sprint would touch too many modules or boundaries for clear ownership
- a deliverable could silently slip while the sprint still claims success
- production-ready closure is not credible for every committed deliverable

## Construction Workflow

When hardening:

1. read the current phase plan and existing sprint docs
2. identify missing sprint docs and overloaded sprints
3. confirm the planning branch and sprint worktree model for the phase
4. create or split sprint docs using
   `.claude/skills/plan-hardening/references/sprint-plan.md.j2`
5. tighten deliverables, acceptance criteria, and validation until the
   sprint docs are directly consumable by implementation and QA
6. only then move into the hostile critical review pass
