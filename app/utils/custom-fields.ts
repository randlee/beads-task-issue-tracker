/**
 * Pure helpers for rendering issue `metadata` as custom fields.
 *
 * Three layers, in priority order:
 *   1. KNOWN_FIELDS — keys this app renders with a dedicated widget
 *      (the example schema from #3, bd's Azure DevOps `ado.*` keys,
 *      bd's execution hints, `beads_priority`)
 *   2. Namespace groups — any `prefix.` or `prefix_` we recognise gets a heading
 *   3. Generic key/value fallback for everything else
 *
 * bd guarantees only that metadata is valid JSON; keys match
 * ^[a-zA-Z_][a-zA-Z0-9_./]*$ and `bd:` / `_` prefixes are reserved.
 * No Vue/Tauri imports here so this stays unit-testable.
 */

import type { IssueMetadata } from '~/types/issue'

export type FieldKind =
  | 'badge'        // short enum-like string rendered as a coloured pill
  | 'wikilinks'    // string or string[] of [[Note]] refs, each copyable
  | 'copyable-id'  // monospace id with copy button
  | 'effort'       // numeric effort (hours); also feeds the effort summary
  | 'progress'     // 0-100 percent; also feeds the effort summary
  | 'date'         // YYYY-MM-DD or ISO date; also feeds the schedule summary
  | 'severity'     // ADO "1 - Critical" .. "4 - Low"
  | 'number'
  | 'text'
  | 'generic'      // anything else: primitives as text, objects as compact JSON

export interface FieldDef {
  key: string
  label: string
  kind: FieldKind
  group: string
  /** badge: value → tailwind classes */
  tones?: Record<string, string>
  /** effort/number: unit suffix */
  unit?: string
}

export interface FieldGroup {
  id: string
  label: string
  /** true for recognised namespaces (heading always shown) */
  namespaced: boolean
}

export interface ResolvedField {
  key: string
  def: FieldDef
  value: unknown
}

export interface RenderedGroup extends FieldGroup {
  fields: ResolvedField[]
}

export interface EffortSummary {
  original: number | null
  remaining: number | null
  completed: number | null
  /** 0-100, from percent_complete or derived from completed/(completed+remaining) */
  percent: number | null
  unit: string
}

export interface ScheduleSummary {
  start: string | null
  end: string | null
  /** whole days from start to end inclusive of start day, null if either missing/invalid */
  durationDays: number | null
  /** true when end < start */
  inverted: boolean
}

