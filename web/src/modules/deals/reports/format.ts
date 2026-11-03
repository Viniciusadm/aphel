import type { TFunction } from 'i18next'
import type { RecordFormatting } from '@/features/records/formatting'

const HOUR = 3600
const DAY = 86_400

export function formatDuration(
  seconds: number | null | undefined,
  formatting: RecordFormatting,
  t: TFunction<'deals', 'reports'>,
): string {
  if (seconds === null || seconds === undefined) return t('states.noData')
  const number = (value: number) => new Intl.NumberFormat(formatting.locale, { maximumFractionDigits: 1 }).format(value)
  if (seconds < HOUR) {
    const minutes = Math.max(1, Math.round(seconds / 60))
    return t('duration.minutes', { count: minutes, value: number(minutes) })
  }
  if (seconds < DAY) {
    const hours = seconds / HOUR
    return t('duration.hours', { count: Math.round(hours), value: number(hours) })
  }
  const days = seconds / DAY
  return t('duration.days', { count: Math.round(days), value: number(days) })
}

export function formatPercent(value: number | null | undefined, formatting: RecordFormatting, empty: string): string {
  if (value === null || value === undefined) return empty
  return new Intl.NumberFormat(formatting.locale, { style: 'percent', maximumFractionDigits: 1 }).format(value)
}

export function amountValue(value: string | null | undefined): number | null {
  if (value === null || value === undefined) return null
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}
