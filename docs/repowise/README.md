# beads-task-issue-tracker — repowise code health

This directory is the home for repowise code-health reports and run history.

- `health.md` — latest run's human-readable report (scores, hot spots, dead code, ready refactor plans)
- `history.md` — one row per run, chronological — trend tracker
- `.sc/repowise/data/*.json` — machine-readable data (KPIs, refactoring targets/plans, dead-code findings)
- `.repowise/` — repowise index (provider: codex_cli, 250 pages)

Regenerate after code changes with:
  repowise update --provider codex_cli --yes
  repowise health --format json
  repowise health --format json --refactoring-targets
  repowise dead-code --format json
