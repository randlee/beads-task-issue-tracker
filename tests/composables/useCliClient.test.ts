import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { ref, computed } from 'vue'

// The composable relies on Nuxt auto-imports (ref, computed). Stub them as
// globals and load the module via dynamic import after vi.resetModules() so
// each test starts with fresh singleton state.
const mocks = vi.hoisted(() => ({
  getCliBinaryPath: vi.fn(),
}))

vi.mock('~/utils/bd-api', () => ({
  getCliBinaryPath: mocks.getCliBinaryPath,
}))

type UseCliClient = typeof import('~/composables/useCliClient')['useCliClient']

async function loadComposable(): Promise<UseCliClient> {
  const mod = await import('~/composables/useCliClient')
  return mod.useCliClient
}

describe('useCliClient', () => {
  beforeEach(() => {
    vi.resetModules()
    mocks.getCliBinaryPath.mockReset()
    vi.stubGlobal('ref', ref)
    vi.stubGlobal('computed', computed)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('defaults to bd before init()', async () => {
    const useCliClient = await loadComposable()
    const { cliBinary, isBr } = useCliClient()

    expect(cliBinary.value).toBe('bd')
    expect(isBr.value).toBe(false)
    expect(mocks.getCliBinaryPath).not.toHaveBeenCalled()
  })

  it('init() sets br when the configured path is exactly "br"', async () => {
    mocks.getCliBinaryPath.mockResolvedValue('br')
    const useCliClient = await loadComposable()
    const { cliBinary, isBr, init } = useCliClient()

    await init()

    expect(cliBinary.value).toBe('br')
    expect(isBr.value).toBe(true)
  })

  it.each([
    'bd',
    '/opt/homebrew/bin/bd',
    '/usr/local/bin/br', // full path is not the bare "br" name
    'brew',
    '',
  ])('init() sets bd for path %j', async (path) => {
    mocks.getCliBinaryPath.mockResolvedValue(path)
    const useCliClient = await loadComposable()
    const { cliBinary, isBr, init } = useCliClient()

    await init()

    expect(cliBinary.value).toBe('bd')
    expect(isBr.value).toBe(false)
  })

  it('init() falls back to bd when getCliBinaryPath throws', async () => {
    mocks.getCliBinaryPath.mockRejectedValue(new Error('not in tauri'))
    const useCliClient = await loadComposable()
    const { cliBinary, isBr, init, setBinary } = useCliClient()

    // Start from br to prove init() actively resets on failure
    setBinary('br')
    expect(isBr.value).toBe(true)

    await expect(init()).resolves.toBeUndefined()

    expect(cliBinary.value).toBe('bd')
    expect(isBr.value).toBe(false)
  })

  it('setBinary updates cliBinary and isBr', async () => {
    const useCliClient = await loadComposable()
    const { cliBinary, isBr, setBinary } = useCliClient()

    setBinary('br')
    expect(cliBinary.value).toBe('br')
    expect(isBr.value).toBe(true)

    setBinary('bd')
    expect(cliBinary.value).toBe('bd')
    expect(isBr.value).toBe(false)
  })

  it('shares singleton state between callers', async () => {
    const useCliClient = await loadComposable()
    const a = useCliClient()
    const b = useCliClient()

    a.setBinary('br')

    expect(b.cliBinary.value).toBe('br')
    expect(b.isBr.value).toBe(true)
  })
})
