export function toNumber(value: unknown): number | null {
  if (typeof value === 'number') return Number.isFinite(value) ? value : null
  if (typeof value === 'string' && value.trim() !== '') {
    const parsed = Number(value)
    return Number.isFinite(parsed) ? parsed : null
  }
  return null
}

interface Separators {
  decimal: string
  group: string
}

const separatorsCache = new Map<string, Separators>()

export function numberSeparators(locale: string): Separators {
  const cached = separatorsCache.get(locale)
  if (cached) return cached
  let parts: Intl.NumberFormatPart[]
  try {
    parts = new Intl.NumberFormat(locale).formatToParts(12345.6)
  } catch {
    parts = new Intl.NumberFormat('en-US').formatToParts(12345.6)
  }
  const separators = {
    decimal: parts.find((part) => part.type === 'decimal')?.value ?? '.',
    group: parts.find((part) => part.type === 'group')?.value ?? ',',
  }
  separatorsCache.set(locale, separators)
  return separators
}

const THOUSANDS_WITH_DOTS = /^-?\d{1,3}(\.\d{3})+$/
const PLAIN_NUMBER = /^-?(\d+\.?\d*|\.\d+)$/

export function parseLocaleNumber(text: string, locale: string): number | null | undefined {
  const compact = text.replace(/[\s  ]/g, '')
  if (compact === '') return null
  const { decimal, group } = numberSeparators(locale)
  let normalized = compact
  if (decimal !== '.' && compact.includes(decimal)) {
    normalized = compact.split(group).join('').replace(decimal, '.')
  } else if (decimal !== '.' && group === '.' && THOUSANDS_WITH_DOTS.test(compact)) {
    normalized = compact.split('.').join('')
  } else if (decimal === '.') {
    normalized = compact.split(group).join('')
  }
  if (!PLAIN_NUMBER.test(normalized)) return undefined
  const parsed = Number(normalized)
  return Number.isFinite(parsed) ? parsed : undefined
}

export function formatNumberDraft(value: unknown, locale: string): string {
  const number = toNumber(value)
  if (number === null) return typeof value === 'string' ? value : ''
  const plain = number.toLocaleString('en-US', { useGrouping: false, maximumFractionDigits: 20 })
  return plain.replace('.', numberSeparators(locale).decimal)
}

export function countDecimals(value: number): number {
  const plain = value.toLocaleString('en-US', { useGrouping: false, maximumFractionDigits: 20 })
  const dot = plain.indexOf('.')
  return dot < 0 ? 0 : plain.length - dot - 1
}