export interface WikiLink {
  raw: string
  target: string
  alias: string | null
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

export const FIELD_GROUPS: FieldGroup[] = [
  { id: 'ado', label: 'Azure DevOps', namespaced: true },
  { id: 'execution', label: 'Execution hints', namespaced: true },
  { id: 'custom', label: 'Custom', namespaced: false },
]

const GROUP_PREFIXES: Array<{ prefix: string; group: string }> = [
  { prefix: 'ado.', group: 'ado' },
  { prefix: 'execution_', group: 'execution' },
]

export const PROJECT_TONES: Record<string, string> = {
  iron: 'bg-zinc-500/15 text-zinc-700 dark:text-zinc-300 border-zinc-500/30',
  bronze: 'bg-amber-700/15 text-amber-800 dark:text-amber-300 border-amber-700/30',
  silver: 'bg-slate-400/15 text-slate-700 dark:text-slate-200 border-slate-400/30',
  gold: 'bg-yellow-500/15 text-yellow-800 dark:text-yellow-300 border-yellow-500/30',
  platinum: 'bg-sky-400/15 text-sky-800 dark:text-sky-200 border-sky-400/30',
}

export const SEVERITY_TONES: Record<string, string> = {
  '1': 'bg-red-500/15 text-red-700 dark:text-red-300 border-red-500/30',
  '2': 'bg-orange-500/15 text-orange-700 dark:text-orange-300 border-orange-500/30',
  '3': 'bg-yellow-500/15 text-yellow-800 dark:text-yellow-300 border-yellow-500/30',
  '4': 'bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border-emerald-500/30',
}

export const NEUTRAL_TONE = 'bg-muted text-foreground border-border'

export const KNOWN_FIELDS: Record<string, FieldDef> = {
  // --- example schema from #3 ---
  project: { key: 'project', label: 'Project', kind: 'badge', group: 'custom', tones: PROJECT_TONES },
  refs: { key: 'refs', label: 'References', kind: 'wikilinks', group: 'custom' },
  source_task_id: { key: 'source_task_id', label: 'Source task', kind: 'copyable-id', group: 'custom' },
  original_estimate: { key: 'original_estimate', label: 'Original', kind: 'effort', group: 'custom', unit: 'h' },
  remaining_work: { key: 'remaining_work', label: 'Remaining', kind: 'effort', group: 'custom', unit: 'h' },
  completed_work: { key: 'completed_work', label: 'Completed', kind: 'effort', group: 'custom', unit: 'h' },
  percent_complete: { key: 'percent_complete', label: '% Done', kind: 'progress', group: 'custom' },
  start_date: { key: 'start_date', label: 'Start', kind: 'date', group: 'custom' },
  end_date: { key: 'end_date', label: 'End', kind: 'date', group: 'custom' },
  // --- bd: Azure DevOps sync (internal/ado/mapping.go) ---
  'ado.area_path': { key: 'ado.area_path', label: 'Area path', kind: 'text', group: 'ado' },
  'ado.iteration_path': { key: 'ado.iteration_path', label: 'Iteration', kind: 'text', group: 'ado' },
  'ado.story_points': { key: 'ado.story_points', label: 'Story points', kind: 'number', group: 'ado' },
  'ado.remaining_work': { key: 'ado.remaining_work', label: 'Remaining work', kind: 'number', group: 'ado', unit: 'h' },
  'ado.severity': { key: 'ado.severity', label: 'Severity', kind: 'severity', group: 'ado' },
  'ado.rev': { key: 'ado.rev', label: 'Revision', kind: 'number', group: 'ado' },
  beads_priority: { key: 'beads_priority', label: 'Original priority', kind: 'number', group: 'ado' },
  // --- bd: execution hints (docs/core-concepts/metadata.md) ---
  execution_agent_type: { key: 'execution_agent_type', label: 'Agent type', kind: 'text', group: 'execution' },
  execution_suggested_model: { key: 'execution_suggested_model', label: 'Suggested model', kind: 'text', group: 'execution' },
  execution_reasoning_effort: { key: 'execution_reasoning_effort', label: 'Reasoning effort', kind: 'badge', group: 'execution' },
  execution_mode: { key: 'execution_mode', label: 'Mode', kind: 'badge', group: 'execution' },
  execution_parallel_group: { key: 'execution_parallel_group', label: 'Parallel group', kind: 'text', group: 'execution' },
}

/** Keys folded into the effort/schedule summaries and therefore hidden from the field list. */
const SUMMARY_KEYS = new Set([
  'original_estimate', 'remaining_work', 'completed_work', 'percent_complete', 'start_date', 'end_date',
])

// ---------------------------------------------------------------------------
// Key helpers
// ---------------------------------------------------------------------------

/** "ado.area_path" → "Area path"; "source_task_id" → "Source task id"; "camelCase" → "Camel case". */
export function humanizeKey(key: string): string {
  const last = key.split(/[./]/).pop() ?? key
  const words = last
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .split(/[_\-\s]+/)
    .filter(Boolean)
  if (words.length === 0) return key
  const text = words.join(' ').toLowerCase()
  return text.charAt(0).toUpperCase() + text.slice(1)
}

/** Group id for a key: registry first, then namespace prefix, else 'custom'. */
export function groupForKey(key: string): string {
  const known = KNOWN_FIELDS[key]
  if (known) return known.group
  const hit = GROUP_PREFIXES.find(p => key.startsWith(p.prefix))
  return hit ? hit.group : 'custom'
}

/** Registry entry or a generic definition with an inferred label and group. */
export function resolveFieldDef(key: string, value: unknown): FieldDef {
  const known = KNOWN_FIELDS[key]
  if (known) return known
  let kind: FieldKind = 'generic'
  if (typeof value === 'number') kind = 'number'
  else if (typeof value === 'string') kind = 'text'
  return { key, label: humanizeKey(key), kind, group: groupForKey(key) }
}

// ---------------------------------------------------------------------------
// Value helpers
// ---------------------------------------------------------------------------

/** Accept numbers and numeric strings; anything else → null. */
export function coerceNumber(v: unknown): number | null {
  if (typeof v === 'number') return Number.isFinite(v) ? v : null
  if (typeof v === 'string' && v.trim() !== '') {
    const n = Number(v)
    return Number.isFinite(n) ? n : null
  }
  return null
}

export function clampPercent(v: unknown): number | null {
  const n = coerceNumber(v)
  if (n === null) return null
  return Math.min(100, Math.max(0, Math.round(n)))
}

/** Effort values in hours with a compact suffix. */
export function formatEffort(v: unknown, unit = 'h'): string {
  const n = coerceNumber(v)
  if (n === null) return '—'
  const text = Number.isInteger(n) ? String(n) : n.toFixed(1).replace(/\.0$/, '')
  return `${text} ${unit}`.trim()
}

/** Parse YYYY-MM-DD or ISO 8601 into a UTC-midnight Date; null when invalid. */
export function parseDateOnly(v: unknown): Date | null {
  if (typeof v !== 'string') return null
  const s = v.trim()
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(s)
  if (!m) return null
  const d = new Date(Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3])))
  // Reject overflow like 2026-02-31
  if (d.getUTCFullYear() !== Number(m[1]) || d.getUTCMonth() !== Number(m[2]) - 1 || d.getUTCDate() !== Number(m[3])) return null
  return d
}

