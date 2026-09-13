import { type Ref, ref, watch, onMounted, onUnmounted } from 'vue'
import type { Issue } from '~/types/issue'
import { useIssues } from '~/composables/useIssues'
import { useIssueHistory } from '~/composables/useIssueHistory'
import { logFrontend } from '~/utils/bd-api'

export type MobilePanel = 'dashboard' | 'issues' | 'details'

export interface UseIssueNavigationOptions {
  isMobileView: Ref<boolean>
  mobilePanel: Ref<MobilePanel>
  isRightSidebarOpen: Ref<boolean>
  isEditMode: Ref<boolean>
  isCreatingNew: Ref<boolean>
}

export interface OpenIssueOptions {
  /** Navigation triggered by back/forward: do not record in history. */
  fromHistory?: boolean
  /** Open directly in edit mode. */
  edit?: boolean
}

function isTypingTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null
  if (!el) return false
  const tag = el.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true
  return !!el.isContentEditable
}

/**
 * Single choke point for opening an issue in the details panel.
 * Every open path (table row, quick list, parent/child/blocker links, history)
 * goes through `openIssue`, which handles epic expansion, immediate selection
 * from the loaded list, panel/sidebar reveal, full fetch, and history recording.
 */
export function useIssueNavigation(options: UseIssueNavigationOptions) {
  const { isMobileView, mobilePanel, isRightSidebarOpen, isEditMode, isCreatingNew } = options
  const { issues, selectIssue, fetchIssue, expandEpic, lastDeletedIssueId } = useIssues()
  const history = useIssueHistory()

  const isNavigating = ref(false)

  const revealDetails = () => {
    if (isMobileView.value) {
      mobilePanel.value = 'details'
    } else {
      isRightSidebarOpen.value = true
    }
  }

  const openIssue = async (target: string | Issue, opts: OpenIssueOptions = {}): Promise<boolean> => {
    const id = typeof target === 'string' ? target : target.id

    // Child issue (parent-id.N): expand the parent epic so the row is visible
    const lastDot = id.lastIndexOf('.')
    if (lastDot > 0) {
      expandEpic(id.slice(0, lastDot))
    }

    // Immediate feedback from the loaded list when available
    const known = typeof target === 'object' ? target : issues.value.find(i => i.id === id)
    if (known) {
      selectIssue(known)
      if (!opts.fromHistory) history.record(known)
    }

    isEditMode.value = !!opts.edit
    isCreatingNew.value = false
    revealDetails()

    // Full details (extended fields, parent, children)
    const data = await fetchIssue(id)
    if (!data) return false
    if (!known && !opts.fromHistory) history.record(data)
    return true
  }

  const leaveEditMode = () => {
    isEditMode.value = false
    isCreatingNew.value = false
  }

  const attemptOpen = async (entry: { id: string }) => {
    const ok = await openIssue(entry.id, { fromHistory: true })
    if (!ok) logFrontend('info', `[history] skipping unavailable issue ${entry.id}`)
    return ok
  }

  const back = async () => {
    if (isNavigating.value || !history.canGoBack.value) return
    isNavigating.value = true
    try {
      leaveEditMode()
      await history.back(attemptOpen)
    } finally {
      isNavigating.value = false
    }
  }

  const forward = async () => {
    if (isNavigating.value || !history.canGoForward.value) return
    isNavigating.value = true
    try {
      leaveEditMode()
      await history.forward(attemptOpen)
    } finally {
      isNavigating.value = false
    }
  }

  /** Open an entry from the MRU dropdown (recorded as a new navigation). */
  const openFromHistory = (id: string) => openIssue(id)

  // Drop deleted issues from history
  watch(lastDeletedIssueId, (id) => {
    if (id) history.remove([id])
  })

  // Alt+ArrowLeft / Alt+ArrowRight — Cmd+[ is deliberately avoided (WebKit back)
  const handleKeydown = (event: KeyboardEvent) => {
    if (!event.altKey || event.metaKey || event.ctrlKey || event.shiftKey) return
    if (isTypingTarget(event.target)) return
    if (event.key === 'ArrowLeft') {
      event.preventDefault()
      void back()
    } else if (event.key === 'ArrowRight') {
      event.preventDefault()
      void forward()
    }
  }

  onMounted(() => window.addEventListener('keydown', handleKeydown))
  onUnmounted(() => window.removeEventListener('keydown', handleKeydown))

  return {
    openIssue,
    openFromHistory,
    back,
    forward,
    isNavigating,
    canGoBack: history.canGoBack,
    canGoForward: history.canGoForward,
    historyEntries: history.mruEntries,
    resetHistory: history.reset,
    handleKeydown,
  }
}
