import { describe, it, expect } from 'vitest'
import { formatLogLine, formatLogLines } from '~/utils/log-format'

// ---------------------------------------------------------------------------
// formatLogLine
// ---------------------------------------------------------------------------
describe('formatLogLine', () => {
  it('renders a valid JSONL record with the default action omitted', () => {
    const line = JSON.stringify({
      timestamp: '2026-09-13T12:00:00Z',
      level: 'Info',
      target: 'app_lib.cli',
      action: 'log',
      message: 'hello',
    })
    expect(formatLogLine(line)).toBe('[2026-09-13T12:00:00Z][INFO][app_lib.cli] hello')
  })

  it('renders a non-default action as a [<action>] segment', () => {
    const line = JSON.stringify({
      timestamp: '2026-09-13T12:00:00Z',
      level: 'Warn',
      target: 'app_lib.startup',
      action: 'startup',
      message: 'starting',
    })
    expect(formatLogLine(line)).toBe('[2026-09-13T12:00:00Z][WARN][app_lib.startup] [startup] starting')
  })

  it('honors a custom defaultAction', () => {
    const line = JSON.stringify({
      timestamp: '2026-09-13T12:00:00Z',
      level: 'Error',
      target: 'frontend',
      action: 'frontend',
      message: 'boom',
    })
    expect(formatLogLine(line, 'frontend')).toBe('[2026-09-13T12:00:00Z][ERROR][frontend] boom')
  })

  it('renders a record with a missing/partial field as an empty string for that field', () => {
    const line = JSON.stringify({ level: 'Debug', message: 'partial' })
    expect(formatLogLine(line)).toBe('[][DEBUG][] partial')
  })

  it('returns a non-JSON line unchanged', () => {
    expect(formatLogLine('not json at all')).toBe('not json at all')
  })

  it('returns an empty line unchanged', () => {
    expect(formatLogLine('')).toBe('')
  })

  it('returns JSON that is not an object unchanged', () => {
    expect(formatLogLine('42')).toBe('42')
    expect(formatLogLine('"just a string"')).toBe('"just a string"')
    expect(formatLogLine('null')).toBe('null')
  })
})

// ---------------------------------------------------------------------------
// formatLogLines
// ---------------------------------------------------------------------------
describe('formatLogLines', () => {
  it('renders multiple JSONL lines', () => {
    const jsonl = [
      JSON.stringify({ timestamp: 't1', level: 'Info', target: 'a', action: 'log', message: 'm1' }),
      JSON.stringify({ timestamp: 't2', level: 'Error', target: 'b', action: 'log', message: 'm2' }),
    ].join('\n')
    expect(formatLogLines(jsonl)).toBe('[t1][INFO][a] m1\n[t2][ERROR][b] m2')
  })

  it('preserves blank lines and non-JSON lines unchanged', () => {
    const jsonl = [
      JSON.stringify({ timestamp: 't1', level: 'Info', target: 'a', action: 'log', message: 'm1' }),
      '',
      'not json',
    ].join('\n')
    expect(formatLogLines(jsonl)).toBe('[t1][INFO][a] m1\n\nnot json')
  })

  it('returns an empty string for empty input', () => {
    expect(formatLogLines('')).toBe('')
  })
})
