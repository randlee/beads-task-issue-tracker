import { describe, it, expect } from 'vitest'
import {
  type HistoryEntry,
  type HistoryState,
  resetHistory,
  pushEntry,
  goBack,
  goForward,
  canGoBack,
  canGoForward,
  currentEntry,
  pruneEntries,
  removeEntries,
  touchMru,
  truncateTitle,
  shortIssueId,
} from '~/utils/history-helpers'

const e = (id: string, status = 'open'): HistoryEntry => ({ id, title: `Title ${id}`, status })

function build(...ids: string[]): HistoryState {
  return ids.reduce((s, id) => pushEntry(s, e(id)), resetHistory())
}

describe('history-helpers', () => {
  describe('resetHistory', () => {
    it('returns an empty stack with index -1 and empty mru by default', () => {
      const s = resetHistory()
      expect(s).toEqual({ stack: [], index: -1, mru: [] })
      expect(canGoBack(s)).toBe(false)
      expect(canGoForward(s)).toBe(false)
      expect(currentEntry(s)).toBeNull()
    })

    it('can preserve a provided mru list', () => {
      const s = resetHistory([e('a')])
      expect(s.stack).toEqual([])
      expect(s.mru).toEqual([e('a')])
    })
  })

  describe('pushEntry', () => {
    it('appends entries and advances the index', () => {
      const s = build('a', 'b', 'c')
      expect(s.stack.map(x => x.id)).toEqual(['a', 'b', 'c'])
      expect(s.index).toBe(2)
      expect(currentEntry(s)?.id).toBe('c')
    })

    it('does not mutate the input state', () => {
      const s0 = build('a')
      const frozen = JSON.stringify(s0)
      pushEntry(s0, e('b'))
      expect(JSON.stringify(s0)).toBe(frozen)
    })

    it('dedupes a push of the current id but refreshes its snapshot', () => {
      const s0 = build('a', 'b')
      const s1 = pushEntry(s0, { id: 'b', title: 'Renamed', status: 'closed' })
      expect(s1.stack.map(x => x.id)).toEqual(['a', 'b'])
      expect(s1.index).toBe(1)
      expect(currentEntry(s1)).toEqual({ id: 'b', title: 'Renamed', status: 'closed' })
      expect(s1.mru[0]).toEqual({ id: 'b', title: 'Renamed', status: 'closed' })
    })

    it('allows the same id to appear again when it is not the current entry', () => {
      const s = build('a', 'b', 'a')
      expect(s.stack.map(x => x.id)).toEqual(['a', 'b', 'a'])
    })

    it('truncates forward entries when pushing after going back', () => {
      const s0 = build('a', 'b', 'c')
      const { state: s1 } = goBack(s0)
      const { state: s2 } = goBack(s1)
      expect(currentEntry(s2)?.id).toBe('a')
      const s3 = pushEntry(s2, e('d'))
      expect(s3.stack.map(x => x.id)).toEqual(['a', 'd'])
      expect(s3.index).toBe(1)
      expect(canGoForward(s3)).toBe(false)
    })

    it('caps the stack at maxStack by dropping the oldest entries', () => {
      let s = resetHistory()
      for (let i = 0; i < 6; i++) s = pushEntry(s, e(`i${i}`), { maxStack: 4 })
      expect(s.stack.map(x => x.id)).toEqual(['i2', 'i3', 'i4', 'i5'])
      expect(s.index).toBe(3)
    })

    it('uses default caps of 50 (stack) and 20 (mru)', () => {
      let s = resetHistory()
      for (let i = 0; i < 60; i++) s = pushEntry(s, e(`i${i}`))
      expect(s.stack).toHaveLength(50)
      expect(s.mru).toHaveLength(20)
      expect(s.stack[0]!.id).toBe('i10')
      expect(s.mru[0]!.id).toBe('i59')
    })
  })

  describe('mru', () => {
    it('moves re-opened entries to the front', () => {
      const s = build('a', 'b', 'c', 'a')
      expect(s.mru.map(x => x.id)).toEqual(['a', 'c', 'b'])
    })

    it('caps at maxMru', () => {
      let s = resetHistory()
      for (let i = 0; i < 5; i++) s = pushEntry(s, e(`i${i}`), { maxMru: 3 })
      expect(s.mru.map(x => x.id)).toEqual(['i4', 'i3', 'i2'])
    })

    it('touchMru is usable standalone and never duplicates ids', () => {
      const mru = touchMru([e('a'), e('b')], e('b'))
      expect(mru.map(x => x.id)).toEqual(['b', 'a'])
    })
  })

  describe('goBack / goForward', () => {
    it('walks back and forward within bounds', () => {
      const s0 = build('a', 'b', 'c')
      const b1 = goBack(s0)
      expect(b1.entry?.id).toBe('b')
      expect(b1.state.index).toBe(1)
      const b2 = goBack(b1.state)
      expect(b2.entry?.id).toBe('a')
      expect(canGoBack(b2.state)).toBe(false)

      const f1 = goForward(b2.state)
      expect(f1.entry?.id).toBe('b')
      const f2 = goForward(f1.state)
      expect(f2.entry?.id).toBe('c')
      expect(canGoForward(f2.state)).toBe(false)
    })

    it('returns null entry and the same state at the bounds', () => {
      const s = build('a')
      expect(goBack(s)).toEqual({ state: s, entry: null })
      expect(goForward(s)).toEqual({ state: s, entry: null })
      const empty = resetHistory()
      expect(goBack(empty).entry).toBeNull()
      expect(goForward(empty).entry).toBeNull()
    })

    it('does not touch the mru when navigating', () => {
      const s0 = build('a', 'b')
      const { state } = goBack(s0)
      expect(state.mru).toBe(s0.mru)
    })
  })

  describe('pruneEntries', () => {
    it('removes ids missing from existingIds in both stack and mru', () => {
      const s0 = build('a', 'b', 'c')
      const s1 = pruneEntries(s0, new Set(['a', 'c']))
      expect(s1.stack.map(x => x.id)).toEqual(['a', 'c'])
      expect(s1.mru.map(x => x.id)).toEqual(['c', 'a'])
    })

    it('keeps the index on the current entry when it survives', () => {
      const s0 = build('a', 'b', 'c')
      const s1 = pruneEntries(s0, new Set(['b', 'c']))
      expect(currentEntry(s1)?.id).toBe('c')
      expect(s1.index).toBe(1)
    })

    it('clamps the index to the nearest surviving previous entry when the current one is removed', () => {
      const s0 = goBack(build('a', 'b', 'c')).state // current = b
      const s1 = pruneEntries(s0, new Set(['a', 'c']))
      expect(s1.stack.map(x => x.id)).toEqual(['a', 'c'])
      expect(currentEntry(s1)?.id).toBe('a')
      expect(canGoForward(s1)).toBe(true)
    })

    it('clamps to 0 when everything before and including the current entry is removed', () => {
      const s0 = goBack(build('a', 'b', 'c')).state // current = b
      const s1 = pruneEntries(s0, new Set(['c']))
      expect(s1.stack.map(x => x.id)).toEqual(['c'])
      expect(s1.index).toBe(0)
    })

    it('yields index -1 when the stack empties', () => {
      const s1 = pruneEntries(build('a', 'b'), new Set())
      expect(s1).toEqual({ stack: [], index: -1, mru: [] })
    })
  })

  describe('removeEntries', () => {
    it('is the inverse of pruneEntries: drops the given ids', () => {
      const s0 = build('a', 'b', 'c')
      const s1 = removeEntries(s0, ['b'])
      expect(s1.stack.map(x => x.id)).toEqual(['a', 'c'])
      expect(s1.mru.map(x => x.id)).toEqual(['c', 'a'])
      expect(currentEntry(s1)?.id).toBe('c')
    })

    it('shifts the index down when an earlier entry is removed', () => {
      const s0 = build('a', 'b', 'c')
      const s1 = removeEntries(s0, new Set(['a']))
      expect(s1.index).toBe(1)
      expect(currentEntry(s1)?.id).toBe('c')
    })
  })

  describe('truncateTitle', () => {
    it('returns short titles unchanged (trimmed)', () => {
      expect(truncateTitle('  Short title ')).toBe('Short title')
    })

    it('cuts long titles and appends an ellipsis within max length', () => {
      const out = truncateTitle('A'.repeat(100), 10)
      expect(out).toHaveLength(10)
      expect(out.endsWith('…')).toBe(true)
    })
  })

  describe('shortIssueId', () => {
    it('returns the part after the last dash', () => {
      expect(shortIssueId('proj-abc12')).toBe('abc12')
      expect(shortIssueId('proj-abc12.3')).toBe('abc12.3')
    })

    it('returns the id unchanged when there is no dash', () => {
      expect(shortIssueId('abc')).toBe('abc')
    })
  })
})
