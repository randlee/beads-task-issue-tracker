import { checkBdCompatibility, logFrontend, type BdCompatibilityInfo } from '~/utils/bd-api'
import { buildCliStatus, compatSignature, shouldShowCliBanner } from '~/utils/cli-compat'

// Singleton state — shared across all callers
const info = ref<BdCompatibilityInfo | null>(null)
const isChecking = ref(false)
const dismissedSignature = useLocalStorage<string>('beads:cliCompatDismissed', '')

export function useCliCompatibility() {
  const status = computed(() => (info.value ? buildCliStatus(info.value) : null))
  const signature = computed(() => (info.value ? compatSignature(info.value) : ''))
  const isVisible = computed(() =>
    status.value ? shouldShowCliBanner(status.value, signature.value, dismissedSignature.value) : false,
  )

  /** Re-run the backend check (startup, and after the CLI binary changes in Settings). */
  async function refresh(): Promise<BdCompatibilityInfo | null> {
    isChecking.value = true
    try {
      const result = await checkBdCompatibility()
      info.value = result
      if (!result.found) {
        logFrontend('error', `[cli-compat] ${result.binary} not found; searched ${result.searchedPaths.length} dirs`)
      } else if (result.warnings.length > 0) {
        logFrontend('warn', `[cli-compat] ${result.clientType} ${result.version}: ${result.warnings.join(' | ')}`)
      } else {
        logFrontend('info', `[cli-compat] ${result.clientType} ${result.version} OK`)
      }
      return result
    } catch (error) {
      logFrontend('error', `[cli-compat] check failed: ${error instanceof Error ? error.message : String(error)}`)
      return null
    } finally {
      isChecking.value = false
    }
  }

  /** Hide the current warning until the detected binary/version changes. */
  function dismiss() {
    if (status.value?.dismissible) {
      dismissedSignature.value = signature.value
    }
  }

  return { info, status, isVisible, isChecking, refresh, dismiss }
}
