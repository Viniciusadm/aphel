export type DurationUnit = 'm' | 'h' | 'd' | 'w'
export type DurationSign = '+' | '-'

export const DURATION_UNITS: DurationUnit[] = ['m', 'h', 'd', 'w']

export interface ParsedDuration {
  sign: DurationSign
  amount: number
  unit: DurationUnit
}

const DURATION = /^([+-]?)(\d+)(m|h|d|w)$/

export function parseDuration(value: unknown): ParsedDuration | null {
  if (typeof value !== 'string') return null
  const match = DURATION.exec(value.trim())
  if (!match) return null
  return { sign: match[1] === '-' ? '-' : '+', amount: Number(match[2]), unit: match[3] as DurationUnit }
}

export function formatDuration(duration: ParsedDuration, signed: boolean): string {
  const sign = signed && duration.sign === '-' ? '-' : ''
  return `${sign}${duration.amount}${duration.unit}`
}

export function isDurationValue(value: unknown): boolean {
  return parseDuration(value) !== null
}
