/**
 * Renders sc-observability JSONL log lines for the debug panel.
 *
 * Each line written by the backend is one `sc_observability_types::LogEvent`
 * serialized as JSON (`timestamp`, `level`, `target`, `action`, `message`).
 * `formatLogLine`/`formatLogLines` turn that into the plain-text form the
 * debug panel colorizes: `[<timestamp>][<LEVEL>][<target>] [<action>] <message>`,
 * omitting the `[<action>] ` segment when `action` equals `defaultAction`.
 */

/** Fields of one sc-observability `LogEvent`, as rendered by the debug panel. */
export interface LogRecordView {
  timestamp: string
  level: string
  target: string
  action: string
  message: string
}

const DEFAULT_ACTION = 'log'

interface RawLogEvent {
  timestamp?: unknown
  level?: unknown
  target?: unknown
  action?: unknown
  message?: unknown
}

function asString(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

function parseLogRecord(line: string): LogRecordView | null {
  let parsed: RawLogEvent
  try {
    parsed = JSON.parse(line) as RawLogEvent
  } catch {
    return null
  }
  if (typeof parsed !== 'object' || parsed === null) {
    return null
  }
  return {
    timestamp: asString(parsed.timestamp),
    level: asString(parsed.level).toUpperCase(),
    target: asString(parsed.target),
    action: asString(parsed.action),
    message: asString(parsed.message),
  }
}

/**
 * Renders one JSONL log line as plain text.
 *
 * A line that is not valid JSON, or not a JSON object, is returned unchanged
 * (for example a stray non-JSON line in the file).
 */
export function formatLogLine(line: string, defaultAction: string = DEFAULT_ACTION): string {
  const record = parseLogRecord(line)
  if (!record) {
    return line
  }
  const actionSegment = record.action && record.action !== defaultAction ? `[${record.action}] ` : ''
  return `[${record.timestamp}][${record.level}][${record.target}] ${actionSegment}${record.message}`
}

/** Renders every JSONL line, preserving blank lines and non-JSON lines unchanged. */
export function formatLogLines(jsonl: string, defaultAction: string = DEFAULT_ACTION): string {
  if (!jsonl) {
    return ''
  }
  return jsonl
    .split('\n')
    .map((line) => (line === '' ? line : formatLogLine(line, defaultAction)))
    .join('\n')
}
