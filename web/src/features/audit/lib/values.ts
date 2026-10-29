export const REDACTED = '[redacted]'

export type DisplayValue = { kind: 'redacted' } | { kind: 'empty' } | { kind: 'text'; text: string }

export function displayValue(value: unknown): DisplayValue {
  if (value === REDACTED) return { kind: 'redacted' }
  if (value === null || value === undefined || value === '') return { kind: 'empty' }
  if (typeof value === 'string') return { kind: 'text', text: value }
  if (typeof value === 'number' || typeof value === 'boolean') return { kind: 'text', text: String(value) }
  if (Array.isArray(value) && value.length === 0) return { kind: 'empty' }
  return { kind: 'text', text: JSON.stringify(value) }
}
