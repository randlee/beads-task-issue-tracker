import { describe, it, expect, vi, beforeEach } from 'vitest'
import { ref } from 'vue'
import type { Issue } from '~/types/issue'

// Live issues list shared with the mocked useIssues
const liveIssues = ref<Pick<Issue, 'id' | 'title' | 'status'>[]>([])

vi.mock('~/composables/useIssues', () => ({
  useIssues: () => ({ issues: liveIssues }),
}))

vi.mock('~/composables/useProjectStorage', () => ({
  useProjectStorage: <T>(_key: string, defaultValue: T) => ref(defaultValue),
}))

import { useIssueHistory } from '~/composables/useIssueHistory'

const issue = (id: string, status = 'open', title = `Title ${id}`) => ({ id, title, status }) as Issue

describe('useIssueHistory', () => {
  beforeEach(() => {
    liveIssues.value = []
    const { reset, state } = useIssueHistory()
    reset()
    state.value = { stack: [], index: -1, mru: [] }
  })

  it('records opens and exposes back/forward availability', () => {
    const h = useIssueHistory()
    expect(h.canGoBack.value).toBe(false)
    h.record(issue('a'))
    h.record(issue('b'))
    expect(h.canGoBack.value).toBe(true)
    expect(h.canGoForward.value).toBe(false)
    expect(h.currentId.value).toBe('b')
  })

  it('shares state across callers (singleton)', () => {
    useIssueHistory().record(issue('a'))
    expect(useIssueHistory().mruEntries.value.map(e => e.id)).toEqual(['a'])
  })

  it('back() navigates to the previous entry when the attempt succeeds', async () => {
    const h = useIssueHistory()
    h.record(issue('a'))
    h.record(issue('b'))
    const attempt = vi.fn(async () => true)
    const entry = await h.back(attempt)
    expect(entry?.id).toBe('a')
    expect(attempt).toHaveBeenCalledTimes(1)
    expect(h.currentId.value).toBe('a')
    expect(h.canGoForward.value).toBe(true)
  })

  it('back() skips and removes entries whose attempt fails', async () => {
    const h = useIssueHistory()
    h.record(issue('a'))
    h.record(issue('b'))
    h.record(issue('c'))
    const attempt = vi.fn(async (e: { id: string }) => e.id !== 'b')
    const entry = await h.back(attempt)
    expect(entry?.id).toBe('a')
    expect(attempt.mock.calls.map(c => c[0].id)).toEqual(['b', 'a'])
    expect(h.state.value.stack.map(e => e.id)).toEqual(['a', 'c'])
    expect(h.mruEntries.value.map(e => e.id)).toEqual(['c', 'a'])
    expect(h.currentId.value).toBe('a')
  })

  it('back() returns null and prunes when nothing resolves', async () => {
    const h = useIssueHistory()
    h.record(issue('a'))
    h.record(issue('b'))
    const entry = await h.back(async () => false)
    expect(entry).toBeNull()
    expect(h.state.value.stack.map(e => e.id)).toEqual(['b'])
    expect(h.canGoBack.value).toBe(false)
  })

  it('forward() skips unavailable entries too', async () => {
    const h = useIssueHistory()
    h.record(issue('a'))
    h.record(issue('b'))
    h.record(issue('c'))
    await h.back(async () => true)
    await h.back(async () => true)
    expect(h.currentId.value).toBe('a')
    const entry = await h.forward(async e => e.id !== 'b')
    expect(entry?.id).toBe('c')
    expect(h.state.value.stack.map(e => e.id)).toEqual(['a', 'c'])
  })

  it('mruEntries resolves title/status from the live list, falling back to the snapshot', () => {
    const h = useIssueHistory()
    h.record(issue('a', 'open', 'Old title'))
    h.record(issue('b', 'open'))
    liveIssues.value = [issue('a', 'closed', 'New title')]
    expect(h.mruEntries.value).toEqual([
      { id: 'b', title: 'Title b', status: 'open' },
      { id: 'a', title: 'New title', status: 'closed' },
    ])
  })

  it('remove() and prune() drop ids from stack and mru', () => {
    const h = useIssueHistory()
    h.record(issue('a'))
    h.record(issue('b'))
    h.record(issue('c'))
    h.remove(['b'])
    expect(h.state.value.stack.map(e => e.id)).toEqual(['a', 'c'])
    h.prune(new Set(['c']))
    expect(h.state.value.stack.map(e => e.id)).toEqual(['c'])
    expect(h.mruEntries.value.map(e => e.id)).toEqual(['c'])
  })

  it('reset() clears the stack but keeps the persisted MRU', () => {
    const h = useIssueHistory()
    h.record(issue('a'))
    h.record(issue('b'))
    h.reset()
    expect(h.state.value.stack).toEqual([])
    expect(h.canGoBack.value).toBe(false)
    expect(h.mruEntries.value.map(e => e.id)).toEqual(['b', 'a'])
  })
})
