export type RecordTypeSelector = string | readonly string[] | ((recordType: string) => boolean)

const WILDCARD = '*'

function matchesPattern(pattern: string, recordType: string): boolean {
  if (pattern === WILDCARD) return true
  if (pattern.endsWith(WILDCARD)) return recordType.startsWith(pattern.slice(0, -WILDCARD.length))
  return pattern === recordType
}

export function matchesRecordType(selector: RecordTypeSelector | undefined, recordType: string | undefined): boolean {
  if (selector === undefined) return true
  if (recordType === undefined) return false
  if (typeof selector === 'function') return selector(recordType)
  if (typeof selector === 'string') return matchesPattern(selector, recordType)
  return selector.some((pattern) => matchesPattern(pattern, recordType))
}
