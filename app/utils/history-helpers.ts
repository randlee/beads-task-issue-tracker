/**
 * Pure, immutable helpers for issue navigation history.
 *
 * Two structures are maintained:
 * - `stack` + `index`: browser-style back/forward stack (session-only)
 * - `mru`: most-recently-used list, newest first (optionally persisted)
 *
 * Entries store id + title/status snapshots only (never Issue objects, which
 * go stale after polling). Every function returns a new state object.
 */

export interface HistoryEntry {
  id: string
  title: string
  status: string
}

export interface HistoryState {
  stack: HistoryEntry[]
  /** Position of the current entry in `stack`; -1 when the stack is empty. */
  index: number
  mru: HistoryEntry[]
}

export interface HistoryLimits {
  maxStack?: number
  maxMru?: number
}

export const DEFAULT_MAX_STACK = 50
export const DEFAULT_MAX_MRU = 20

export function resetHistory(mru: HistoryEntry[] = []): HistoryState {
  return { stack: [], index: -1, mru }
}

export function currentEntry(state: HistoryState): HistoryEntry | null {
  return state.stack[state.index] ?? null
}

export function canGoBack(state: HistoryState): boolean {
  return state.index > 0
}

export function canGoForward(state: HistoryState): boolean {
  return state.index >= 0 && state.index < state.stack.length - 1
}

/** Move `entry` to the front of `mru`, dropping any previous occurrence, capped at `maxMru`. */
export function touchMru(mru: HistoryEntry[], entry: HistoryEntry, maxMru = DEFAULT_MAX_MRU): HistoryEntry[] {
  return [entry, ...mru.filter(e => e.id !== entry.id)].slice(0, maxMru)
}

/**
 * Record a newly opened issue.
 * - No-op on the stack if `entry.id` equals the current entry (title/status are refreshed).
 * - Discards forward entries beyond `index` (browser semantics).
 * - Caps the stack at `maxStack` by dropping the oldest entries.
 * - Always moves the entry to the front of the MRU list.
 */
export function pushEntry(state: HistoryState, entry: HistoryEntry, limits: HistoryLimits = {}): HistoryState {
  const maxStack = limits.maxStack ?? DEFAULT_MAX_STACK
  const maxMru = limits.maxMru ?? DEFAULT_MAX_MRU
  const mru = touchMru(state.mru, entry, maxMru)

  const current = currentEntry(state)
  if (current && current.id === entry.id) {
    const stack = state.stack.slice()
    stack[state.index] = entry
    return { stack, index: state.index, mru }
  }

  let stack = [...state.stack.slice(0, state.index + 1), entry]
  if (stack.length > maxStack) {
    stack = stack.slice(stack.length - maxStack)
  }
  return { stack, index: stack.length - 1, mru }
}

export interface HistoryMove {
  state: HistoryState
  entry: HistoryEntry | null
}

export function goBack(state: HistoryState): HistoryMove {
  if (!canGoBack(state)) return { state, entry: null }
  const index = state.index - 1
  return { state: { ...state, index }, entry: state.stack[index]! }
}

export function goForward(state: HistoryState): HistoryMove {
  if (!canGoForward(state)) return { state, entry: null }
  const index = state.index + 1
  return { state: { ...state, index }, entry: state.stack[index]! }
}

/**
 * Keep only entries whose id satisfies `keep`. The index follows the current
 * entry when it survives; otherwise it points at the nearest surviving entry
 * before it (or the first entry when none precede it). -1 when the stack empties.
 */
function filterEntries(state: HistoryState, keep: (id: string) => boolean): HistoryState {
  const stack: HistoryEntry[] = []
  let survivingUpToIndex = 0
  state.stack.forEach((entry, i) => {
    if (!keep(entry.id)) return
    stack.push(entry)
    if (i <= state.index) survivingUpToIndex++
  })
  const index = stack.length === 0 ? -1 : Math.max(0, survivingUpToIndex - 1)
  return { stack, index, mru: state.mru.filter(e => keep(e.id)) }
}

/** Remove entries whose ids are no longer in `existingIds` (e.g. after a full list refresh). */
export function pruneEntries(state: HistoryState, existingIds: Set<string>): HistoryState {
  return filterEntries(state, id => existingIds.has(id))
}

/** Remove the given ids (e.g. after a delete, or when a back target no longer resolves). */
export function removeEntries(state: HistoryState, ids: Iterable<string>): HistoryState {
  const gone = ids instanceof Set ? ids : new Set(ids)
  return filterEntries(state, id => !gone.has(id))
}

/** Truncate a title for compact menus, appending an ellipsis when cut. */
export function truncateTitle(title: string, max = 40): string {
  const trimmed = title.trim()
  if (trimmed.length <= max) return trimmed
  return trimmed.slice(0, Math.max(0, max - 1)).trimEnd() + '…'
}

/** Short display id: the part after the last dash (matches CopyableId's convention). */
export function shortIssueId(id: string): string {
  const dash = id.lastIndexOf('-')
  return dash === -1 ? id : id.slice(dash + 1)
}
