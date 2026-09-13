import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { ref, computed } from 'vue'
import type { BdCompatibilityInfo } from '~/utils/bd-api'

// The composable relies on Nuxt auto-imports (ref, computed, useLocalStorage),
// which are not available under Vitest. Stub them as globals and load the
// module via dynamic import after vi.resetModules() so each test starts with
// fresh singleton state.
const mocks = vi.hoisted(() => ({
  checkBdCompatibility: vi.fn(),
  logFrontend: vi.fn(),
}))

vi.mock('~/utils/bd-api', () => ({
  checkBdCompatibility: mocks.checkBdCompatibility,
  logFrontend: mocks.logFrontend,
}))

function makeInfo(overrides: Partial<BdCompatibilityInfo> = {}): BdCompatibilityInfo {
  return {
    binary: 'bd',
    found: true,
    version: 'bd version 1.2.0',
    clientType: 'bd',
    versionTuple: [1, 2, 0],
    legacy: false,
    minSupportedMajor: 1,
    supportsDaemonFlag: true,
    usesJsonlFiles: false,
    usesDoltBackend: true,
    supportsListAllFlag: true,
    searchedPaths: [],
    warnings: [],
    ...overrides,
  }
}

const legacyInfo = (): BdCompatibilityInfo =>
  makeInfo({
    version: 'bd version 0.49.5',
    versionTuple: [0, 49, 5],
    legacy: true,
    warnings: ['bd 0.49.5 is below the supported floor (1.x)'],
  })

const notFoundInfo = (): BdCompatibilityInfo =>
  makeInfo({
    found: false,
    version: 'bd not found',
    clientType: 'unknown',
    versionTuple: null,
    searchedPaths: ['/usr/local/bin', '/opt/homebrew/bin'],
    warnings: [],
  })

type UseCliCompatibility = typeof import('~/composables/useCliCompatibility')['useCliCompatibility']

async function loadComposable(): Promise<UseCliCompatibility> {
  const mod = await import('~/composables/useCliCompatibility')
  return mod.useCliCompatibility
}

