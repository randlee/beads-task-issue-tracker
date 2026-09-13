# Step 1 — Plan Scope Review (`plan-coordinator`, in-session)

btit has no ATM/NTM multi-agent swarm, so there is no separate `arch-ctm`
teammate to send this step to (unlike the atm-core source skill). The
Claude Code session driving `/plan-hardening` performs this pass directly,
in the same session, using the rendered task as its instructions.

## Execute

**1. Render the task (optional but recommended)**

If `sc-compose` is installed (`which sc-compose`):

```bash
sc-compose render \
  --root .claude/skills/plan-hardening \
  --file 01-plan-scope-review.xml.j2 \
  --var-file /tmp/plan-hardening-vars.json \
  --output /tmp/step-1-message.xml
```

If `/tmp/plan-hardening-vars.json` does not exist, start from:

`.claude/skills/plan-hardening/examples/plan-hardening-vars.example.json`

Make sure the vars file includes the current round metadata:
- `round_id`
- `round_index`
- `replay_nonce`
- `reviewed_commit`
- `previous_reviewed_commit`
- `findings_hash`

If `sc-compose` is not available, read `01-plan-scope-review.xml.j2` and
`sprint-planning-guidelines.md` directly and follow the
`<plan-hardening>` and `<workflow>` instructions inline — the rendered XML
is a convenience, not a hard requirement.

**2. Perform the pass**

Read the current plan docs from disk first (`source_of_truth` and
`references`), then `sprint-planning-guidelines.md` and
`references/plan-construction-notes.md`, and follow the
`<plan-hardening>` instructions in `01-plan-scope-review.xml.j2`:
tighten the current plan docs to the guidelines, create missing sprint docs
when the phase plan already implies they are needed, and make the sprint
set reviewable by `phb-plan-scope-reviewer` (Step 2).

Do not materially change user-discussed scope. If task framing or repo
drift implies a substantial deliverable-scope change from what the user
already discussed, stop and raise it with the user directly before
continuing — there is no `plan-coordinator` to escalate to separately, so
this session must surface it itself.

**3. Produce the fenced JSON**

Produce fenced JSON matching the expected output shape specified inside
`01-plan-scope-review.xml.j2`. Do not proceed to Step 2 until that fenced
JSON is present and well formed. Save it to `/tmp/step-1.json`.

**4. Route by status**

- `PASS` -> proceed to Step 2
- `FAIL` -> repeat this pass and re-render if using `sc-compose`
- if this pass would otherwise return unchanged fenced JSON on a rerun,
  increment `round_index`, update `round_id`, and refresh `replay_nonce`
  with the current UTC timestamp before repeating

## Hard stops

- worktree does not exist: create it under
  `../beads-task-issue-tracker-worktrees/` from `develop` before running
  this step, and record it in
  `../beads-task-issue-tracker-worktrees/worktree-tracking.md`
- fenced JSON is missing or malformed: do not advance; identify the missing
  or malformed fields explicitly and redo the pass
