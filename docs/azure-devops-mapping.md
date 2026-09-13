# Azure DevOps ↔ beads: scheduling field mapping

**Status:** working document, 2026-09-12. Facts below were verified against:

- Azure DevOps organization `RadiantDev`, project `RadiantSource`, via `az devops invoke --area wit --resource fields` and `--resource workitemtypes` (Azure CLI 2.84, azure-devops extension 1.0.2).
- beads (`bd`) source at `../beads`, `main` = `v1.2.1-64-gb7175be22`. The machine runs bd 1.0.4; the metadata type and ADO mapper are the same in both.

**Goal:** get enough scheduling data from Azure DevOps into beads issues to draw a Gantt chart in this app, using a custom sync script (cron) that writes issue `metadata` directly. `bd ado sync` is not the planned transport; once the mapping is proven, the pieces that belong in bd go upstream as a beads PR.

---

## 1. Azure DevOps scheduling fields

All `Microsoft.VSTS.Scheduling.*` fields present in the organization. Every one is also defined in `RadiantSource`.

| Reference name | Display name | Type |
|---|---|---|
| Microsoft.VSTS.Scheduling.OriginalEstimate | Original Estimate | double (hours) |
| Microsoft.VSTS.Scheduling.RemainingWork | Remaining Work | double (hours) |
| Microsoft.VSTS.Scheduling.CompletedWork | Completed Work | double (hours) |
| Microsoft.VSTS.Scheduling.StartDate | Start Date | dateTime (UTC ISO 8601) |
| Microsoft.VSTS.Scheduling.FinishDate | Finish Date | dateTime |
| Microsoft.VSTS.Scheduling.TargetDate | Target Date | dateTime |
| Microsoft.VSTS.Scheduling.DueDate | Due Date | dateTime |
| Microsoft.VSTS.Scheduling.StoryPoints | Story Points | double |
| Microsoft.VSTS.Scheduling.Effort | Effort | double |
| Microsoft.VSTS.Scheduling.Size | Size | double |

### Which work item types carry which fields (RadiantSource, Agile process)

| Type | Scheduling fields on the type |
|---|---|
| Epic | StartDate, TargetDate, Effort |
| Feature | StartDate, TargetDate, Effort |
| User Story | StartDate, FinishDate, StoryPoints |
| Task | StartDate, FinishDate, OriginalEstimate, RemainingWork, CompletedWork |
| Bug | OriginalEstimate, RemainingWork, CompletedWork, StoryPoints, Severity. **No date fields.** |

Consequences:

- **DueDate and Size are on none of these types.** They are out of scope for this project.
- The planned end date is **FinishDate** for User Story and Task, and **TargetDate** for Epic and Feature. The two never appear on the same type, so "end date" is well defined per item.
- Bugs have effort but no planned dates. A Gantt can show a Bug's progress but must place it by other means (parent, iteration, or actual dates).
- Other planning fields available on every type: `System.Parent`, `System.IterationPath`, `System.AreaPath`, `System.State`, `Microsoft.VSTS.Common.Priority`, `System.CreatedDate`, `System.ChangedDate`, `Microsoft.VSTS.Common.ActivatedDate`, `Microsoft.VSTS.Common.ClosedDate`, `Microsoft.VSTS.Common.StackRank`.

---

## 2. Current state of Azure DevOps support in beads (bd main)

### 2.1 Issue metadata written by `bd ado sync`

Source: `internal/ado/mapping.go` `buildMetadata` / `restoreMetadata`.

| Metadata key | Azure source | Pushed back to Azure on `bd ado push` |
|---|---|---|
| `ado.area_path` | System.AreaPath | yes |
| `ado.iteration_path` | System.IterationPath | yes |
| `ado.story_points` | Microsoft.VSTS.Scheduling.StoryPoints | yes |
| `ado.remaining_work` | Microsoft.VSTS.Scheduling.RemainingWork | **no** (pull only) |
| `ado.severity` | Microsoft.VSTS.Common.Severity | yes |
| `ado.rev` | work item revision | no |
| `beads_priority` | not Azure; original beads priority kept because Azure's 1–4 scale collapses beads 3 and 4 | n/a |