/** Locale-agnostic YYYY-MM-DD for display; the raw string when unparsable. */
export function formatDateOnly(v: unknown): string {
  const d = parseDateOnly(v)
  if (!d) return typeof v === 'string' && v.trim() ? v : '—'
  return d.toISOString().slice(0, 10)
}

/** Whole days between two date-only values (end - start). */
export function daysBetween(start: unknown, end: unknown): number | null {
  const a = parseDateOnly(start)
  const b = parseDateOnly(end)
  if (!a || !b) return null
  return Math.round((b.getTime() - a.getTime()) / 86_400_000)
}

/** "[[Note]]" → target "Note"; "[[Note|Alias]]" → alias; plain strings pass through. */
export function parseWikiLink(raw: string): WikiLink {
  const m = /^\s*\[\[([^\]|]+?)(?:\|([^\]]+?))?\]\]\s*$/.exec(raw)
  if (!m) return { raw, target: raw.trim(), alias: null }
  return { raw, target: m[1]!.trim(), alias: m[2]?.trim() ?? null }
}

/** string → [string]; string[] → filtered strings; anything else → []. */
export function toStringList(v: unknown): string[] {
  if (typeof v === 'string') return v.trim() ? [v] : []
  if (Array.isArray(v)) return v.filter((x): x is string => typeof x === 'string' && x.trim() !== '')
  return []
}

/** Generic display: primitives as text, null as a dash, objects/arrays as compact JSON, capped. */
export function formatGenericValue(v: unknown, maxLength = 200): string {
  let text: string
  if (v === null || v === undefined) text = '—'
  else if (typeof v === 'string') text = v
  else if (typeof v === 'number' || typeof v === 'boolean') text = String(v)
  else {
    try {
      text = JSON.stringify(v)
    } catch {
      text = String(v)
    }
  }
  return text.length > maxLength ? `${text.slice(0, maxLength - 1)}…` : text
}