describe('useCliCompatibility', () => {
  beforeEach(() => {
    vi.resetModules()
    mocks.checkBdCompatibility.mockReset()
    mocks.logFrontend.mockReset()
    localStorage.clear()
    vi.stubGlobal('ref', ref)
    vi.stubGlobal('computed', computed)
    // Minimal stand-in for the app's useLocalStorage: a plain ref, no persistence.
    vi.stubGlobal('useLocalStorage', <T>(_key: string, defaultValue: T) => ref(defaultValue))
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('status and isVisible are null/false before refresh', async () => {
    const useCliCompatibility = await loadComposable()
    const { info, status, isVisible, isChecking } = useCliCompatibility()

    expect(info.value).toBeNull()
    expect(status.value).toBeNull()
    expect(isVisible.value).toBe(false)
    expect(isChecking.value).toBe(false)
  })

  it('refresh() stores the payload, returns it, and derives status', async () => {
    const payload = makeInfo()
    mocks.checkBdCompatibility.mockResolvedValue(payload)
    const useCliCompatibility = await loadComposable()
    const { info, status, isVisible, refresh } = useCliCompatibility()

    const result = await refresh()

    expect(result).toEqual(payload)
    expect(info.value).toEqual(payload)
    expect(status.value?.level).toBe('ok')
    expect(isVisible.value).toBe(false)
  })

  it('logs info when the CLI is found with no warnings', async () => {
    mocks.checkBdCompatibility.mockResolvedValue(makeInfo())
    const useCliCompatibility = await loadComposable()

    await useCliCompatibility().refresh()

    expect(mocks.logFrontend).toHaveBeenCalledTimes(1)
    expect(mocks.logFrontend).toHaveBeenCalledWith('info', expect.stringContaining('bd version 1.2.0 OK'))
  })

  it('logs warn when the CLI is found but has warnings', async () => {
    mocks.checkBdCompatibility.mockResolvedValue(
      makeInfo({ warnings: ['first warning', 'second warning'] }),
    )
    const useCliCompatibility = await loadComposable()

    await useCliCompatibility().refresh()

    expect(mocks.logFrontend).toHaveBeenCalledTimes(1)
    const [level, message] = mocks.logFrontend.mock.calls[0]!
    expect(level).toBe('warn')
    expect(message).toContain('first warning | second warning')
  })

  it('logs error when the CLI is not found, including searched dir count', async () => {
    mocks.checkBdCompatibility.mockResolvedValue(notFoundInfo())
    const useCliCompatibility = await loadComposable()

    await useCliCompatibility().refresh()

    expect(mocks.logFrontend).toHaveBeenCalledTimes(1)
    expect(mocks.logFrontend).toHaveBeenCalledWith('error', expect.stringContaining('searched 2 dirs'))
  })

  it('refresh() returns null, logs error, and leaves info untouched when the check rejects', async () => {
    mocks.checkBdCompatibility.mockRejectedValue(new Error('invoke failed'))
    const useCliCompatibility = await loadComposable()
    const { info, status, isChecking, refresh } = useCliCompatibility()

    const result = await refresh()

    expect(result).toBeNull()
    expect(info.value).toBeNull()
    expect(status.value).toBeNull()
    expect(isChecking.value).toBe(false)
    expect(mocks.logFrontend).toHaveBeenCalledWith('error', expect.stringContaining('invoke failed'))
  })

  it('stringifies non-Error rejections in the log message', async () => {
    mocks.checkBdCompatibility.mockRejectedValue('plain string failure')
    const useCliCompatibility = await loadComposable()

    await useCliCompatibility().refresh()

    expect(mocks.logFrontend).toHaveBeenCalledWith('error', expect.stringContaining('plain string failure'))
  })

  it('isChecking is true while the check is in flight and false afterwards', async () => {
    let resolveCheck: (value: BdCompatibilityInfo) => void = () => {}
    mocks.checkBdCompatibility.mockReturnValue(
      new Promise<BdCompatibilityInfo>((resolve) => {
        resolveCheck = resolve
      }),
    )
    const useCliCompatibility = await loadComposable()
    const { isChecking, refresh } = useCliCompatibility()

    const pending = refresh()
    expect(isChecking.value).toBe(true)

    resolveCheck(makeInfo())
    await pending
    expect(isChecking.value).toBe(false)
  })

  it('isChecking resets to false even when the check rejects', async () => {
    mocks.checkBdCompatibility.mockRejectedValue(new Error('boom'))
    const useCliCompatibility = await loadComposable()
    const { isChecking, refresh } = useCliCompatibility()

    await refresh()

    expect(isChecking.value).toBe(false)
  })

  it('dismiss() hides a dismissible (legacy) warning', async () => {
    mocks.checkBdCompatibility.mockResolvedValue(legacyInfo())
    const useCliCompatibility = await loadComposable()
    const { status, isVisible, refresh, dismiss } = useCliCompatibility()

    await refresh()
    expect(status.value?.level).toBe('warning')
    expect(status.value?.dismissible).toBe(true)
    expect(isVisible.value).toBe(true)

    dismiss()
    expect(isVisible.value).toBe(false)
  })

  it('dismiss() has no effect on a non-dismissible (not found) error', async () => {
    mocks.checkBdCompatibility.mockResolvedValue(notFoundInfo())
    const useCliCompatibility = await loadComposable()
    const { status, isVisible, refresh, dismiss } = useCliCompatibility()

    await refresh()
    expect(status.value?.level).toBe('error')
    expect(status.value?.dismissible).toBe(false)
    expect(isVisible.value).toBe(true)

    dismiss()
    expect(isVisible.value).toBe(true)
  })

  it('dismiss() before any refresh is a no-op', async () => {
    const useCliCompatibility = await loadComposable()
    const { isVisible, dismiss } = useCliCompatibility()

    expect(() => dismiss()).not.toThrow()
    expect(isVisible.value).toBe(false)
  })

  it('a dismissed warning stays hidden when the same version is re-detected', async () => {
    mocks.checkBdCompatibility.mockResolvedValue(legacyInfo())
    const useCliCompatibility = await loadComposable()
    const { isVisible, refresh, dismiss } = useCliCompatibility()

    await refresh()
    dismiss()
    expect(isVisible.value).toBe(false)

    await refresh()
    expect(isVisible.value).toBe(false)
  })

  it('a subsequent refresh with a different version un-dismisses the warning', async () => {
    mocks.checkBdCompatibility.mockResolvedValueOnce(legacyInfo())
    const useCliCompatibility = await loadComposable()
    const { isVisible, refresh, dismiss } = useCliCompatibility()

    await refresh()
    dismiss()
    expect(isVisible.value).toBe(false)

    mocks.checkBdCompatibility.mockResolvedValueOnce(
      makeInfo({
        version: 'bd version 0.50.0',
        versionTuple: [0, 50, 0],
        legacy: true,
        warnings: ['still legacy, different version'],
      }),
    )
    await refresh()
    expect(isVisible.value).toBe(true)
  })

  it('a subsequent refresh with a different binary un-dismisses the warning', async () => {
    mocks.checkBdCompatibility.mockResolvedValueOnce(legacyInfo())
    const useCliCompatibility = await loadComposable()
    const { isVisible, refresh, dismiss } = useCliCompatibility()

    await refresh()
    dismiss()
    expect(isVisible.value).toBe(false)

    mocks.checkBdCompatibility.mockResolvedValueOnce(legacyInfo())
    // Same version, different binary path -> different signature
    mocks.checkBdCompatibility.mockResolvedValueOnce(
      makeInfo({ ...legacyInfo(), binary: '/opt/homebrew/bin/bd' }),
    )
    await refresh() // same signature, still hidden
    expect(isVisible.value).toBe(false)
    await refresh() // new binary, visible again
    expect(isVisible.value).toBe(true)
  })

  it('shares singleton state between callers', async () => {
    mocks.checkBdCompatibility.mockResolvedValue(legacyInfo())
    const useCliCompatibility = await loadComposable()
    const a = useCliCompatibility()
    const b = useCliCompatibility()

    await a.refresh()

    expect(b.info.value).toEqual(a.info.value)
    expect(b.status.value?.level).toBe('warning')
    expect(b.isVisible.value).toBe(true)

    b.dismiss()
    expect(a.isVisible.value).toBe(false)
  })

  it('reads the persisted dismissed signature via useLocalStorage', async () => {
    // Pre-seed the "stored" value the way the real useLocalStorage would.
    vi.stubGlobal(
      'useLocalStorage',
      <T>(_key: string, _defaultValue: T) => ref('bd|bd|bd version 0.49.5' as unknown as T),
    )
    mocks.checkBdCompatibility.mockResolvedValue(legacyInfo())
    const useCliCompatibility = await loadComposable()
    const { isVisible, refresh } = useCliCompatibility()

    await refresh()

    expect(isVisible.value).toBe(false)
  })

  it('uses the expected localStorage key for the dismissed signature', async () => {
    const spy = vi.fn(<T>(_key: string, defaultValue: T) => ref(defaultValue))
    vi.stubGlobal('useLocalStorage', spy)

    await loadComposable()

    expect(spy).toHaveBeenCalledWith('beads:cliCompatDismissed', '')
  })
})
