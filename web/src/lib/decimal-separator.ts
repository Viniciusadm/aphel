export type DecimalSeparator = 'comma' | 'dot'

export const DECIMAL_SEPARATOR_CHARS: Record<DecimalSeparator, string> = { comma: ',', dot: '.' }

export function parseDecimalSeparator(value: unknown): DecimalSeparator {
  return value === 'dot' ? 'dot' : 'comma'
}
