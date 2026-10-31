import { DECIMAL_SEPARATOR_CHARS, type DecimalSeparator } from './decimal-separator'

export type NumberMaskKind = 'integer' | 'signedInteger' | 'quantity' | 'percent'

interface NumberMaskRule {
  decimals: number
  signed: boolean
  max?: number
}

const RULES: Record<NumberMaskKind, NumberMaskRule> = {
  integer: { decimals: 0, signed: false },
  signedInteger: { decimals: 0, signed: true },
  quantity: { decimals: 3, signed: false },
  percent: { decimals: 2, signed: false, max: 100 },
}

export function numberMaskAllowsDecimals(kind: NumberMaskKind): boolean {
  return RULES[kind].decimals > 0
}

export function maskNumber(raw: string, kind: NumberMaskKind, separator: DecimalSeparator): string {
  const rule = RULES[kind]
  const negative = rule.signed && raw.startsWith('-')
  const cleaned = raw.replace(/[^\d.,]/g, '')
  const markIndex = rule.decimals > 0 ? Math.max(cleaned.lastIndexOf('.'), cleaned.lastIndexOf(',')) : -1
  const integerDigits = (markIndex === -1 ? cleaned : cleaned.slice(0, markIndex)).replace(/\D/g, '')
  const fractionDigits = markIndex === -1 ? '' : cleaned.slice(markIndex + 1).replace(/\D/g, '').slice(0, rule.decimals)
  const body =
    markIndex === -1 ? integerDigits : `${integerDigits}${DECIMAL_SEPARATOR_CHARS[separator]}${fractionDigits}`
  const masked = `${negative ? '-' : ''}${body}`
  if (rule.max !== undefined && Number(numberToCanonical(masked)) > rule.max) return String(rule.max)
  return masked
}

export function numberToCanonical(display: string): string {
  return display.replace(',', '.')
}

export function numberToDisplay(canonical: string, separator: DecimalSeparator): string {
  return canonical.replace('.', DECIMAL_SEPARATOR_CHARS[separator])
}
