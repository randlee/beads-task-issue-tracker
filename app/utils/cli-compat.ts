/**
 * Pure helpers for presenting CLI compatibility state (bd / br detection).
 * Mirrors the Rust `check_bd_compatibility` payload; keep free of Vue/Tauri
 * imports so it stays unit-testable.
 */

import type { BdCompatibilityInfo } from '~/utils/bd-api'

/** Mirror of the Rust MIN_SUPPORTED_BD_MAJOR; the backend value wins at runtime. */
export const DEFAULT_MIN_SUPPORTED_BD_MAJOR = 1

export type CliStatusLevel = 'ok' | 'warning' | 'error'

export interface CliStatus {
  level: CliStatusLevel
  title: string
  details: string[]
  /** Only populated for the "not found" error so the UI can show where we looked. */
  searchedPaths: string[]
  /** Errors (CLI missing) cannot be dismissed; warnings can. */
  dismissible: boolean
}

/**
 * True when the client is `bd` and its major version is below the floor,
 * or when it is `bd` and the version could not be parsed.
 */
export function isLegacyBd(
  clientType: string,
  versionTuple: number[] | null | undefined,
  minMajor: number = DEFAULT_MIN_SUPPORTED_BD_MAJOR,
): boolean {
  if (clientType !== 'bd') return false
  if (!versionTuple || versionTuple.length === 0) return true
  return versionTuple[0]! < minMajor
}

/**
 * Stable identity for a detected CLI so a dismissal only applies to that
 * exact binary + version and reappears after an upgrade or switch.
 */
export function compatSignature(info: Pick<BdCompatibilityInfo, 'binary' | 'clientType' | 'version'>): string {
  return `${info.binary}|${info.clientType}|${info.version}`
}

/** Collapse a long searched-path list for display. */
export function formatSearchedPaths(paths: string[], max = 6): string[] {
  const cleaned = paths.map(p => p.trim()).filter(Boolean)
  if (cleaned.length <= max) return cleaned
  return [...cleaned.slice(0, max), `… and ${cleaned.length - max} more`]
}

/** Derive what the banner should show from the backend payload. */
export function buildCliStatus(info: BdCompatibilityInfo): CliStatus {
  if (!info.found) {
    return {
      level: 'error',
      title: `${info.binary} was not found`,
      details: info.warnings.length
        ? info.warnings
        : [`Install bd ${info.minSupportedMajor}.x or choose a binary in Settings.`],
      searchedPaths: formatSearchedPaths(info.searchedPaths ?? []),
      dismissible: false,
    }
  }

  const legacy = info.legacy || isLegacyBd(info.clientType, info.versionTuple, info.minSupportedMajor)
  if (legacy) {
    return {
      level: 'warning',
      title: `Legacy ${info.clientType} detected (${info.version})`,
      details: info.warnings,
      searchedPaths: [],
      dismissible: true,
    }
  }

  if (info.warnings.length > 0) {
    return {
      level: 'warning',
      title: `${info.clientType} compatibility notice`,
      details: info.warnings,
      searchedPaths: [],
      dismissible: true,
    }
  }

  return { level: 'ok', title: '', details: [], searchedPaths: [], dismissible: true }
}

/** Whether the banner should render given the status and the last dismissed signature. */
export function shouldShowCliBanner(status: CliStatus, signature: string, dismissedSignature: string): boolean {
  if (status.level === 'ok') return false
  if (!status.dismissible) return true
  return signature !== dismissedSignature
}
