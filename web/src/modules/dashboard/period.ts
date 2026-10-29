import type { LayoutPeriod, PeriodPreset } from './types'

export const PERIOD_PRESETS: PeriodPreset[] = [
  'today',
  'yesterday',
  'last_7_days',
  'last_30_days',
  'week',
  'last_week',
  'month',
  'last_month',
  'quarter',
  'last_quarter',
  'year',
  'last_year',
  'custom',
]

export const DEFAULT_PERIOD: LayoutPeriod = { preset: 'month', from: null, to: null }

const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/

export function isValidCustomRange(from: string | null | undefined, to: string | null | undefined): boolean {
  if (!from || !to || !DATE_PATTERN.test(from) || !DATE_PATTERN.test(to)) return false
  return from <= to
}

export function periodQuery(period: LayoutPeriod): Record<string, string | undefined> {
  if (period.preset === 'custom') {
    return { period: 'custom', from: period.from ?? undefined, to: period.to ?? undefined }
  }
  return { period: period.preset }
}

export function periodKey(period: LayoutPeriod): string {
  return period.preset === 'custom' ? `custom:${period.from ?? ''}:${period.to ?? ''}` : period.preset
}
