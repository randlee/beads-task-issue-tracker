# Step 3 — Sprint Scope Hardening (`plan-coordinator`, in-session)

As in Step 1, this pass is performed directly by the driving Claude Code
session — btit has no `arch-ctm` teammate to route this to.

## Execute

**1. Render the task (optional but recommended)**

If `sc-compose` is installed:

```bash
sc-compose render \
  --root .claude/skills/plan-hardening \
  --file 02-sprint-scope-hardening.xml.j2 \
  --var-file /tmp/plan-hardening-vars.json \
  --output /tmp/step-3-message.xml
```

The vars file or rendered task must include `step-2` fenced JSON as the
required input payload. It must also carry current round metadata:
- `round_id`
- `round_index`
- `replay_nonce`
- `reviewed_commit`
- `previous_reviewed_commit`
- `findings_hash`

If `sc-compose` is not available, read `02-sprint-scope-hardening.xml.j2`
directly and follow its `<plan-hardening>` instructions inline, passing in
the Step 2 fenced JSON as required input.

**2. Perform the pass**

Follow the `<plan-hardening>` audit-phase / fix-phase / repeat loop
described in `02-sprint-scope-hardening.xml.j2`, using
`sprint-planning-guidelines.md` and
`references/plan-construction-notes.md` as required references. Work
through every finding from Step 2 (or from a prior Step 4 `FAIL`) until the
audit phase produces zero findings.

**3. Produce the fenced JSON**

Produce fenced JSON matching the expected output shape specified inside
`02-sprint-scope-hardening.xml.j2`. Do not proceed to Step 4 until that
fenced JSON is present and well formed. Save it to `/tmp/step-3.json`.

**4. Route by status**

- `PASS` -> proceed to Step 4
- `FAIL` -> repeat this pass
- if this pass would otherwise return unchanged fenced JSON on a rerun of
  an already-fixed round, increment `round_index`, update `round_id`, and
  refresh `replay_nonce` with the current UTC timestamp before repeating

Maintain the round table after every Step 3 / Step 4 loop:

| Round | Step | Reviewer | reviewed_commit | status | blocking | important | minor | findings_hash | supersedes | Note |
|-------|------|----------|-----------------|--------|----------|-----------|-------|---------------|------------|------|

## Hard stops

- `step-2` fenced JSON from the Step 2 response is missing or malformed: do
  not advance; identify the missing or malformed fields explicitly and
  request a corrected Step 2 response
- fenced JSON is missing or malformed: do not advance; identify the missing
  or malformed fields explicitly and redo the pass
