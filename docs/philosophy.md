# Philosophy: A Human Window into AI-Piloted Work

## Core Purpose

Beads Task-Issue Tracker is **a human control panel** for an AI-native issue tracker.

The Beads CLI (`bd` or `br`) is designed for AI agents — they create issues, update statuses, and pilot workflows programmatically. This application gives humans the visibility and controls to observe what the AI is driving, and to step in when needed.

## Following the CLI, Not Leading It

This application reads what the CLI writes and presents it for humans. It does not define the issue format, the database schema, or the sync protocol.

When the CLI evolves, we adapt. When it adds features, we surface them. But if the CLI becomes purely machine-to-machine with no human-interpretable output, we freeze at the last meaningful version. The goal is human readability.

## CLI Policy: bd First, br Supported

The original Beads philosophy was simple: issues stored as JSONL, backed by SQLite, with a lightweight CLI that writes human-readable files. The original author of this app preferred [`br`](https://github.com/Dicklesworthstone/beads_rust) for freezing at that classic architecture, and pinned `bd` at 0.49.x when bd 0.50–0.56 moved toward embedded Dolt and server mode.

This project is now maintained independently and follows current [`bd`](https://github.com/steveyegge/beads):

- **`bd` 1.x** (primary) — the CLI the maintainer runs and new features target
- **`bd` < 1.0** (legacy) — may work through version-gated code paths, but the app warns rather than silently degrading
- **`br`** (secondary) — supported and selectable in Settings; kept working, not driven

The guiding principle is unchanged: we follow the CLI and present what it writes for humans. When `bd` evolves, we adapt.

## Design Principle: Observe and Edit

Two complementary roles:

1. **Observe** — dashboards, issue lists, change detection
2. **Edit** — create, update, comment, reassign, close

The AI pilots through the CLI, the human monitors and corrects through this app. This is a control panel, not a workspace — lightweight, fast to open, immediately useful.

---

*Originally written by Laurent Chapin. CLI policy section updated by the current maintainer (September 2026).*
