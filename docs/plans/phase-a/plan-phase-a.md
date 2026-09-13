---
phase: a
title: "phase-a: sc-observability-log — log bridge, tracing-compatible macros, and sc-observability handoff"
canonical_path: docs/plans/phase-a/plan-phase-a.md
planning_branch: plan/phase-a
integration_branch: develop
status: draft
owner: Rand Lee
authored: 2026-09-13
baseline: develop@1e493bf
---

# phase-a: sc-observability-log

## Why this phase exists

btit is being migrated into the sc ecosystem. Its logging is plain text today:

- **Backend:** `log` 0.4 plus `tauri-plugin-log` writing `beads.log` (`src-tauri/src/lib.rs:43-53`, `src-tauri/Cargo.toml:21,23`).
- **Frontend:** `logFrontend()` forwards frontend messages into that file.

`sc-observability` 1.2.0 (crates.io) writes structured JSONL but has none of the following (verified in `../sc-observability` at 1.2.0):

- no `log`/`tracing` bridge (the dependency ban in `scripts/ci/validate_dependency_bans.sh` forbids them in the core crates)
- no event macros
- no attributes
- no compile-time tooling

phase-a builds that missing layer as a new crate pair, `sc-observability-log` and `sc-observability-log-macros`. Every sprint in the phase is written for the final destination, `../sc-observability`.

**Adoption goal (user direction, 2026-09-13):** the public macro and attribute API matches the API of commonly used tools, so a future migration means referencing a couple of crates and renaming imports:

- **`log` users:** existing `log::info!` call sites keep working unchanged through the bridge.
- **`tracing` users:** swap `use tracing::{info, instrument}` for `use sc_observability_log::{info, instrument}`.

The final sprint delivers a critical review by the sc-observability team and copies the crates into `../sc-observability` for publishing.

## Binding outcomes

### Destination-first crate contract

These rules apply to both crates from the first sprint onward:

- Both crates live under `crates/` at the btit repo root, in a transitional phase-a Cargo workspace at `crates/Cargo.toml`.
- They are built to sc-observability's standards: `edition = "2024"`, `rust-version = "1.94.1"`, `license = "MIT"`, and `cargo fmt --check` plus `cargo clippy --all-targets --all-features -- -D warnings` clean. These are the sc-observability CI gates (`../sc-observability/.github/workflows/ci.yml` lines 24, 57).
- **Dependencies:** they depend only on published crates (`sc-observability = "1.2.0"`, `sc-observability-types = "1.2.0"`), never on btit code.
- **Publishing:** `publish = false` and `version = "0.1.0"` while in btit. The final sprint adopts the sc-observability workspace version.
- **btit usage:** btit consumes the crates only through path dependencies from `src-tauri/Cargo.toml`.

### Compatibility contract

| Existing tool API | sc-observability-log equivalent | Sprint |
|---|---|---|
| `log::{trace,debug,info,warn,error}!` including `target:` and the `kv` syntax `key = value; "msg"` | unchanged call sites, captured by `sc_observability_log::init` (a `log::Log` implementation) | a-1 |
| `tracing::{trace,debug,info,warn,error,event}!` with `target:`, `name:`, `parent:` (rejected), fields `a.b = v`, `?v`, `%v`, shorthand `v`, and a format message | `sc_observability_log::{trace,debug,info,warn,error,event}!` | a-2 |
| `tracing::instrument` with `name`, `target`, `level`, `skip`, `skip_all`, `fields`, `ret`, `err` (plus `err(Display)`/`err(Debug)`, `ret(Display)`/`ret(Debug)`) on sync and async fns | `sc_observability_log::instrument` | a-3 |

**Mapping rules:**

- **Target:** `target` maps to `TargetCategory`, after sanitizing to `[A-Za-z0-9._-]+`, with `::` becoming `.`.
- **Action:** the tracing `name`, the instrument `name`, or a leading `[tag]` in a `log` message maps to `ActionName`.
- **Fields:** fields go into `LogEvent.fields` as JSON values.
- **Unsupported tracing arguments:** these are rejected at compile time with a message naming the argument. They are `parent:`, `follows_from`, and span macros (`info_span!` etc.).

### btit adoption (option 1)

- btit replaces `tauri-plugin-log` with the a-1 bridge.
- None of its 200 Rust log call sites or 38 `logFrontend` call sites change.
- The log file becomes `<app_log_dir>/logs/beads-task-issue-tracker.log.jsonl`.
- The debug panel renders JSONL.

## Issue inventory

