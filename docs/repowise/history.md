# beads-task-issue-tracker — repowise run history

One row per run, oldest first. Appended automatically each run (do not reorder).

| Date | Base | Hotspot | Avg (files) | Worst file | Dead (findings) | Plans | Provider | Ref |
|---|---|---|---|---|---|---|---|---|
| 2026-09-13 | `develop @ e3da9bc` | **1.65/10** | 5.82 / 231 | `src-tauri/src/lib.rs` (1.65, CCN 102, 4572 NLOC) | 178 (93 unreachable + 84 unused_export + 1 zombie) | 20 (2 split_file, 16 extract_method, 1 perf, 1 helper) | codex_cli (repowise 0.50.0) | PR #TBD |

## Notes

- 2026-09-13: first repowise run on this repo. `src-tauri/src/lib.rs` is the
  dominant hotspot: 4572 NLOC, 117 symbols, CCN 102, 3 bug-fixes in ~6 months.
  A dataflow-evidenced 9-way split is planned. App-layer composables cluster
  around 5.9–6.2 health. No prior baseline to compare (this is row one).
