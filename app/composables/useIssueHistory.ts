import { ref, computed, watch } from 'vue'
import type { Issue } from '~/types/issue'
import { useIssues } from '~/composables/useIssues'
import { useProjectStorage } from '~/composables/useProjectStorage'
import {
  type HistoryEntry,
  type HistoryState,
  resetHistory,
  pushEntry,
  goBack,
  goForward,
  canGoBack as helperCanGoBack,
  canGoForward as helperCanGoForward,
  currentEntry,
  pruneEntries,
  removeEntries,
} from '~/utils/history-helpers'

export type HistoryDirection = 'back' | 'forward'

/**
 * Singleton navigation history for the issue details panel.
 *
 * - Back/forward stack is session-only.
 * - MRU list is persisted per project via useProjectStorage('issueHistoryMru').
 * - Entries hold id + title/status snapshots only; `mruEntries` re-resolves
 *   title/status against the live issues list when the issue is loaded.
 */
const persistedMru = useProjectStorage<HistoryEntry[]>('issueHistoryMru', [])
const state = ref<HistoryState>(resetHistory(persistedMru.value))

// Storage reloads its value on project switch — keep the in-memory MRU aligned.
watch(persistedMru, (mru) => {
  if (mru !== state.value.mru) {
    state.value = { ...state.value, mru: Array.isArray(mru) ? mru : [] }
  }
})

function commit(next: HistoryState) {
  state.value = next
  if (persistedMru.value !== next.mru) {
    persistedMru.value = next.mru
  }
}

function toEntry(issue: Pick<Issue, 'id' | 'title' | 'status'>): HistoryEntry {
  return { id: issue.id, title: issue.title, status: issue.status }
}

export function useIssueHistory() {
  const { issues } = useIssues()

  const canGoBack = computed(() => helperCanGoBack(state.value))
  const canGoForward = computed(() => helperCanGoForward(state.value))
  const currentId = computed(() => currentEntry(state.value)?.id ?? null)

  const mruEntries = computed<HistoryEntry[]>(() => {
    const live = new Map(issues.value.map(i => [i.id, i]))
    return state.value.mru.map((entry) => {
      const issue = live.get(entry.id)
      return issue ? toEntry(issue) : entry
    })
  })

  /** Record an opened issue (call on every non-history open). */
  const record = (issue: Pick<Issue, 'id' | 'title' | 'status'>) => {
    commit(pushEntry(state.value, toEntry(issue)))
  }

  /**
   * Walk the stack in `direction`, calling `attempt` for each candidate until one
   * succeeds. Candidates that fail (deleted/unavailable) are removed from history
   * and skipped. Returns the entry that was navigated to, or null.
   */
  const navigate = async (
    direction: HistoryDirection,
    attempt: (entry: HistoryEntry) => Promise<boolean>,
  ): Promise<HistoryEntry | null> => {
    const step = direction === 'back' ? goBack : goForward
    let s = state.value
    for (;;) {
      const { state: next, entry } = step(s)
      if (!entry) break
      if (await attempt(entry)) {
        commit(next)
        return entry
      }
      // Remove from the *pre-move* state so the origin index shifts correctly
      // and the next iteration steps over the gap.
      s = removeEntries(s, [entry.id])
      commit(s)
    }
    return null
  }

  const back = (attempt: (entry: HistoryEntry) => Promise<boolean>) => navigate('back', attempt)
  const forward = (attempt: (entry: HistoryEntry) => Promise<boolean>) => navigate('forward', attempt)

  /** Drop ids that no longer exist (deleted issues) from stack and MRU. */
  const remove = (ids: Iterable<string>) => {
    commit(removeEntries(state.value, ids))
  }

  /** Keep only ids present in `existingIds`. */
  const prune = (existingIds: Set<string>) => {
    commit(pruneEntries(state.value, existingIds))
  }

  /** Clear the session stack (project switch). The persisted MRU reloads with the project. */
  const reset = () => {
    state.value = resetHistory(Array.isArray(persistedMru.value) ? persistedMru.value : [])
  }

  return {
    state,
    canGoBack,
    canGoForward,
    currentId,
    mruEntries,
    record,
    navigate,
    back,
    forward,
    remove,
    prune,
    reset,
  }
}
