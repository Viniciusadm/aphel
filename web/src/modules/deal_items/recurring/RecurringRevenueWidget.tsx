import { useTranslation } from 'react-i18next'
import { Delta, StatTile } from '@/kernel/widgets/shared'
import { formatCurrency, formatNumber, useRecordFormatting } from '@/features/records/formatting'
import type { RecurringRevenueData } from './types'

function amountOf(value: string | null | undefined): number | null {
  if (value === null || value === undefined) return null
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

export function RecurringRevenueWidget({ data }: { data: RecurringRevenueData }) {
  const { t } = useTranslation('deal_items')
  const formatting = useRecordFormatting()
  const money = (value: string | number | null) =>
    value === null ? t('widgets.hidden') : formatCurrency(value, formatting, data.currency)
  const count = (value: number) => formatNumber(value, formatting)
  const previous = data.previous
  return (
    <div className="grid gap-2">
    <dl className="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <StatTile
        label={t('widgets.recurring_revenue.mrr')}
        value={money(data.mrr)}
        delta={previous && <Delta current={amountOf(data.mrr)} previous={amountOf(previous.mrr)} format={money} />}
      />
      <StatTile label={t('widgets.recurring_revenue.arr')} value={money(data.arr)} />
      <StatTile label={t('widgets.recurring_revenue.tcv')} value={money(data.tcv)} />
      <StatTile
        label={t('widgets.recurring_revenue.wonCount')}
        value={count(data.won_count)}
        delta={previous && <Delta current={data.won_count} previous={previous.won_count} format={count} />}
      />
    </dl>
      {data.partial && <p className="text-xs text-muted-foreground">{t('widgets.recurring_revenue.partial')}</p>}
    </div>
  )
}
