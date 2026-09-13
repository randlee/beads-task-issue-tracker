---
name: plan-hardening
version: 1.0.0
description: >
  Plan-coordinator drives plan hardening for a beads-task-issue-tracker (btit)
  phase/sprint plan after the current plan state already exists in repo docs.
---

# Plan Hardening

Audience: the Claude Code session acting as `plan-coordinator`.

Use this only for phase-plan hardening before implementation starts or
resumes. This is a port of the `plan-hardening` skill from
[atm-core](https://github.com/) (`.claude/skills/plan-hardening/`), adapted
for btit — a single-session Claude Code repo with no ATM/NTM multi-agent
swarm. Where the source skill routed steps through ATM teammates
(`team-lead`, `arch-ctm`, `quality-mgr`) via `atm send`, this port has the
`plan-coordinator` (the driving Claude Code session) perform those steps
directly in-session, and uses the `Agent` tool for the two background
review passes. See the porting PR for the full dependency list.

## Assumptions

- the current plan state already exists in repo docs, though sprint docs may
  still be partial or missing
- do not ask the user to explain detailed plan content; read the planning
  docs and references directly after they are created
- `plan-coordinator` routes and performs the hardening passes but is not the
  sole authority for rewriting the plan — findings from the two review
  passes below must be applied before the plan is considered hardened
- the user-discussed deliverable scope is authoritative
- the planning line starts from `develop` (btit's integration branch; `main`
  is release-only)
- if no target phase worktree exists, create one under
  `../beads-task-issue-tracker-worktrees/` from `develop` before starting,
  and record it in `../beads-task-issue-tracker-worktrees/worktree-tracking.md`

## Branch Model

Use this branch/worktree model unless the user explicitly says otherwise.
btit has no intermediate `integrate/<phase>` branch (unlike the atm-core
original) — feature and planning worktrees branch directly from `develop`,
matching existing btit practice (see
`../beads-task-issue-tracker-worktrees/worktree-tracking.md`):

- `develop`
  - baseline integration branch (PRs target this, not `main`)
- `plan/<phase>`
  - planning branch/worktree for phase-plan hardening, created from `develop`
- `feature/<phase>-<sprint>-...`
  - sprint implementation branches, created from `develop`

Execution rule:

- harden the phase plan on `plan/<phase>`, branched from `develop`
- create sprint implementation worktrees from `develop`
- accepted sprint branches merge back into `develop` via PR
- `main` only receives merges at release time (see CLAUDE.md `## GitHub —
  Account: w3dev33` → `### Releases`)

## Expected Result

Sprint plan approved by:
- `phb-plan-scope-reviewer` (background agent, `.claude/agents/phb-plan-scope-reviewer.md`)
- `phb-critical-plan-reviewer` (background agent, `.claude/agents/phb-critical-plan-reviewer.md`)
- `plan-coordinator`'s own quality-gate pass (Step 6 — btit has no separate
  QA teammate; see Step 6)

## Required Reference

Always use:
- `.claude/skills/plan-hardening/sprint-planning-guidelines.md`
- `.claude/skills/plan-hardening/references/plan-construction-notes.md`

When creating a missing sprint doc or normalizing an incomplete one, also
use:
- `.claude/skills/plan-hardening/references/sprint-plan.md.j2`

## Execution Table

| # | Performed by | Input required | Output expected | Read before executing |
|---|----------|----------------|-----------------|-----------------------|
| 1 | `plan-coordinator` (in-session) | vars file | `step-1` fenced JSON | `steps/step-1.md` |
| 2 | `phb-plan-scope-reviewer` (background, via `Agent` tool) | context + `step-1` JSON | `step-2` fenced JSON | `steps/step-2.md` |
| 3 | `plan-coordinator` (in-session) | `step-2` JSON | `step-3` fenced JSON | `steps/step-3.md` |
| 4 | `phb-critical-plan-reviewer` (background, via `Agent` tool) | context + `step-3` JSON | `step-4` fenced JSON | `steps/step-4.md` |
| 5 | `plan-coordinator` (in-session) | `step-4` JSON | `step-5` fenced JSON | `steps/step-5.md` |
| 6 | `plan-coordinator` (in-session quality gates) | `step-5` JSON | pass/fail on btit quality gates | `steps/step-6.md` |

