import { describe, it, expect } from 'vitest'
import {
  humanizeKey,
  groupForKey,
  resolveFieldDef,
  coerceNumber,
  clampPercent,
  formatEffort,
  parseDateOnly,
  formatDateOnly,
  daysBetween,
  parseWikiLink,
  toStringList,
  formatGenericValue,
  severityTone,
  badgeTone,
  buildEffortSummary,
  buildScheduleSummary,
  partitionMetadata,
  firstPresent,
  KNOWN_FIELDS,
  SEVERITY_TONES,
  NEUTRAL_TONE,
  PROJECT_TONES,
} from '~/utils/custom-fields'

describe('humanizeKey', () => {
  it('humanizes snake, dotted, and camel keys', () => {
    expect(humanizeKey('source_task_id')).toBe('Source task id')
    expect(humanizeKey('ado.area_path')).toBe('Area path')
    expect(humanizeKey('storyPoints')).toBe('Story points')
    expect(humanizeKey('a/b/c_d')).toBe('C d')
  })
  it('falls back to the key for degenerate input', () => {
    expect(humanizeKey('___')).toBe('___')
  })
})

describe('groupForKey / resolveFieldDef', () => {
  it('uses the registry group when known', () => {
    expect(groupForKey('project')).toBe('custom')
    expect(groupForKey('ado.severity')).toBe('ado')
    expect(groupForKey('beads_priority')).toBe('ado')
    expect(groupForKey('execution_mode')).toBe('execution')
  })
  it('infers namespaces for unknown keys by prefix', () => {
    expect(groupForKey('ado.something_new')).toBe('ado')
    expect(groupForKey('execution_budget')).toBe('execution')
    expect(groupForKey('whatever')).toBe('custom')
  })
  it('returns registry defs and generic defs with inferred kind', () => {
    expect(resolveFieldDef('refs', [])).toBe(KNOWN_FIELDS.refs)
    expect(resolveFieldDef('foo', 3)).toMatchObject({ key: 'foo', label: 'Foo', kind: 'number', group: 'custom' })
    expect(resolveFieldDef('foo', 'x').kind).toBe('text')
    expect(resolveFieldDef('foo', { a: 1 }).kind).toBe('generic')
    expect(resolveFieldDef('ado.new', 'x')).toMatchObject({ group: 'ado', label: 'New' })
  })
})

describe('numbers', () => {
  it('coerces numbers and numeric strings only', () => {
    expect(coerceNumber(4.5)).toBe(4.5)
    expect(coerceNumber('56')).toBe(56)
    expect(coerceNumber(' 7 ')).toBe(7)
    expect(coerceNumber('')).toBeNull()
    expect(coerceNumber('abc')).toBeNull()
    expect(coerceNumber(NaN)).toBeNull()
    expect(coerceNumber(Infinity)).toBeNull()
    expect(coerceNumber(null)).toBeNull()
    expect(coerceNumber(true)).toBeNull()
  })
  it('clamps and rounds percent', () => {
    expect(clampPercent(56.4)).toBe(56)
    expect(clampPercent('120')).toBe(100)
    expect(clampPercent(-3)).toBe(0)
    expect(clampPercent('n/a')).toBeNull()
  })
  it('formats effort', () => {
    expect(formatEffort(8)).toBe('8 h')
    expect(formatEffort(3.5)).toBe('3.5 h')
    expect(formatEffort(4.0)).toBe('4 h')
    expect(formatEffort(2.25)).toBe('2.3 h')
    expect(formatEffort('1.5', 'd')).toBe('1.5 d')
    expect(formatEffort(undefined)).toBe('—')
  })
})

describe('dates', () => {
  it('parses YYYY-MM-DD and ISO, rejects invalid', () => {
    expect(parseDateOnly('2026-03-01')?.toISOString()).toBe('2026-03-01T00:00:00.000Z')
    expect(parseDateOnly('2026-03-01T15:00:00Z')?.toISOString()).toBe('2026-03-01T00:00:00.000Z')
    expect(parseDateOnly('2026-02-31')).toBeNull()
    expect(parseDateOnly('03/01/2026')).toBeNull()
    expect(parseDateOnly(20260301)).toBeNull()
    expect(parseDateOnly('')).toBeNull()
  })
  it('formats or passes through', () => {
    expect(formatDateOnly('2026-03-01T15:00:00Z')).toBe('2026-03-01')
    expect(formatDateOnly('next tuesday')).toBe('next tuesday')
    expect(formatDateOnly(null)).toBe('—')
  })
  it('computes day differences', () => {
    expect(daysBetween('2026-03-01', '2026-03-05')).toBe(4)
    expect(daysBetween('2026-03-05', '2026-03-01')).toBe(-4)
    expect(daysBetween('2026-03-01', 'bad')).toBeNull()
  })
})

