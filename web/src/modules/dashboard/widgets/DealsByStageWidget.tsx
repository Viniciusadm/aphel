import { useTranslation } from 'react-i18next'
import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import { formatCurrency, formatNumber, useRecordFormatting } from '@/features/records/formatting'
import type { DealsByStageData, StageBar } from '../types'

interface TooltipProps {
  active?: boolean
  payload?: { payload: StageBar }[]
  currency: string
}

function StageTooltip({ active, payload, currency }: TooltipProps) {
  const { t } = useTranslation('dashboard')
  const formatting = useRecordFormatting()
  const stage = active ? payload?.[0]?.payload : undefined
  if (!stage) return null
  return (
    <div className="rounded-md border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md">
      <p className="font-medium">{stage.name}</p>
      <p>{t('widgets.deals_by_stage.deals', { count: stage.count })}</p>
      <p className="text-muted-foreground">{formatCurrency(stage.amount, formatting, currency)}</p>
    </div>
  )
}

export function DealsByStageWidget({ data }: { data: DealsByStageData }) {
  const { t } = useTranslation('dashboard')
  const formatting = useRecordFormatting()
  if (!data.pipeline) {
    return <p className="text-sm text-muted-foreground">{t('widgets.deals_by_stage.noPipeline')}</p>
  }
  const label = t('widgets.deals_by_stage.chartLabel', { pipeline: data.pipeline.name })
  const height = Math.max(160, data.stages.length * 40 + 32)
  return (
    <figure className="grid gap-2">
      <div role="img" aria-label={label} style={{ height }} className="w-full min-w-0">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={data.stages} layout="vertical" margin={{ top: 4, right: 16, bottom: 4, left: 4 }} barCategoryGap={8}>
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
              dataKey="name"
              width={120}
              tickLine={false}
              axisLine={false}
              tick={{ fill: 'var(--muted-foreground)', fontSize: 12 }}
            />
            <Tooltip cursor={{ fill: 'var(--muted)' }} content={<StageTooltip currency={data.currency} />} />
            <Bar dataKey="count" name={t('widgets.deals_by_stage.count')} fill="var(--primary)" radius={[0, 4, 4, 0]} maxBarSize={24} />
          </BarChart>
        </ResponsiveContainer>
      </div>
      <div className="sr-only">
        <table>
          <caption>{label}</caption>
          <thead>
            <tr>
              <th scope="col">{t('widgets.deals_by_stage.title')}</th>
              <th scope="col">{t('widgets.deals_by_stage.count')}</th>
              <th scope="col">{t('widgets.deals_by_stage.amount')}</th>
            </tr>
          </thead>
          <tbody>
            {data.stages.map((stage) => (
              <tr key={stage.id}>
                <th scope="row">{stage.name}</th>
                <td>{formatNumber(stage.count, formatting)}</td>
                <td>{formatCurrency(stage.amount, formatting, data.currency)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </figure>
  )
}
