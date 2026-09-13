# beads-task-issue-tracker — code health report

Run 2026-09-13 | base `develop @ e3da9bc` | repowise 0.50.0 | provider `codex_cli` (LLM analysis)

## Scores

| Metric | Value |
|---|---|
| Hotspot health (worst file) | **1.65/10** — `src-tauri/src/lib.rs` |
| Average health (231 files) | **5.82/10** |
| Maintainability avg / hotspot | 5.30 / 1.30 |
| Structure avg / hotspot | 2.85 / 4.86 |
| History avg / hotspot | 1.33 / 3.50 |
| Performance avg / hotspot | 9.24 / 8.00 |
| Production files | 217 of 231 |

## Top hot spots (worst 10)

| File | Score | Max CCN | NLOC | Dup % | Why |
|---|---|---|---|---|---|
| `src-tauri/src/lib.rs` | 1.65 | 102 | 4572 | 25.4% | prior_defect (3 bug-fixes ~6mo), 126 findings, critical |
| `app/composables/useIssueDialogs.ts` | 5.94 | 81 | 463 | 36.5% | nested complexity (5 levels) |
| `app/composables/useIssues.ts` | 5.94 | 130 | 648 | 52.5% | nested complexity + heavy duplication |
| `app/utils/bd-api.ts` | 6.17 | 14 | 697 | 23.4% | nested complexity |
| `app/composables/useFavorites.ts` | 6.89 | 13 | 171 | — | nested complexity |
| `app/composables/useBeadsPath.ts` | 7.17 | 11 | 77 | — | nested complexity |
| `server/api/fs/list.get.ts` | 7.25 | 14 | 62 | — | nested complexity + serial-await perf |
| `app/utils/issue-helpers.ts` | 7.55 | 24 | 300 | 27.1% | bumpy_road (bumpy call sites) |
| `app/composables/useKeyboardNavigation.ts` | 7.56 | 22 | 76 | — | complex method |
| `app/composables/useRepairDatabase.ts` | 7.69 | 13 | 91 | 10.3% | nested complexity |

## Dead code (178 findings)

| Kind | Count | Notes |
|---|---|---|
| unreachable_file | 93 | no importers (in_degree=0) |
| unused_export | 84 | exported, never imported |
| zombie_package | 1 | whole `src-tauri` flagged (conf 0.50 — treat as noise; the crate does compile) |

All findings `safe_to_delete: false` — verify before removal. Largest:
`app/utils/bd-api.ts` (697 NLOC unreachable_file, conf 0.40),
`app/composables/useIssueDialogs.ts` (510 unused_export / 380 unreachable),
`app/composables/useIssues.ts` (210 unreachable).

## Ready refactor plans (20 total: 16 extract_method, 2 split_file, 1 perf, 1 helper)

**Splits (high-confidence, dataflow-evidenced):**
1. `src-tauri/src/lib.rs` → **9 files** (binary, remove, attachment, check, config, logs, binary_2, issue + residual) — modularity 0.528, 9 groups, 117 symbols. **XL effort, the single biggest win in the repo.**
2. `app/utils/issue-helpers.ts` → 2 files — L effort.

**Perf (proven safe):** `server/api/fs/list.get.ts:39` — parallelize independent awaits in loop (serial_await_in_loop, no cross-iteration dependence, boundary=filesystem, prod context).

**S-effort mechanical extracts (high confidence, each ≤2 steps):**
- `server/api/bd/count.get.ts` — extract `defineEventHandler callback` (ccn −4)
- `server/utils/bd-transformers.ts` — extract `transformToRaw` (ccn −4)
- `server/utils/bd-executor.ts` — extract `bdCreate` + `bdUpdate` (ccn −15 each)
- `server/api/fs/list.get.ts` — extract handler callback (ccn −2)

## Where the numbers live

- `.sc/repowise/data/repowise-health.json` — KPIs + per-file metrics (231 files)
- `.sc/repowise/data/repowise-health-rt.json` — refactoring targets, plans, opportunities
- `.sc/repowise/data/repowise-dead-code.json` — 178 dead-code findings
- `.repowise/` — repowise index (250 pages, 593k tokens, codex_cli)

## Scope caveat

- Coverage is not reported (cov=N/A everywhere — this repo has no coverage data wired into repowise).
- The `zombie_package` finding on `src-tauri` is an in-degree artifact of a Tauri crate (entry via tauri macro, not imports) — disregard.
- 231 scored files ≈ full-repo (app + server + src-tauri + tests). No `.sc/repowise.yaml` scope config exists for this repo yet.