/** "2 - High" → SEVERITY_TONES['2']; also accepts bare "2" or "high". */
export function severityTone(v: unknown): string {
  if (typeof v !== 'string') return NEUTRAL_TONE
  const digit = /^\s*([1-4])/.exec(v)?.[1]
  if (digit) return SEVERITY_TONES[digit] ?? NEUTRAL_TONE
  const word = v.trim().toLowerCase()
  const byWord: Record<string, string> = { critical: '1', high: '2', medium: '3', low: '4' }
  return SEVERITY_TONES[byWord[word] ?? ''] ?? NEUTRAL_TONE
}

/** Badge classes for a value against a def's tones (case-insensitive), neutral otherwise. */
export function badgeTone(def: FieldDef, v: unknown): string {
  if (typeof v !== 'string' || !def.tones) return NEUTRAL_TONE
  return def.tones[v.trim().toLowerCase()] ?? NEUTRAL_TONE
}

// ---------------------------------------------------------------------------
// Summaries
// ---------------------------------------------------------------------------

export function buildEffortSummary(meta: IssueMetadata): EffortSummary | null {
  const original = coerceNumber(meta.original_estimate)
  const remaining = coerceNumber(meta.remaining_work)
  const completed = coerceNumber(meta.completed_work)
  let percent = clampPercent(meta.percent_complete)
  if (percent === null && completed !== null && remaining !== null && completed + remaining > 0) {
    percent = clampPercent((completed / (completed + remaining)) * 100)
  }
  if (original === null && remaining === null && completed === null && percent === null) return null
  return { original, remaining, completed, percent, unit: 'h' }
}

export function buildScheduleSummary(meta: IssueMetadata): ScheduleSummary | null {
  const hasStart = 'start_date' in meta && meta.start_date != null && meta.start_date !== ''
  const hasEnd = 'end_date' in meta && meta.end_date != null && meta.end_date !== ''
  if (!hasStart && !hasEnd) return null
  const start = hasStart ? formatDateOnly(meta.start_date) : null
  const end = hasEnd ? formatDateOnly(meta.end_date) : null
  const diff = daysBetween(meta.start_date, meta.end_date)
  return {
    start,
    end,
    durationDays: diff === null ? null : Math.abs(diff) + 1,
    inverted: diff !== null && diff < 0,
  }
}

// ---------------------------------------------------------------------------
// Partition
// ---------------------------------------------------------------------------

export interface PartitionedMetadata {
  effort: EffortSummary | null
  schedule: ScheduleSummary | null
  groups: RenderedGroup[]
  /** total keys in the metadata object (for the section header count) */
  count: number
}

/**
 * Split metadata into summaries and grouped fields. Summary keys are hidden from
 * the field list when a summary is produced; groups keep FIELD_GROUPS order and
 * fields keep registry order first, then alphabetical.
 */
export function partitionMetadata(meta: IssueMetadata): PartitionedMetadata {
  const effort = buildEffortSummary(meta)
  const schedule = buildScheduleSummary(meta)
  const keys = Object.keys(meta)

  const registryOrder = Object.keys(KNOWN_FIELDS)
  const sortedKeys = [...keys].sort((a, b) => {
    const ia = registryOrder.indexOf(a)
    const ib = registryOrder.indexOf(b)
    if (ia !== -1 && ib !== -1) return ia - ib
    if (ia !== -1) return -1
    if (ib !== -1) return 1
    return a.localeCompare(b)
  })

  const byGroup = new Map<string, ResolvedField[]>()
  for (const key of sortedKeys) {
    if (SUMMARY_KEYS.has(key) && (effort || schedule)) continue
    const value = meta[key]
    const def = resolveFieldDef(key, value)
    const list = byGroup.get(def.group) ?? []
    list.push({ key, def, value })
    byGroup.set(def.group, list)
  }

  const groups: RenderedGroup[] = FIELD_GROUPS
    .filter(g => (byGroup.get(g.id)?.length ?? 0) > 0)
    .map(g => ({ ...g, fields: byGroup.get(g.id)! }))

  return { effort, schedule, groups, count: keys.length }
}
