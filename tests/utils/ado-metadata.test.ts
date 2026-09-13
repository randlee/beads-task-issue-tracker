import { describe, it, expect } from 'vitest'
import { parseAdoMetadata, adoPlannedEnd, ADO_KEYS } from '~/utils/ado-metadata'

describe('parseAdoMetadata', () => {
  it('returns an all-empty, not-present view for null, undefined, and non-object input', () => {
    for (const input of [null, undefined, 'x' as unknown as Record<string, unknown>, 42 as unknown as Record<string, unknown>]) {
      const ado = parseAdoMetadata(input)
      expect(ado.present).toBe(false)
      expect(Object.entries(ado).filter(([k]) => k !== 'present').every(([, v]) => v === '')).toBe(true)
    }
  })

  it('is not present when metadata has only non-ADO keys', () => {
    const ado = parseAdoMetadata({ project: 'gold', refs: ['[[A]]'] })
    expect(ado.present).toBe(false)
    expect(ado.areaPath).toBe('')
  })

  it('maps every key bd writes today, numbers rendered as strings', () => {
    const ado = parseAdoMetadata({
      'ado.area_path': 'RadiantSource\\Team',
      'ado.iteration_path': 'RadiantSource\\2026\\09',
      'ado.story_points': 5,
      'ado.remaining_work': 3.5,
      'ado.severity': '2 - High',
      'ado.rev': 11,
      beads_priority: 3,
    })
    expect(ado.present).toBe(true)
    expect(ado.areaPath).toBe('RadiantSource\\Team')
    expect(ado.iterationPath).toBe('RadiantSource\\2026\\09')
    expect(ado.storyPoints).toBe('5')
    expect(ado.remainingWork).toBe('3.5')
    expect(ado.severity).toBe('2 - High')
    expect(ado.rev).toBe('11')
    expect(ado.beadsPriority).toBe('3')
    // scheduling keys absent → empty, no error
    expect(ado.startDate).toBe('')
    expect(ado.completedWork).toBe('')
  })

  it('maps the scheduling keys from the mapping doc', () => {
    const ado = parseAdoMetadata({
      'ado.original_estimate': 8,
      'ado.completed_work': 4.5,
      'ado.effort': 13,
      'ado.start_date': '2026-03-01T00:00:00Z',
      'ado.finish_date': '2026-03-05T00:00:00Z',
      'ado.target_date': '2026-03-31T00:00:00Z',
    })
    expect(ado.present).toBe(true)
    expect(ado.originalEstimate).toBe('8')
    expect(ado.completedWork).toBe('4.5')
    expect(ado.effort).toBe('13')
    expect(ado.startDate).toBe('2026-03-01T00:00:00Z')
    expect(ado.finishDate).toBe('2026-03-05T00:00:00Z')
    expect(ado.targetDate).toBe('2026-03-31T00:00:00Z')
  })

  it('treats a present-but-null key as present with an empty value', () => {
    const ado = parseAdoMetadata({ 'ado.severity': null })
    expect(ado.present).toBe(true)
    expect(ado.severity).toBe('')
  })

  it('never throws on unexpected value shapes', () => {
    const ado = parseAdoMetadata({ 'ado.area_path': { weird: true }, 'ado.rev': [1, 2], 'ado.story_points': false })
    expect(ado.areaPath).toBe('{"weird":true}')
    expect(ado.rev).toBe('[1,2]')
    expect(ado.storyPoints).toBe('false')
  })

  it('ADO_KEYS covers every field except present, with unique keys', () => {
    const fields = Object.keys(parseAdoMetadata({})).filter(k => k !== 'present')
    expect(Object.keys(ADO_KEYS).sort()).toEqual(fields.sort())
    const keys = Object.values(ADO_KEYS)
    expect(new Set(keys).size).toBe(keys.length)
    expect(keys.filter(k => k.startsWith('ado.')).length).toBe(keys.length - 1) // beads_priority is the exception
  })
})

describe('adoPlannedEnd', () => {
  it('prefers finish date, falls back to target date, else empty', () => {
    expect(adoPlannedEnd(parseAdoMetadata({ 'ado.finish_date': 'F', 'ado.target_date': 'T' }))).toBe('F')
    expect(adoPlannedEnd(parseAdoMetadata({ 'ado.target_date': 'T' }))).toBe('T')
    expect(adoPlannedEnd(parseAdoMetadata({}))).toBe('')
  })
})