describe('parseWikiLink', () => {
  it('parses targets and aliases', () => {
    expect(parseWikiLink('[[Some Note]]')).toEqual({ raw: '[[Some Note]]', target: 'Some Note', alias: null })
    expect(parseWikiLink('  [[Folder/Note|Shown]] ')).toMatchObject({ target: 'Folder/Note', alias: 'Shown' })
  })
  it('passes plain strings through', () => {
    expect(parseWikiLink('TASK-1')).toEqual({ raw: 'TASK-1', target: 'TASK-1', alias: null })
    expect(parseWikiLink('[[unclosed')).toMatchObject({ target: '[[unclosed', alias: null })
  })
})

describe('toStringList / formatGenericValue', () => {
  it('normalizes refs-like values', () => {
    expect(toStringList('[[A]]')).toEqual(['[[A]]'])
    expect(toStringList(['[[A]]', 3, '', '[[B]]'])).toEqual(['[[A]]', '[[B]]'])
    expect(toStringList(42)).toEqual([])
    expect(toStringList('  ')).toEqual([])
  })
  it('formats generic values safely', () => {
    expect(formatGenericValue('x')).toBe('x')
    expect(formatGenericValue(3)).toBe('3')
    expect(formatGenericValue(false)).toBe('false')
    expect(formatGenericValue(null)).toBe('—')
    expect(formatGenericValue({ a: [1, { b: 2 }] })).toBe('{"a":[1,{"b":2}]}')
    expect(formatGenericValue('x'.repeat(300), 50)).toHaveLength(50)
    expect(formatGenericValue('x'.repeat(300), 50).endsWith('…')).toBe(true)
  })
})

describe('tones', () => {
  it('maps ADO severity strings', () => {
    expect(severityTone('1 - Critical')).toBe(SEVERITY_TONES['1'])
    expect(severityTone('2 - High')).toBe(SEVERITY_TONES['2'])
    expect(severityTone('4')).toBe(SEVERITY_TONES['4'])
    expect(severityTone('medium')).toBe(SEVERITY_TONES['3'])
    expect(severityTone('9 - Weird')).toBe(NEUTRAL_TONE)
    expect(severityTone(2)).toBe(NEUTRAL_TONE)
  })
  it('maps project tiers case-insensitively and falls back to neutral', () => {
    expect(badgeTone(KNOWN_FIELDS.project!, 'Gold')).toBe(PROJECT_TONES.gold)
    expect(badgeTone(KNOWN_FIELDS.project!, 'iron')).toBe(PROJECT_TONES.iron)
    expect(badgeTone(KNOWN_FIELDS.project!, 'diamond')).toBe(NEUTRAL_TONE)
    expect(badgeTone(KNOWN_FIELDS.project!, 7)).toBe(NEUTRAL_TONE)
    expect(badgeTone(KNOWN_FIELDS.execution_mode!, 'fast')).toBe(NEUTRAL_TONE)
  })
})

describe('buildEffortSummary', () => {
  it('returns null when no effort keys are present', () => {
    expect(buildEffortSummary({ project: 'gold' })).toBeNull()
  })
  it('reads all four values and clamps percent', () => {
    expect(buildEffortSummary({ original_estimate: 8, remaining_work: 3.5, completed_work: 4.5, percent_complete: 56 }))
      .toEqual({ original: 8, remaining: 3.5, completed: 4.5, percent: 56, unit: 'h' })
  })
  it('derives percent from completed/(completed+remaining) when missing', () => {
    expect(buildEffortSummary({ remaining_work: 2, completed_work: 6 })!.percent).toBe(75)
    expect(buildEffortSummary({ remaining_work: 0, completed_work: 0 })!.percent).toBeNull()
  })
  it('tolerates numeric strings and ignores garbage', () => {
    const s = buildEffortSummary({ original_estimate: '10', percent_complete: 'lots' })!
    expect(s.original).toBe(10)
    expect(s.percent).toBeNull()
  })
})

describe('buildScheduleSummary', () => {
  it('returns null without dates', () => {
    expect(buildScheduleSummary({ x: 1 })).toBeNull()
    expect(buildScheduleSummary({ start_date: '', end_date: null })).toBeNull()
  })
  it('computes inclusive duration and inverted flag', () => {
    expect(buildScheduleSummary({ start_date: '2026-03-01', end_date: '2026-03-05' }))
      .toEqual({ start: '2026-03-01', end: '2026-03-05', durationDays: 5, inverted: false })
    expect(buildScheduleSummary({ start_date: '2026-03-05', end_date: '2026-03-01' })!.inverted).toBe(true)
  })
  it('handles a single or invalid date', () => {
    expect(buildScheduleSummary({ start_date: '2026-03-01' })).toEqual({ start: '2026-03-01', end: null, durationDays: null, inverted: false })
    expect(buildScheduleSummary({ start_date: 'soon', end_date: '2026-03-01' })!.durationDays).toBeNull()
  })
})

