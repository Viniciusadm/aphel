import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import { formatCurrency, formatNumber, useRecordFormatting } from '@/features/records/formatting'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import type { DealsBySourceData } from './types'

interface SourceBar {
  key: string
  label: string
  count: number
  amount: string | null
}

interface TooltipProps {
  active?: boolean
  payload?: { payload: SourceBar }[]
  currency: string
}

function SourceTooltip({ active, payload, currency }: TooltipProps) {
  const t = useModuleTranslation('insights')
  const formatting = useRecordFormatting()
  const bar = active ? payload?.[0]?.payload : undefined
  if (!bar) return null
  return (
    <div className="rounded-md border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md">
      <p className="font-medium">{bar.label}</p>
      <p>{t('dealsBySource.deals', { count: bar.count })}</p>
      {bar.amount !== null && <p className="text-muted-foreground">{formatCurrency(bar.amount, formatting, currency)}</p>}
    </div>
  )
}

export function DealsBySourceWidget({ data }: { data: DealsBySourceData }) {
  const t = useModuleTranslation('insights')
  const formatting = useRecordFormatting()
  if (data.status === 'source_hidden') {
    return <p className="text-sm text-muted-foreground">{t('dealsBySource.sourceHidden')}</p>
  }
  const bars: SourceBar[] = data.rows.map((row, index) => ({
    key: row.source ?? `none-${index}`,
    label: row.source ?? t('dealsBySource.noSource'),
    count: row.count,
    amount: row.amount,
  }))
  if (data.other) {
    bars.push({ key: 'other', label: t('dealsBySource.other'), count: data.other.count, amount: data.other.amount })
  }
  const chartLabel = t(data.only_won ? 'dealsBySource.chartLabelWon' : 'dealsBySource.chartLabel')
  const height = Math.max(140, bars.length * 32 + 24)
  return (
    <figure className="grid gap-3">
      <div role="img" aria-label={chartLabel} style={{ height }} className="w-full min-w-0">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={bars} layout="vertical" margin={{ top: 4, right: 16, bottom: 4, left: 4 }} barCategoryGap={6}>
            <CartesianGrid horizontal={false} stroke="var(--border)" />
            <XAxis
              type="number"
              allowDecimals={false}
              tickLine={false}
              axisLine={false}
              tick={{ fill: 'var(--muted-foreground)', fontSize: 12 }}
            />
            <YAxis
              type="category"
              dataKey="label"
              width={110}
              tickLine={false}
              axisLine={false}
              tick={{ fill: 'var(--muted-foreground)', fontSize: 12 }}
            />
            <Tooltip cursor={{ fill: 'var(--muted)' }} content={<SourceTooltip currency={data.currency} />} />
            <Bar dataKey="count" name={t('dealsBySource.count')} fill="var(--primary)" radius={[0, 4, 4, 0]} maxBarSize={22} />
          </BarChart>
        </ResponsiveContainer>
      </div>
      <figcaption className="sr-only">{chartLabel}</figcaption>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b text-left text-xs text-muted-foreground">
              <th scope="col" className="py-1.5 pr-2 font-medium">
                {t('dealsBySource.source')}
              </th>
              <th scope="col" className="py-1.5 pr-2 text-right font-medium">
                {t('dealsBySource.count')}
              </th>
              {data.amount_visible && (
                <th scope="col" className="py-1.5 text-right font-medium">
                  {t('dealsBySource.amount')}
                </th>
              )}
            </tr>
          </thead>
          <tbody>
            {bars.map((bar) => (
              <tr key={bar.key} className="border-b last:border-b-0">
                <th scope="row" className="max-w-0 truncate py-1.5 pr-2 text-left font-normal">
                  {bar.label}
                </th>
                <td className="py-1.5 pr-2 text-right tabular-nums">{formatNumber(bar.count, formatting)}</td>
                {data.amount_visible && (
                  <td className="py-1.5 text-right tabular-nums">{formatCurrency(bar.amount, formatting, data.currency)}</td>
                )}
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr className="border-t font-medium">
              <th scope="row" className="py-1.5 pr-2 text-left">
                {t('dealsBySource.total')}
              </th>
              <td className="py-1.5 pr-2 text-right tabular-nums">{formatNumber(data.total.count, formatting)}</td>
              {data.amount_visible && (
                <td className="py-1.5 text-right tabular-nums">{formatCurrency(data.total.amount, formatting, data.currency)}</td>
              )}
            </tr>
          </tfoot>
        </table>
      </div>
      {!data.amount_visible && <p className="text-xs text-muted-foreground">{t('dealsBySource.amountHidden')}</p>}
    </figure>
  )
}
