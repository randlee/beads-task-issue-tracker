/**
 * Pure helpers for issue `metadata` (custom fields).
 * bd emits metadata as a JSON object; older data may arrive as a JSON string.
 * Keep free of Vue/Tauri imports so this stays unit-testable.
 */

import type { IssueMetadata } from '~/types/issue'

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v)
}

/**
 * Coerce whatever the backend sent into a metadata object, or null when there
 * is nothing usable (absent, null, empty object, blank string, non-object JSON).
 */
export function normalizeMetadata(raw: unknown): IssueMetadata | null {
  if (raw == null) return null
  if (isPlainObject(raw)) {
    return Object.keys(raw).length > 0 ? raw : null
  }
  if (typeof raw === 'string') {
    const trimmed = raw.trim()
    if (!trimmed) return null
    try {
      const parsed: unknown = JSON.parse(trimmed)
      return isPlainObject(parsed) && Object.keys(parsed).length > 0 ? parsed : null
    } catch {
      return null
    }
  }
  return null
}

/** True when the issue has at least one custom field to show. */
export function hasMetadata(raw: unknown): boolean {
  return normalizeMetadata(raw) !== null
}

/**
 * Pretty JSON for the raw view. Objects are re-serialized; strings that hold
 * JSON are re-indented; anything else is shown as-is.
 */
export function formatMetadataJson(raw: unknown): string {
  if (raw == null) return ''
  if (typeof raw === 'string') {
    try {
      return JSON.stringify(JSON.parse(raw), null, 2)
    } catch {
      return raw
    }
  }
  try {
    return JSON.stringify(raw, null, 2)
  } catch {
    return String(raw)
  }
}
