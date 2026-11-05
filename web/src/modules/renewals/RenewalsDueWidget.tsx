import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { StatTile } from '@/kernel/widgets/shared'
import { formatCurrency, formatDate, formatNumber, recordDetailHref, useRecordFormatting } from '@/sdk'
import type { RenewalsDueData } from './types'

export function RenewalsDueWidget({ data }: { data: RenewalsDueData }) {
  const { t } = useTranslation('renewals')
  const formatting = useRecordFormatting()
  const money = (value: string | null, currency: string) =>
    value === null ? t('widgets.hidden') : formatCurrency(value, formatting, currency)
  return (
    <div className="grid gap-3">
      <dl className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <StatTile
          label={t('widgets.renewals_due.count', { days: data.days })}
          value={formatNumber(data.count, formatting)}
        />
        <StatTile label={t('widgets.renewals_due.mrr')} value={money(data.mrr, data.currency)} />
      </dl>
      {data.partial && <p className="text-xs text-muted-foreground">{t('widgets.renewals_due.partial')}</p>}
      {data.items.length > 0 && (
        <ul className="grid gap-2" aria-label={t('widgets.renewals_due.listLabel')}>
          {data.items.map((item) => (
            <li
              key={item.id}
              className="flex min-w-0 flex-wrap items-center justify-between gap-2 rounded-md border p-2 text-sm"
            >
              <div className="grid min-w-0">
                <Link to={recordDetailHref('deals.deal', item.id) ?? '#'} className="truncate font-medium underline-offset-2 hover:underline">
                  {item.title}
                </Link>
                <span className="truncate text-xs text-muted-foreground">
                  {[item.stage_name, item.owner?.display_name].filter(Boolean).join(' · ')}
                </span>
                {item.contract_end && (
                  <span className="truncate text-xs text-muted-foreground">
                    {t('widgets.renewals_due.contractEnd', { date: formatDate(item.contract_end.slice(0, 10), formatting) })}
                  </span>
                )}
              </div>
              {item.amount !== null && (
                <span className="shrink-0 tabular-nums">{money(item.amount, item.currency)}</span>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
