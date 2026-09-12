import { getCliBinaryPath } from '~/utils/bd-api'

// Singleton state — shared across all callers
// Default matches the backend (CLI_FALLBACK = "bd"); init() corrects it from settings.
const cliBinary = ref<'br' | 'bd'>('bd')

export function useCliClient() {
  const isBr = computed(() => cliBinary.value === 'br')

  async function init() {
    try {
      const path = await getCliBinaryPath()
      cliBinary.value = path === 'br' ? 'br' : 'bd'
    } catch {
      cliBinary.value = 'bd'
    }
  }

  function setBinary(value: 'br' | 'bd') {
    cliBinary.value = value
  }

  return { cliBinary, isBr, init, setBinary }
}
