import { useTranslation } from 'react-i18next'
import { formatCurrency, formatNumber, type RecordFormatting, useRecordFormatting } from '@/sdk'
import { isCountMetric, type Goal, type GoalMetric } from './api'

export function formatGoalValue(
  metric: GoalMetric,
  value: string | number,
  formatting: RecordFormatting,
  currency?: string | null,
): string {
  return isCountMetric(metric) ? formatNumber(value, formatting) : formatCurrency(value, formatting, currency)
}

function parseDay(value: string): Date {
  const [year, month, day] = value.split('-').map(Number)
  return new Date(Date.UTC(year, month - 1, day))
}

export function useGoalPeriodLabel() {
  const { t } = useTranslation('goals')
  const formatting = useRecordFormatting()
  return (goal: Pick<Goal, 'period' | 'starts_on'>) => {
    const start = parseDay(goal.starts_on)
    const year = start.getUTCFullYear()
    if (goal.period === 'year') return t('periodLabel.year', { year })
    if (goal.period === 'quarter') {
      return t('periodLabel.quarter', { quarter: Math.floor(start.getUTCMonth() / 3) + 1, year })
    }
    return new Intl.DateTimeFormat(formatting.locale, { timeZone: 'UTC', month: 'long', year: 'numeric' }).format(start)
  }
}