Known inconsistency: `internal/ado/tracker.go` `adoWorkItemToTrackerIssue` builds metadata by hand and writes only `ado.rev`, `ado.area_path`, `ado.iteration_path`, `ado.story_points`; it omits `ado.remaining_work` and `ado.severity` that `buildMetadata` writes.

### 2.2 First-class beads fields filled by the ADO mapper

| beads field | Azure source |
|---|---|
| `title`, `description` (HTML→Markdown) | System.Title, System.Description |
| `status` | System.State via `ado.state_map.*` config |
| `issue_type` | System.WorkItemType via `ado.type_map.*` config |
| `priority` (0–4) | Microsoft.VSTS.Common.Priority (1–4), lossy |
| `owner` | System.AssignedTo |
| `labels` | System.Tags (semicolon list), `beads:*` tags filtered |
| `external_ref` | work item web URL |
| `dependencies` | see 2.3 |

**Not filled by the mapper although beads has the field:** `due_at`, `estimated_minutes`, `started_at`, `defer_until`. `started_at` is set by beads itself on the in-progress transition, i.e. it is an *actual*, not the Azure planned start.

### 2.3 Dependency links (`internal/ado/links.go`)

| Azure relation | beads dependency |
|---|---|
| System.LinkTypes.Dependency-Forward (predecessor) | `blocks` |
| System.LinkTypes.Dependency-Reverse (successor) | `blocks`, direction swapped |
| System.LinkTypes.Hierarchy-Forward / -Reverse | `parent-child` |
| System.LinkTypes.Related | `related` |

This is sufficient for Gantt hierarchy and arrows; nothing is needed here.

### 2.4 Configuration keys (project config, not issue data)

`ado.org`, `ado.project` / `ado.projects`, `ado.url`, `ado.pat`; `ado.state_map.<status>`, `ado.type_map.<type>`; `ado.filter.area_path`, `ado.filter.iteration_path`, `ado.filter.types`, `ado.filter.states`; `ado.reconcile_interval`, `ado.syncs_since_reconcile`.

### 2.5 Metadata contract that any sync script must respect

From `internal/types/types.go`, `internal/storage/metadata.go`, and `docs/core-concepts/metadata.md` in bd:

- `metadata` is a single JSON **object**; bd validates only that it is well-formed JSON, but every consumer assumes an object.
- `bd update --metadata '<json>'` **merges** top-level keys (since bd 0.60). Keys you do not mention are preserved. Use this, not a full replace, so `ado.*` keys written by other tools survive.
- `--set-metadata k=v` always stores a **string**; typed values (numbers, dates as strings, arrays) must go through `--metadata`.
- Keys must match `^[a-zA-Z_][a-zA-Z0-9_./]*$`. Prefixes `bd:` and `_` are reserved by beads.
- `bd list --json` and `bd show --json` include `metadata` on every issue; the key is omitted when empty.

---

## 3. Full mapping table

"Today" is bd main. "Agreed target" is what the custom sync script will write and what this app will read. Key names under **Proposed** follow bd's existing `ado.<snake_case>` convention so a later beads PR needs no rename; they are not yet written by any tool.

