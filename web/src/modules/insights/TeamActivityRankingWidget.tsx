import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import { formatNumber, useRecordFormatting } from '@/features/records/formatting'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import type { RankingRow, TeamActivityRankingData } from './types'

interface RankingBar {
  key: string
  label: string
  count: number
}

function RankingTooltip({ active, payload }: { active?: boolean; payload?: { payload: RankingBar }[] }) {
  const t = useModuleTranslation('insights')
  const bar = active ? payload?.[0]?.payload : undefined
  if (!bar) return null
  return (
    <div className="rounded-md border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md">
      <p className="font-medium">{bar.label}</p>
      <p>{t('ranking.tasks', { count: bar.count })}</p>
    </div>
  )
}

export function TeamActivityRankingWidget({ data }: { data: TeamActivityRankingData }) {
  const t = useModuleTranslation('insights')
  const formatting = useRecordFormatting()
  const nameOf = (row: RankingRow) => row.name ?? (row.user_id ? t('ranking.unknownUser') : t('ranking.unassigned'))
  const bars: RankingBar[] = data.rows.map((row) => ({
    key: row.user_id ?? `none-${row.position}`,
    label: nameOf(row),
    count: row.count,
  }))
  const chartLabel = t('ranking.chartLabel')
  const height = Math.max(120, bars.length * 32 + 24)
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
            <Tooltip cursor={{ fill: 'var(--muted)' }} content={<RankingTooltip />} />
            <Bar dataKey="count" name={t('ranking.count')} fill="var(--primary)" radius={[0, 4, 4, 0]} maxBarSize={22} />
          </BarChart>
        </ResponsiveContainer>
      </div>
      <figcaption className="sr-only">{chartLabel}</figcaption>
      <ol className="grid gap-1.5" aria-label={t('ranking.listLabel')}>
        {data.rows.map((row) => (
          <li key={row.user_id ?? `none-${row.position}`} className="grid min-w-0 gap-1 rounded-md border px-3 py-2 text-sm">
            <div className="flex min-w-0 items-center gap-2">
              <span className="min-w-6 shrink-0 text-xs font-medium text-muted-foreground tabular-nums">{t('ranking.position', { position: row.position })}</span>
              <span className="truncate font-medium">{nameOf(row)}</span>
              <span className="ml-auto shrink-0 tabular-nums">{t('ranking.tasks', { count: row.count })}</span>
            </div>
            {data.by_type && row.types.length > 0 && (
              <ul className="flex flex-wrap gap-1 pl-7" aria-label={t('ranking.byTypeLabel', { name: nameOf(row) })}>
                {row.types.map((type) => (
                  <li key={type.id ?? 'none'} className="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">
                    {type.name ?? t('ranking.noType')}: <span className="tabular-nums">{formatNumber(type.count, formatting)}</span>
                  </li>
                ))}
              </ul>
            )}
          </li>
        ))}
      </ol>
      <p className="text-xs text-muted-foreground">{t('ranking.total', { count: data.total })}</p>
    </figure>
  )
}
