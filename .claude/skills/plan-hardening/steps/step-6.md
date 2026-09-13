# Step 6 — Quality Gates (`plan-coordinator`, in-session)

The atm-core source skill routed this step to a `quality-mgr` teammate via
`atm send`, using the `codex-orchestration` skill's `qa-template.xml.j2` and
handing off reviewer execution, reporting, and fix routing to that external
QA system. btit has neither a `quality-mgr`/`qa-coordinator` teammate nor a
`codex-orchestration` skill, so there is no equivalent QA system to hand off
to — this port does **not** carry over that handoff. Instead,
`plan-coordinator` runs btit's own quality gates directly against the
hardened plan docs, in-session, as the final step.

## Execute

**1. Identify applicable gates**

List the phase plan and every sprint doc in the current plan state (the
same set used as `source_of_truth`/`references` in Steps 1–5). For each
gate below, apply it only if the corresponding validation commands appear
in the sprint docs' "Required Validation" sections (see
`references/plan-construction-notes.md`):

- `pnpm test`
- `npx vue-tsc --noEmit`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets`

A plan-only sprint (docs, no code changes) applies none of these — skip
this step's command execution and instead confirm the plan docs themselves
are internally consistent, which Steps 1–5 already established.

**2. Run the gates**

Run each applicable command from the repo root (or the planning worktree
root). Record pass/fail for each.

**3. Route by result**

- all applicable gates pass -> the plan is hardened; report completion with
  the final round table
- any gate fails -> this is not a plan-content defect from Steps 1–5; it
  means the plan's own "Required Validation" list does not match what the
  repo can actually run. Fix the sprint doc's validation list (or, if the
  plan is right and the repo is genuinely broken, treat that as an
  out-of-scope finding to raise with the user) and re-run Step 6

**4. Report**

Report the final hardened-plan state to the user: the phase plan and
sprint docs touched, the final round table (Steps 2 and 4), and the Step 6
gate results.

## Hard stops

- `step-5` fenced JSON from the Step 5 response is missing or malformed: do
  not advance; identify the missing or malformed fields explicitly and
  request a corrected Step 5 response
- a sprint doc's "Required Validation" section lists a gate that does not
  exist in btit (i.e. not one of the four commands above, and not
  independently justified in the doc): treat this as an unresolved Step 3/5
  finding and route back accordingly rather than silently running an
  invented command
