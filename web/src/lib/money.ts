import { DECIMAL_SEPARATOR_CHARS, type DecimalSeparator } from './decimal-separator'

export const MAX_MONEY_DIGITS = 15

export interface MoneySeparators {
  decimal: string
  group: string
}

export interface MoneyMask {
  decimals: number
  separators: MoneySeparators
  allowNegative: boolean
}

export function moneySeparatorsFromSetting(separator: DecimalSeparator): MoneySeparators {
  return { decimal: DECIMAL_SEPARATOR_CHARS[separator], group: '' }
}

function canonicalFromDigits(digits: string, negative: boolean, decimals: number): string {
  const padded = digits.padStart(decimals + 1, '0')
  const whole = padded.slice(0, padded.length - decimals)
  const fraction = padded.slice(padded.length - decimals)
  const sign = negative ? '-' : ''
  return decimals > 0 ? `${sign}${whole}.${fraction}` : `${sign}${whole}`
}

function significantDigits(text: string): string {
  return text.replace(/\D/g, '').replace(/^0+/, '').slice(0, MAX_MONEY_DIGITS)
}

export function formatMoney(canonical: string, { decimals, separators }: Pick<MoneyMask, 'decimals' | 'separators'>): string {
  if (canonical === '') return ''
  const negative = canonical.startsWith('-')
  const [whole = '0', fraction = ''] = canonical.replace('-', '').split('.')
  const grouped = separators.group ? whole.replace(/\B(?=(\d{3})+(?!\d))/g, separators.group) : whole
  const fixedFraction = fraction.padEnd(decimals, '0').slice(0, decimals)
  const body = decimals > 0 ? `${grouped}${separators.decimal}${fixedFraction}` : grouped
  return negative ? `-${body}` : body
}

export function moneyFromTyped(text: string, previousDisplay: string, mask: MoneyMask): string {
  const negative = mask.allowNegative && text.includes('-')
  const digits = significantDigits(text)
  if (digits === '') {
    const deleting = text.length < previousDisplay.length
    if (deleting || (!negative && !/\d/.test(text))) return ''
    return canonicalFromDigits('', negative, mask.decimals)
  }
  return canonicalFromDigits(digits, negative, mask.decimals)
}

export function moneyFromPasted(text: string, mask: MoneyMask): string | null {
  const cleaned = text.replace(/[^\d.,]/g, '')
  if (!/\d/.test(cleaned)) return null
  const negative = mask.allowNegative && text.includes('-')
  const markIndex = Math.max(cleaned.lastIndexOf('.'), cleaned.lastIndexOf(','))
  const trailing = markIndex === -1 ? 0 : cleaned.length - markIndex - 1
  const hasFraction = mask.decimals > 0 && trailing > 0 && trailing <= mask.decimals
  const wholeText = hasFraction ? cleaned.slice(0, markIndex) : cleaned
  const fractionText = hasFraction ? cleaned.slice(markIndex + 1) : ''
  const digits = `${wholeText.replace(/\D/g, '')}${fractionText.padEnd(mask.decimals, '0')}`
    .replace(/^0+/, '')
    .slice(0, MAX_MONEY_DIGITS)
  return canonicalFromDigits(digits, negative, mask.decimals)
}

export function moneyFromNumber(value: number | null, decimals: number): string {
  if (value === null || !Number.isFinite(value)) return ''
  const fixed = Math.abs(value).toFixed(decimals)
  return value < 0 || Object.is(value, -0) ? `-${fixed}` : fixed
}

export function moneyToNumber(canonical: string): number | null {
  if (canonical === '') return null
  const parsed = Number(canonical)
  return Number.isFinite(parsed) ? parsed : null
}
