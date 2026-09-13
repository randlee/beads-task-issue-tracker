import { describe, it, expect } from 'vitest'
import {
  isLegacyBd,
  compatSignature,
  formatSearchedPaths,
  buildCliStatus,
  shouldShowCliBanner,
  DEFAULT_MIN_SUPPORTED_BD_MAJOR,
} from '~/utils/cli-compat'
import type { BdCompatibilityInfo } from '~/utils/bd-api'

function makeInfo(overrides: Partial<BdCompatibilityInfo> = {}): BdCompatibilityInfo {
  return {
    binary: 'bd',
    found: true,
    version: 'bd version 1.0.4 (ce242a879)',
    clientType: 'bd',
    versionTuple: [1, 0, 4],
    legacy: false,
    minSupportedMajor: 1,
    supportsDaemonFlag: false,
    usesJsonlFiles: false,
    usesDoltBackend: true,
    supportsListAllFlag: true,
    searchedPaths: ['/opt/homebrew/bin', '/usr/local/bin'],
    warnings: [],
    ...overrides,
  }
}

describe('isLegacyBd', () => {
  it('flags bd below the floor', () => {
    expect(isLegacyBd('bd', [0, 49, 6])).toBe(true)
    expect(isLegacyBd('bd', [0, 56, 0])).toBe(true)
  })
  it('accepts bd at or above the floor', () => {
    expect(isLegacyBd('bd', [1, 0, 4])).toBe(false)
    expect(isLegacyBd('bd', [2, 0, 0])).toBe(false)
  })
  it('treats unparsable bd version as legacy', () => {
    expect(isLegacyBd('bd', null)).toBe(true)
    expect(isLegacyBd('bd', [])).toBe(true)
  })
  it('never flags br or unknown', () => {
    expect(isLegacyBd('br', [0, 1, 33])).toBe(false)
    expect(isLegacyBd('unknown', null)).toBe(false)
  })
  it('honors a raised floor', () => {
    expect(isLegacyBd('bd', [1, 9, 9], 2)).toBe(true)
    expect(isLegacyBd('bd', [2, 0, 0], 2)).toBe(false)
    expect(DEFAULT_MIN_SUPPORTED_BD_MAJOR).toBe(1)
  })
})

describe('compatSignature', () => {
  it('changes when binary, client, or version changes', () => {
    const a = compatSignature({ binary: 'bd', clientType: 'bd', version: 'bd version 1.0.4' })
    expect(compatSignature({ binary: 'bd', clientType: 'bd', version: 'bd version 1.0.4' })).toBe(a)
    expect(compatSignature({ binary: 'br', clientType: 'br', version: 'br 0.1.33' })).not.toBe(a)
    expect(compatSignature({ binary: 'bd', clientType: 'bd', version: 'bd version 1.0.5' })).not.toBe(a)
  })
})

describe('formatSearchedPaths', () => {
  it('returns short lists unchanged and drops blanks', () => {
    expect(formatSearchedPaths(['/a', ' ', '/b'])).toEqual(['/a', '/b'])
  })
  it('truncates long lists with a count', () => {
    const paths = Array.from({ length: 10 }, (_, i) => `/p${i}`)
    const out = formatSearchedPaths(paths, 4)
    expect(out).toHaveLength(5)
    expect(out[4]).toBe('… and 6 more')
  })
  it('handles Windows-style paths without splitting them', () => {
    const win = ['C:\\Users\\me\\go\\bin', 'C:\\Users\\me\\.cargo\\bin']
    expect(formatSearchedPaths(win)).toEqual(win)
  })
})

describe('buildCliStatus', () => {
  it('is ok for supported bd with no warnings', () => {
    const s = buildCliStatus(makeInfo())
    expect(s.level).toBe('ok')
    expect(s.details).toEqual([])
  })
  it('is a non-dismissible error when the binary is not found', () => {
    const s = buildCliStatus(makeInfo({ found: false, version: 'bd not found', clientType: 'unknown', versionTuple: null, warnings: ['bd was not found on PATH'] }))
    expect(s.level).toBe('error')
    expect(s.dismissible).toBe(false)
    expect(s.title).toContain('bd')
    expect(s.searchedPaths).toEqual(['/opt/homebrew/bin', '/usr/local/bin'])
    expect(s.details[0]).toContain('not found')
  })
  it('falls back to an install hint when not found and backend sent no warnings', () => {
    const s = buildCliStatus(makeInfo({ found: false, warnings: [] }))
    expect(s.details[0]).toContain('Install bd 1.x')
  })
  it('is a dismissible warning for legacy bd', () => {
    const s = buildCliStatus(makeInfo({ version: 'bd version 0.49.6', versionTuple: [0, 49, 6], legacy: true, warnings: ['bd 0.49.6 is a legacy version'] }))
    expect(s.level).toBe('warning')
    expect(s.dismissible).toBe(true)
    expect(s.title).toContain('Legacy')
    expect(s.details).toEqual(['bd 0.49.6 is a legacy version'])
  })
  it('derives legacy client-side when the backend flag is missing', () => {
    const s = buildCliStatus(makeInfo({ versionTuple: [0, 49, 6], legacy: false }))
    expect(s.level).toBe('warning')
  })
  it('is a dismissible warning for br', () => {
    const s = buildCliStatus(makeInfo({ binary: 'br', clientType: 'br', version: 'br 0.1.33', versionTuple: [0, 1, 33], warnings: ['br detected: secondary CLI'] }))
    expect(s.level).toBe('warning')
    expect(s.title).toContain('br')
    expect(s.searchedPaths).toEqual([])
  })
})

describe('shouldShowCliBanner', () => {
  const ok = buildCliStatus(makeInfo())
  const missing = buildCliStatus(makeInfo({ found: false }))
  const legacyInfo = makeInfo({ versionTuple: [0, 49, 6], legacy: true, warnings: ['legacy'] })
  const legacy = buildCliStatus(legacyInfo)
  const sig = compatSignature(legacyInfo)

  it('hides when ok', () => {
    expect(shouldShowCliBanner(ok, 'x', '')).toBe(false)
  })
  it('always shows a not-found error, even if "dismissed"', () => {
    expect(shouldShowCliBanner(missing, 'x', 'x')).toBe(true)
  })
  it('shows a warning until its exact signature is dismissed', () => {
    expect(shouldShowCliBanner(legacy, sig, '')).toBe(true)
    expect(shouldShowCliBanner(legacy, sig, sig)).toBe(false)
    expect(shouldShowCliBanner(legacy, sig, 'bd|bd|bd version 1.0.4')).toBe(true)
  })
})
