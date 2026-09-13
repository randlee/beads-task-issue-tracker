import { describe, it, expect } from 'vitest'
import { normalizeMetadata, hasMetadata, formatMetadataJson } from '~/utils/metadata'

describe('normalizeMetadata', () => {
  it('returns objects as-is when non-empty', () => {
    const meta = { project: 'gold', refs: ['[[Note]]'], nested: { a: 1 } }
    expect(normalizeMetadata(meta)).toBe(meta)
  })
  it('returns null for absent, null, empty object, blank string', () => {
    expect(normalizeMetadata(undefined)).toBeNull()
    expect(normalizeMetadata(null)).toBeNull()
    expect(normalizeMetadata({})).toBeNull()
    expect(normalizeMetadata('')).toBeNull()
    expect(normalizeMetadata('   ')).toBeNull()
    expect(normalizeMetadata('{}')).toBeNull()
  })
  it('parses a JSON string holding an object', () => {
    expect(normalizeMetadata('{"project":"iron"}')).toEqual({ project: 'iron' })
    expect(normalizeMetadata('  {"a": 1}  ')).toEqual({ a: 1 })
  })
  it('returns null for non-object values and invalid JSON strings', () => {
    expect(normalizeMetadata('[1,2]')).toBeNull()
    expect(normalizeMetadata([1, 2])).toBeNull()
    expect(normalizeMetadata(42)).toBeNull()
    expect(normalizeMetadata('not json')).toBeNull()
    expect(normalizeMetadata('"just a string"')).toBeNull()
  })
})

describe('hasMetadata', () => {
  it('mirrors normalizeMetadata presence', () => {
    expect(hasMetadata({ a: 1 })).toBe(true)
    expect(hasMetadata('{"a":1}')).toBe(true)
    expect(hasMetadata({})).toBe(false)
    expect(hasMetadata(null)).toBe(false)
    expect(hasMetadata('oops')).toBe(false)
  })
})

describe('formatMetadataJson', () => {
  it('pretty-prints objects', () => {
    expect(formatMetadataJson({ a: 1, b: [1, 2] })).toBe('{\n  "a": 1,\n  "b": [\n    1,\n    2\n  ]\n}')
  })
  it('re-indents JSON strings and passes through non-JSON strings', () => {
    expect(formatMetadataJson('{"a":1}')).toBe('{\n  "a": 1\n}')
    expect(formatMetadataJson('free text')).toBe('free text')
  })
  it('returns empty string for null/undefined', () => {
    expect(formatMetadataJson(null)).toBe('')
    expect(formatMetadataJson(undefined)).toBe('')
  })
  it('is safe for arrays and numbers', () => {
    expect(formatMetadataJson([1])).toBe('[\n  1\n]')
    expect(formatMetadataJson(7)).toBe('7')
  })
})
