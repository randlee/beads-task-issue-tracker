# Step 5 — Consistency Hardening (`plan-coordinator`, in-session)

As in Steps 1 and 3, this pass is performed directly by the driving Claude
Code session.

## Execute

**1. Render the task (optional but recommended)**

If `sc-compose` is installed:

```bash
sc-compose render \
  --root .claude/skills/plan-hardening \
  --file 03-consistency-hardening.xml.j2 \
  --var-file /tmp/plan-hardening-vars.json \
  --output /tmp/step-5-message.xml
```

The vars file or rendered task must include `step-4` fenced JSON as the
required input payload. It must also carry current round metadata:
- `round_id`
- `round_index`
- `replay_nonce`
- `reviewed_commit`
- `previous_reviewed_commit`
- `findings_hash`

If `sc-compose` is not available, read `03-consistency-hardening.xml.j2`
directly and follow its `<plan-hardening-consistency>` instructions
inline, passing in the Step 4 fenced JSON as required input.

**2. Perform the pass**

Follow the `<plan-hardening-consistency>` audit-phase / fix-phase / repeat
loop described in `03-consistency-hardening.xml.j2`. Do not relax or
reinterpret the already-approved sprint deliverables from Steps 1–4;
eliminate document inconsistency, contradiction, ambiguity, undefined
types/interfaces, and missing boundary coverage instead.

**3. Produce the fenced JSON**

Produce fenced JSON matching the expected output shape specified inside
`03-consistency-hardening.xml.j2`. Do not proceed to Step 6 until that
fenced JSON is present and well formed. Save it to `/tmp/step-5.json`.

**4. Route by status**

- `PASS` -> proceed to Step 6
- `FAIL` -> repeat this pass
- if this pass would otherwise return unchanged fenced JSON on a rerun of
  an already-fixed round, increment `round_index`, update `round_id`, and
  refresh `replay_nonce` with the current UTC timestamp before repeating
- if Step 5 fails to converge after three rounds, stop and report
  `cap-exhausted / not converged`; do not ask the user what to do mid-loop

## Hard stops

- `step-4` fenced JSON from the Step 4 response is missing or malformed: do
  not advance; identify the missing or malformed fields explicitly and
  request a corrected Step 4 response
- fenced JSON is missing or malformed: do not advance; identify the missing
  or malformed fields explicitly and redo the pass
- Step 5 has failed to converge after three rounds: do not advance; stop
  and report `cap-exhausted / not converged` without prompting the user