## Round Tracking

`plan-coordinator` must keep a round table for every `/plan-hardening` run.

Minimum columns:

| Round | Step | Reviewer | reviewed_commit | status | blocking | important | minor | findings_hash | supersedes | Note |
|-------|------|----------|-----------------|--------|----------|-----------|-------|---------------|------------|------|

Use the example in:
- `.claude/skills/plan-hardening/examples/plan-hardening-rounds.example.md`

## Reviewer Cycle Caps

- `phb-plan-scope-reviewer` and `phb-critical-plan-reviewer` both default to
  a 3-cycle cap
- these caps must be carried in JSON:
  - `plan_scope_review_cycle_limit`
  - `critical_review_cycle_limit`
- reviewer launch payloads must also include:
  - `review_cycle_limit`
  - `review_cycle_index`
- if the vars JSON omits the cap fields, `plan-coordinator` must default them
  to `3`

Cycle-cap behavior:

- every `FAIL` from `phb-plan-scope-reviewer` or `phb-critical-plan-reviewer`
  must be routed back through the matching in-session plan-editing step
  immediately
- no reviewer findings may be accepted as-is or bypass the plan-editing pass
- if a reviewer returns `FAIL` on the final allowed reviewer cycle,
  `plan-coordinator` must still perform one final correction pass over the
  findings
- after that final correction pass, if no reviewer cycles remain, stop the
  hardening run as `cap-exhausted / not converged` and report status plainly
- do not ask the user how to proceed, do not offer multiple-choice options,
  and do not invent an "accept and proceed" path

## Hard Stops

- `plan-coordinator` only checks the top-level `status` and expected `mode`
  fields on each fenced JSON response before advancing
- every step after step 1 must receive the previous step's fenced JSON
- missing or malformed fenced JSON is a hard stop
- a reviewer rerun is valid only when either `reviewed_commit` changed or
  `findings_hash` changed
- if the same reviewer returns the same `reviewed_commit` and the same
  `findings_hash` again, treat it as a stale replay and do not open a new
  hardening round
- substantial scope drift from the user-discussed plan is a hard stop
- remaining in-scope work without sprint ownership is a hard stop
- if a sprint cannot credibly land its committed deliverables at a
  production-ready level, split it before implementation
- if a reviewer loop reaches its configured cap without converging, stop
  after the final in-session correction pass and report `cap-exhausted / not
  converged`; do not continue launching background reviewers and do not ask
  the user for a decision mid-loop

## Render

- `.claude/skills/plan-hardening/01-plan-scope-review.xml.j2`
- `.claude/skills/plan-hardening/02-sprint-scope-hardening.xml.j2`
- `.claude/skills/plan-hardening/03-consistency-hardening.xml.j2`
- `.claude/skills/plan-hardening/steps/step-1.md`
- `.claude/skills/plan-hardening/steps/step-2.md`
- `.claude/skills/plan-hardening/steps/step-3.md`
- `.claude/skills/plan-hardening/steps/step-4.md`
- `.claude/skills/plan-hardening/steps/step-5.md`
- `.claude/skills/plan-hardening/steps/step-6.md`
- `.claude/skills/plan-hardening/examples/plan-hardening-vars.example.json`
- `.claude/skills/plan-hardening/examples/plan-hardening-rounds.example.md`
- `.claude/skills/plan-hardening/examples/plan-hardening-qa-vars.example.json`
- `.claude/skills/plan-hardening/sprint-planning-guidelines.md`
- `.claude/skills/plan-hardening/references/plan-construction-notes.md`
- `.claude/skills/plan-hardening/references/sprint-plan.md.j2`
