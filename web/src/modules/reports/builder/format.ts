import type { TFunction } from 'i18next'
import { type AggregateResult, formatCurrency, formatNumber, type RecordFormatting } from '@/sdk'
type GroupColumn = AggregateResult['groups'][number]
type MetricColumn = AggregateResult['metrics'][number]
type GroupValue = AggregateResult['rows'][number]['groups'][number]

function parseDay(value: string): Date | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value)
  if (!match) return null
  return new Date(Date.UTC(Number(match[1]), Number(match[2]) - 1, Number(match[3])))
}

export function bucketLabel(value: string, granularity: string | null | undefined, formatting: RecordFormatting) {
  const date = parseDay(value)
  if (!date) return value
  const locale = formatting.locale
  switch (granularity ?? 'day') {
    case 'year':
      return String(date.getUTCFullYear())
    case 'quarter':
      return `T${Math.floor(date.getUTCMonth() / 3) + 1}/${date.getUTCFullYear()}`
    case 'month':
      return new Intl.DateTimeFormat(locale, { timeZone: 'UTC', month: 'short', year: 'numeric' }).format(date)
    default:
      return new Intl.DateTimeFormat(locale, { timeZone: 'UTC', dateStyle: 'short' }).format(date)
  }
}

export interface GroupLabels {
  empty: string
  yes: string
  no: string
}

export function groupValueLabel(
  column: GroupColumn,
  group: GroupValue,
  formatting: RecordFormatting,
  labels: GroupLabels,
): string {
  if (group.label) return group.label
  if (group.value === null || group.value === undefined || group.value === '') return labels.empty
  if (column.field_type === 'date' || column.field_type === 'datetime') {
    return bucketLabel(group.value, column.granularity, formatting)
  }
  if (column.field_type === 'checkbox') return group.value === 'true' ? labels.yes : labels.no
  if (column.field_type === 'number' || column.field_type === 'currency') return formatNumber(group.value, formatting)
  return group.value
}

export function isCurrencyMetric(metric: MetricColumn): boolean {
  return metric.field_type === 'currency' && metric.fn !== 'count'
}

export function metricValueLabel(metric: MetricColumn, value: string | number | null | undefined, formatting: RecordFormatting) {
  if (value === null || value === undefined || value === '') return '—'
  if (isCurrencyMetric(metric)) return formatCurrency(value, formatting)
  return formatNumber(value, formatting, metric.fn === 'count' ? 0 : null)
}

export function metricLabel(metric: MetricColumn, t: TFunction<'reports'>): string {
  const fn = t(`functions.${metric.fn}`)
  const field = metric.field_label ?? metric.field
  if (field && metric.to) {
    return t('durationLabel', {
      fn,
      from: field,
      to: metric.to_label ?? metric.to,
      unit: t(`durationUnits.${metric.unit ?? 'days'}`),
    })
  }
  return field ? `${fn} · ${field}` : fn
}

export function variationLabel(ratio: number | null, formatting: RecordFormatting): string {
  if (ratio === null) return '—'
  const formatted = new Intl.NumberFormat(formatting.locale, {
    style: 'percent',
    maximumFractionDigits: 1,
    signDisplay: 'exceptZero',
  }).format(ratio)
  return formatted
}
