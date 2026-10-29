import { useTranslation } from 'react-i18next'
import { formatNumber, useRecordFormatting } from '@/features/records/formatting'
import type { ReportTotalsData, ReportTotalsPrevious } from '../types'
import { Delta, StatTile } from '@/kernel/widgets/shared'

type MetricKey = Exclude<keyof ReportTotalsData, 'previous'>

const METRICS: { key: MetricKey; label: string }[] = [
  { key: 'contacts_total', label: 'contactsTotal' },
  { key: 'open_deals', label: 'openDeals' },
  { key: 'won_deals', label: 'wonDeals' },
  { key: 'lost_deals', label: 'lostDeals' },
  { key: 'pending_tasks', label: 'pendingTasks' },
  { key: 'completed_activities', label: 'completedActivities' },
]

function previousOf(previous: ReportTotalsPrevious | null | undefined, key: MetricKey): number | null {
  if (!previous || !(key in previous)) return null
  return previous[key as keyof ReportTotalsPrevious]
}

export function ReportTotalsWidget({ data }: { data: ReportTotalsData }) {
  const { t } = useTranslation('dashboard')
  const formatting = useRecordFormatting()
  const count = (value: number) => formatNumber(value, formatting)
  const visible = METRICS.filter((metric) => data[metric.key] !== null && data[metric.key] !== undefined)
  return (
    <dl className="grid grid-cols-2 gap-3 sm:grid-cols-3 xl:grid-cols-6">
      {visible.map((metric) => {
        const previous = previousOf(data.previous, metric.key)
        return (
          <StatTile
            key={metric.key}
            label={t(`widgets.report_totals.${metric.label}` as 'widgets.report_totals.contactsTotal')}
            value={formatNumber(data[metric.key], formatting)}
            delta={previous !== null ? <Delta current={data[metric.key]} previous={previous} format={count} /> : undefined}
          />
        )
      })}
    </dl>
  )
}