| Azure field | Type | In beads today | Agreed target | Status |
|---|---|---|---|---|
| Scheduling.OriginalEstimate | hours | none | first-class `estimated_minutes` = round(hours × 60) | **Agreed** |
| Scheduling.CompletedWork | hours | none | `metadata["ado.completed_work"]` (hours) | **Agreed** (the main missing item) |
| Scheduling.RemainingWork | hours | `metadata["ado.remaining_work"]` | unchanged | as bd today |
| Scheduling.StartDate | dateTime | none | `metadata["ado.start_date"]` (ISO string as returned) | **Agreed** that a start date must exist; key name **proposed** |
| Scheduling.FinishDate (Story, Task) | dateTime | none | `metadata["ado.finish_date"]` | **Proposed** |
| Scheduling.TargetDate (Epic, Feature) | dateTime | none | `metadata["ado.target_date"]` | **Proposed** |
| Scheduling.DueDate | dateTime | none | out of scope: not on any RadiantSource type. If ever present, first-class `due_at` | n/a |
| Scheduling.StoryPoints | double | `metadata["ado.story_points"]` | unchanged | as bd today |
| Scheduling.Effort (Epic, Feature) | double | none | `metadata["ado.effort"]` | **Proposed**, low priority |
| Scheduling.Size | double | none | out of scope: not on any RadiantSource type | n/a |
| System.AreaPath | treePath | `metadata["ado.area_path"]` | unchanged | as bd today |
| System.IterationPath | treePath | `metadata["ado.iteration_path"]` | unchanged | as bd today |
| Common.Severity | string | `metadata["ado.severity"]` | unchanged | as bd today |
| System.Rev | int | `metadata["ado.rev"]` | unchanged | as bd today |
| Common.Priority | 1–4 | `priority` + `metadata["beads_priority"]` | unchanged | as bd today |
| System.State | string | `status` | unchanged | as bd today |
| System.WorkItemType | string | `issue_type` | unchanged | as bd today |
| System.Parent, link relations | links | `dependencies` (parent-child, blocks, related) | unchanged | as bd today |
| System.AssignedTo | identity | `owner` | unchanged | as bd today |
| System.Tags | text | `labels` | unchanged | as bd today |
| Common.ActivatedDate | dateTime | none (`started_at` is beads' own actual) | not mapped; optional later as `metadata["ado.activated_date"]` for planned-vs-actual | open |
| Common.ClosedDate | dateTime | `closed_at` is set by beads on close, not from Azure | not mapped | open |
| Percent complete | n/a | no Azure field on these types | derived in the app: completed / (completed + remaining) | derived |

### Units

Azure effort fields are hours as doubles. `estimated_minutes` is an integer in minutes: `round(OriginalEstimate * 60)`. `ado.completed_work` and `ado.remaining_work` stay in hours so the derived percent uses one unit.

### Dates

Store the ISO string Azure returns (UTC). Display date-only. Planned end for an item = FinishDate if the type has one, else TargetDate. Planned start = StartDate. Actual start/end = beads `started_at` / `closed_at`.

---

## 4. What a Gantt needs, and where it comes from

| Need | Source |
|---|---|
| Bar start | `ado.start_date` |
| Bar end | `ado.finish_date` (Story, Task) or `ado.target_date` (Epic, Feature) |
| Bar fill / progress | `ado.completed_work` / (`ado.completed_work` + `ado.remaining_work`); fallback `estimated_minutes` vs completed |
| Row grouping | `parent-child` dependencies (Epic → Feature → Story → Task) |
| Arrows | `blocks` dependencies |
| Swimlane / timebox fallback | `ado.iteration_path` when an item has no dates (Bugs) |
| Actual overlay | `started_at`, `closed_at` |

---

## 5. Plan

1. **Custom sync script (cron), Azure → beads.** Reads work items via the Azure REST API or `az boards`, writes with `bd update <id> --metadata '{...}'` (merge) and `bd update <id> --estimate <minutes>` for the estimate. Pull-only for the effort trio: nothing in this plan writes CompletedWork or RemainingWork back to Azure.
2. **This app** renders the keys above in the Custom Fields panel (PR #26 stack) and, later, a Gantt view.
3. **Upstream to beads once proven.** Items that require a beads PR:
   - `buildMetadata`: add `ado.start_date`, `ado.finish_date`, `ado.target_date`, `ado.completed_work`, `ado.original_estimate`, `ado.effort`.
   - `restoreMetadata`: decide which of these round-trip on push (dates yes; effort trio probably pull-only).
   - Map OriginalEstimate → `estimated_minutes` (hours × 60) in `IssueToBeads`.
   - Fix the `tracker.go` / `mapping.go` drift so both paths use `buildMetadata`.
   - Update the "Metadata Preserved" table in bd's `docs/integrations/azure-devops.md`.
   - A local, unpushed branch `feat/ado-scheduling-metadata` in `../beads` already implements the metadata part of this list with tests; it is a draft for that PR, not something in use.

---

## 6. Open questions

- Confirm the proposed key names (`ado.start_date`, `ado.finish_date`, `ado.target_date`, `ado.completed_work`, `ado.effort`) before the sync script writes them.
- Should `ado.original_estimate` (raw hours) also be kept in metadata alongside `estimated_minutes`, to avoid the rounding loss?
- Do we want ActivatedDate / ClosedDate from Azure for planned-vs-actual, or rely on beads' own `started_at` / `closed_at`?
- Conflict policy when both sides edit a synced field between runs (`ado.rev` can detect the Azure side).