| Planning id | Disposition | Closure |
| --- | --- | --- |
| `a-log-bridge` | In scope | a-1: the `log::Log` → `LogEvent` bridge crate, global handle, guard, and pure mapping with tests. |
| `a-event-macros` | In scope | a-2: tracing-compatible event macros (proc-macro) emitting `LogEvent` directly. |
| `a-instrument` | In scope | a-3: tracing-compatible `#[instrument]` attribute, sync and async. |
| `a-btit-adoption` | In scope | a-4: btit switches from `tauri-plugin-log` to the bridge; log commands and the debug panel read JSONL. |
| `a-sc-handoff` | In scope | a-5: sc-observability team critical review; crates copied into `../sc-observability` with every CI gate green there. |
| `b-crate-split` | Out of phase | Moving the btit backend to root `crates/` and splitting it into multiple crates is phase-b. It depends on phase-a landing first (user direction, 2026-09-13). |
| `REFACTOR-REVIEW-B1..B13` | Out of phase | Behavior issues pinned by tests, recorded in `docs/crate-split-refactor-issues.md`. phase-a does not change btit issue/CLI behavior. |
| `REFACTOR-REVIEW-A1` | Closed by a-4 | The `app_lib::module` log-target churn stops mattering. The target becomes a `LogEvent.target` field rendered by the debug panel. |

## Prerequisites (outside this phase)

- **Toolchain:** PR #36 (`chore/toolchain-and-version-ssot`) must be merged to `develop`. It pins Rust 1.98.1 (≥ 1.94.1, as the sc-observability crates require) and makes `src-tauri/Cargo.toml` a workspace with `[workspace.package]`.
- **Planning skill:** PR #35 (`chore/plan-hardening-skill`) must be merged, so this plan can be hardened with `/plan-hardening`.

## Sprint sequence

| Sprint | Status | Branch | Authoritative plan | Production closure |
| --- | --- | --- | --- | --- |
| `a-1` | `planned` | `feature/sprint-a-1-log-bridge` | [`sprint-a-1.md`](./sprint-a-1.md) | `crates/` workspace plus `sc-observability-log` bridge, CI on 3 OSes |
| `a-2` | `planned` | `feature/sprint-a-2-event-macros` | [`sprint-a-2.md`](./sprint-a-2.md) | tracing-compatible event macros with a compatibility fixture |
| `a-3` | `planned` | `feature/sprint-a-3-instrument` | [`sprint-a-3.md`](./sprint-a-3.md) | tracing-compatible `#[instrument]` (sync and async) with a compatibility fixture |
| `a-4` | `planned` | `feature/sprint-a-4-btit-adoption` | [`sprint-a-4.md`](./sprint-a-4.md) | btit on the bridge: JSONL file, log commands, debug panel |
| `a-5` | `planned` | `feature/sprint-a-5-sc-observability-handoff` | [`sprint-a-5.md`](./sprint-a-5.md) | sc-observability team review resolved; crates in `../sc-observability` with CI green, ready for its release workflow |

The phase runs strictly in sequence.

- **a-2 and a-3 build on a-1:** both extend a-1's public crate surface and the shared emit path.
- **a-4 needs a final API:** it consumes the final crate API and the `src-tauri/Cargo.lock` entries.
- **a-5 needs everything:** it reviews and copies the complete, adopted crates.

No deliverable is repeated across sprint checklists.

## Dependency relations

- **`a-1 must_follow` the toolchain PR:** PR #36 (`chore/toolchain-and-version-ssot`) must merge first. Rust ≥ 1.94.1 and the toolchain pin are required to build `sc-observability` 1.2.0.
- **`a-2 must_follow a-1`:** a-1 development must be pushed before a-2 begins. Merge a-1 into a-2 before every dev/fix round. The a-1 PR merges first.
- **`a-3 must_follow a-2`:** the same rules apply. a-3 reuses a-2's field-parsing module (`crates/sc-observability-log-macros/src/fields.rs`).
- **`a-4 must_follow a-3`:** the same rules apply. a-4 pins the complete crate API and path dependencies in `src-tauri/Cargo.lock`.
- **`a-5 must_follow a-4`:** the same rules apply. The review covers the crates as actually adopted by a consumer.

None of these relations is `parallel_safe`. a-1–a-3 intersect on `crates/sc-observability-log/src/lib.rs` (the public re-exports) and `crates/Cargo.lock`, and a-4 intersects on `src-tauri/Cargo.lock`.

## Cross-sprint document ownership

- **a-1** creates `crates/sc-observability-log/README.md` (bridge section) and `crates/sc-observability-log/docs/mapping.md` (the record → `LogEvent` mapping).
- **a-2 and a-3** extend `crates/sc-observability-log/docs/compatibility.md` (the tracing/log API compatibility table and the list of rejected arguments).
- **a-4** updates btit `CLAUDE.md` → `### Logging` (file location and format) and `.claude/codebase-map.md` (logging section).
- **a-5** owns every document added to `../sc-observability`.
- **Each sprint** updates only its own row status in this plan.

## Phase closure

phase-a closes when all of the following hold:

- a-1–a-5 are merged.
- btit `develop` runs on the bridge.
- A PR in `../sc-observability` adding both crates is merged, with all sc-observability CI gates green and review findings resolved.

**Not part of phase-a:**

- **Publishing to crates.io:** that runs through sc-observability's release workflow and publisher, per `../sc-observability/docs/publishing.md`.
- **Switching btit to the crates.io release:** the first phase-b sprint does this.
