/**
 * Typed, lenient view over the `ado.*` keys in issue metadata.
 *
 * Covers every key bd's Azure DevOps sync writes today (see ../beads
 * internal/ado/mapping.go) plus the scheduling keys documented in
 * docs/azure-devops-mapping.md for the custom sync. All values are returned
 * as strings; a missing or null key is the empty string. Nothing here throws,
 * warns, or logs — ADO data is optional and absence is normal.
 */

import type { IssueMetadata } from '~/types/issue'

export interface AdoMetadata {
  /** True when at least one ado.* key (or beads_priority) is present. UI should hide ADO when false. */
  present: boolean
  // --- written by bd ado sync today ---
  areaPath: string
  iterationPath: string
  storyPoints: string
  remainingWork: string
  severity: string
  rev: string
  /** Original beads priority kept by bd because Azure's 1-4 scale is lossy. Not an ado.* key. */
  beadsPriority: string
  // --- scheduling keys from docs/azure-devops-mapping.md (custom sync) ---
  originalEstimate: string
  completedWork: string
  effort: string
  startDate: string
  finishDate: string
  targetDate: string
}

/** Metadata key for each AdoMetadata field. Single source of truth for the mapping. */
export const ADO_KEYS: Record<Exclude<keyof AdoMetadata, 'present'>, string> = {
  areaPath: 'ado.area_path',
  iterationPath: 'ado.iteration_path',
  storyPoints: 'ado.story_points',
  remainingWork: 'ado.remaining_work',
  severity: 'ado.severity',
  rev: 'ado.rev',
  beadsPriority: 'beads_priority',
  originalEstimate: 'ado.original_estimate',
  completedWork: 'ado.completed_work',
  effort: 'ado.effort',
  startDate: 'ado.start_date',
  finishDate: 'ado.finish_date',
  targetDate: 'ado.target_date',
}

/** Scalar → string; null/undefined → ''. Objects/arrays are not expected for these keys but are JSON-encoded rather than dropped. */
function asString(v: unknown): string {
  if (v === null || v === undefined) return ''
  if (typeof v === 'string') return v
  if (typeof v === 'number' || typeof v === 'boolean') return String(v)
  try {
    return JSON.stringify(v)
  } catch {
    return ''
  }
}

const EMPTY: AdoMetadata = {
  present: false,
  areaPath: '',
  iterationPath: '',
  storyPoints: '',
  remainingWork: '',
  severity: '',
  rev: '',
  beadsPriority: '',
  originalEstimate: '',
  completedWork: '',
  effort: '',
  startDate: '',
  finishDate: '',
  targetDate: '',
}

/**
 * Build the ADO view. Never throws. `present` is true when any mapped key exists
 * in the metadata object, even if its value is empty.
 */
export function parseAdoMetadata(meta: IssueMetadata | null | undefined): AdoMetadata {
  if (!meta || typeof meta !== 'object') return { ...EMPTY }
  const out: AdoMetadata = { ...EMPTY }
  let present = false
  for (const field of Object.keys(ADO_KEYS) as Array<keyof typeof ADO_KEYS>) {
    const key = ADO_KEYS[field]
    if (key in meta) {
      present = true
      out[field] = asString(meta[key])
    }
  }
  out.present = present
  return out
}

/** Planned end date per the mapping doc: FinishDate (Story, Task) wins over TargetDate (Epic, Feature). */
export function adoPlannedEnd(ado: AdoMetadata): string {
  return ado.finishDate || ado.targetDate
}
