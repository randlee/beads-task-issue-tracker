# Dolt migration — recovery notes for legacy bd projects

Internal notes. Scope: recovering a project that was left half-migrated by **bd 0.50–0.56**, the versions that moved beads from SQLite+JSONL to Dolt. Projects created by **bd 1.x** are Dolt from the start and do not need any of this.

## What happened here (2026-02-18, bd 0.52)

**Symptom.** After upgrading to bd 0.52 the project showed an empty list: `bd list` returned `[]` with no error, `.beads/dolt/` existed (a partial migration), and `.beads/.dolt` was missing.

**Cause.** bd 0.52 started an automatic migration on a write, created `dolt/`, and did not finish. bd then read from the empty Dolt database instead of SQLite.

## Recovery

`.beads/issues.jsonl` is the source of truth. While it is intact, the issues are recoverable.

```bash
# 1. Confirm the JSONL still holds the issues
wc -l .beads/issues.jsonl

# 2. Kill anything holding a lock on the database
pkill -f "bd doctor"; pkill -f "bd daemon"; pkill -f dolt
rm -f .beads/daemon.lock .beads/daemon.pid .beads/bd.sock .beads/dolt-access.lock .beads/.jsonl.lock

# 3. Remove the partial Dolt directory and the corrupt SQLite database
rm -rf .beads/dolt .beads/beads.db .beads/beads.db-shm .beads/beads.db-wal

# 4. Re-initialise with the prefix the issue IDs already use (check issues.jsonl)
bd init --prefix beads

# 5. Drop tombstones (deleted issues; older bd rejects them on import), then import
python3 -c "
import json
with open('.beads/issues.jsonl') as f, open('/tmp/beads-clean.jsonl', 'w') as out:
    for line in f:
        line = line.strip()
        if not line: continue
        try:
            issue = json.loads(line)
            if issue.get('status') == 'tombstone': continue
            out.write(json.dumps(issue) + '\n')
        except Exception: continue
"
bd import -i /tmp/beads-clean.jsonl

# 6. Verify
bd count
bd list --limit=5
```

Notes:
- **The prefix must match.** `bd init --prefix X` has to match the prefix in the JSONL issue IDs (`beads-0bn` → `beads`).
- **Dolt locks.** `bd doctor` can create a lock and then block on it. Kill running bd/dolt processes before retrying.
- **`bd migrate --to-dolt` is gone in bd 1.x** (issue #74). On bd 1.x, `bd init` plus `bd import` is the path; the app's own migration command falls back to it automatically.

## When a partial migration can still happen

Only on legacy bd (0.50–0.56): the version is installed, a write command runs against a SQLite project, and the automatic migration fails or is interrupted.

Indicators:
- `.beads/dolt/` exists but `bd list` returns `[]`
- `.beads/dolt-access.lock` is present
- `bd list` reports `Dolt backend configured but database not found`

## How the app handles it today

1. **Explicit error.** `isDoltMigrationError()` (`app/utils/bd-api.ts`) catches `Dolt backend configured but database not found` and raises the migration banner.
2. **Proactive check.** `bd_check_needs_migration` reports whether the project still needs migrating. On bd ≥ 0.51 (and for an unknown or unprobed CLI) Dolt detection reads `.beads/metadata.json` with bd's own backend rule, so embedded (`embeddeddolt/`), server mode and custom data directories all detect correctly. bd 0.50.x keeps the old filesystem probe (`crates/btit-bd/src/dolt.rs`).
3. **Partial migration.** `bd_migrate_to_dolt` removes a leftover `dolt/` directory and `dolt-access.lock` before retrying, and falls back to init plus import (including label, dependency and comment restore) when `bd migrate` is unavailable or fails. An empty project takes the init-only path (`crates/btit-app/src/migration.rs`).
