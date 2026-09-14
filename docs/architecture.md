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
