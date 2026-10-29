import { toNumber } from './number'

export const DEFAULT_CURRENCY_DECIMALS = 2
export const MAX_CURRENCY_DIGITS = 15

export interface CurrencyFormat {
  locale: string
  currency: string
  decimals: number
}

export function formatCurrencyInput(value: unknown, format: CurrencyFormat): string {
  const number = toNumber(value)
  if (number === null) return ''
  const options = { minimumFractionDigits: format.decimals, maximumFractionDigits: format.decimals }
  try {
    return new Intl.NumberFormat(format.locale, { ...options, style: 'currency', currency: format.currency }).format(number)
  } catch {
    return new Intl.NumberFormat(format.locale, options).format(number)
  }
}

export function parseCurrencyInput(text: string, decimals: number): number | null {
  const allDigits = text.replace(/\D/g, '')
  if (!allDigits) return null
  const digits = allDigits.replace(/^0+/, '').slice(0, MAX_CURRENCY_DIGITS)
  if (!digits) return allDigits.length === decimals ? null : 0
  const amount = Number((Number(digits) / 10 ** decimals).toFixed(decimals))
  return text.includes('-') ? -amount : amount
}