describe('partitionMetadata', () => {
  const meta = {
    zeta: true,
    'ado.severity': '2 - High',
    'ado.area_path': 'Team\\Area',
    project: 'gold',
    refs: ['[[A]]'],
    percent_complete: 56,
    remaining_work: 3.5,
    completed_work: 4.5,
    start_date: '2026-03-01',
    end_date: '2026-03-05',
    execution_mode: 'auto',
    'ado.fresh_key': 1,
    nested: { a: 1 },
  }

  it('hides summary keys when summaries exist and counts all keys', () => {
    const p = partitionMetadata(meta)
    expect(p.count).toBe(13)
    expect(p.effort?.percent).toBe(56)
    expect(p.schedule?.durationDays).toBe(5)
    const allKeys = p.groups.flatMap(g => g.fields.map(f => f.key))
    for (const k of ['percent_complete', 'remaining_work', 'completed_work', 'start_date', 'end_date']) {
      expect(allKeys).not.toContain(k)
    }
  })

  it('orders groups per FIELD_GROUPS and fields registry-first then alphabetical', () => {
    const p = partitionMetadata(meta)
    expect(p.groups.map(g => g.id)).toEqual(['ado', 'execution', 'custom'])
    expect(p.groups[0]!.fields.map(f => f.key)).toEqual(['ado.area_path', 'ado.severity', 'ado.fresh_key'])
    expect(p.groups[2]!.fields.map(f => f.key)).toEqual(['project', 'refs', 'nested', 'zeta'])
  })

  it('keeps effort keys as plain fields when no summary is produced', () => {
    // percent_complete alone still produces an effort summary; a bare date alone produces schedule.
    // A metadata with none of the summary keys shows everything as fields.
    const p = partitionMetadata({ project: 'iron', foo: 'bar' })
    expect(p.effort).toBeNull()
    expect(p.schedule).toBeNull()
    expect(p.groups).toHaveLength(1)
    expect(p.groups[0]!.fields.map(f => f.key)).toEqual(['project', 'foo'])
  })

  it('handles an empty object', () => {
    const p = partitionMetadata({})
    expect(p.count).toBe(0)
    expect(p.groups).toEqual([])
  })
})

describe('ado.* aliases (what bd Azure DevOps sync writes)', () => {
  it('firstPresent skips undefined, null, and empty strings', () => {
    expect(firstPresent({ a: '', b: null, c: 0 }, ['a', 'b', 'c'])).toBe(0)
    expect(firstPresent({ a: 'x' }, ['zz', 'a'])).toBe('x')
    expect(firstPresent({}, ['a'])).toBeUndefined()
  })

  it('effort summary reads ado.remaining_work (bd today) when the bare key is absent', () => {
    const s = buildEffortSummary({ 'ado.remaining_work': 3.5, 'ado.rev': 4 })!
    expect(s.remaining).toBe(3.5)
    expect(s.original).toBeNull()
  })

  it('bare keys win over ado.* aliases', () => {
    const s = buildEffortSummary({ remaining_work: 1, 'ado.remaining_work': 9 })!
    expect(s.remaining).toBe(1)
  })

  it('schedule summary falls back through ado.finish_date / target_date', () => {
    expect(buildScheduleSummary({ 'ado.start_date': '2026-03-01T00:00:00Z', 'ado.target_date': '2026-03-10T00:00:00Z' }))
      .toEqual({ start: '2026-03-01', end: '2026-03-10', durationDays: 10, inverted: false })
    expect(buildScheduleSummary({ 'ado.finish_date': '2026-04-01' })!.end).toBe('2026-04-01')
  })

  it('alias keys are hidden from the field list when folded into a summary', () => {
    const p = partitionMetadata({ 'ado.remaining_work': 2, 'ado.area_path': 'A', 'ado.start_date': '2026-01-01' })
    const keys = p.groups.flatMap(g => g.fields.map(f => f.key))
    expect(keys).toEqual(['ado.area_path'])
    expect(p.effort?.remaining).toBe(2)
    expect(p.schedule?.start).toBe('2026-01-01')
  })

  it('ado scheduling keys are registered under the Azure DevOps group with typed widgets', () => {
    expect(KNOWN_FIELDS['ado.original_estimate']).toMatchObject({ group: 'ado', kind: 'effort' })
    expect(KNOWN_FIELDS['ado.finish_date']).toMatchObject({ group: 'ado', kind: 'date' })
  })
})
