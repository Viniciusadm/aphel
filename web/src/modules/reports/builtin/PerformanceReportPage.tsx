import type { TFunction } from 'i18next'
import { Users } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Bar, BarChart, CartesianGrid, Legend, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Delta, StatTile } from '@/kernel/widgets/shared'
import { formatCurrency, formatDate, formatNumber, useRecordFormatting } from '@/features/records/formatting'
import { activeLocale } from '@/lib/locale'
import { cn } from '@/lib/utils'
import { type PerformanceMetrics, type PerformanceReport, type PerformanceRow, usePerformanceReport } from './api'
import { useReportFilters } from './filters'
import { amountValue, formatDuration, formatPercent } from './format'
import { ReportFilters } from './ReportFilters'
import { ReportSkeleton } from './ReportState'
import { Table, TableBody, TableHead, TableHeader, TableRow } from '@/components/ui/table'

const CHART_LIMIT = 10

function useFormatters(currency: string) {
  const { t } = useTranslation('reports', { keyPrefix: 'builtin' })
  const formatting = useRecordFormatting()
  return {
    count: (value: number) => formatNumber(value, formatting),
    money: (value: string | number | null | undefined) =>
      value === null || value === undefined ? t('states.noData') : formatCurrency(value, formatting, currency),
    percent: (value: number | null | undefined) => formatPercent(value, formatting, t('states.noData')),
    duration: (value: number | null | undefined) => formatDuration(value, formatting, t),
  }
}

function rowLabel(row: PerformanceRow, report: PerformanceReport, t: TFunction<'reports', 'builtin'>): string {
  if (row.key === null || row.key === undefined) return t(`performance.empty.${report.dimension}`)
  return row.label ?? (report.dimension === 'source' ? row.key : t('performance.hidden'))
}

function Totals({ report }: { report: PerformanceReport }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builtin' })
  const format = useFormatters(report.currency)
  const totals = report.totals
  const before = report.previous_totals ?? null
  const percent = (value: number) => format.percent(value)
  const duration = (value: number) => format.duration(value)
  const money = (value: number) => format.money(value)
  return (
    <dl className="grid grid-cols-1 gap-3 min-[420px]:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6">
      <StatTile
        label={t('performance.metrics.created')}
        value={format.count(totals.created)}
        delta={before && <Delta current={totals.created} previous={before.created} format={format.count} />}
      />
      <StatTile
        label={t('performance.metrics.won')}
        value={format.count(totals.won)}
        delta={before && <Delta current={totals.won} previous={before.won} format={format.count} />}
      />
      {report.amounts_visible && (
        <StatTile
          label={t('performance.metrics.wonAmount')}
          value={format.money(totals.won_amount)}
          delta={
            before && <Delta current={amountValue(totals.won_amount)} previous={amountValue(before.won_amount)} format={money} />
          }
        />
      )}
      <StatTile
        label={t('performance.metrics.conversion')}
        value={format.percent(totals.conversion_rate)}
        hint={t('performance.metrics.lostHint', { value: format.count(totals.lost) })}
        delta={before && <Delta current={totals.conversion_rate} previous={before.conversion_rate} format={percent} points />}
      />
      <StatTile
        label={t('performance.metrics.cycle')}
        value={format.duration(totals.average_cycle_seconds)}
        delta={
          before && (
            <Delta current={totals.average_cycle_seconds} previous={before.average_cycle_seconds} format={duration} inverse />
          )
        }
      />
      {report.activities_visible && (
        <StatTile
          label={t('performance.metrics.tasks')}
          value={format.count(totals.activities_done ?? 0)}
          delta={
            before && <Delta current={totals.activities_done} previous={before.activities_done} format={format.count} />
          }
        />
      )}
    </dl>
  )
}

interface ChartPoint {
  name: string
  value: number
  previous?: number
}

function PerformanceChart({ report }: { report: PerformanceReport }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builtin' })
  const format = useFormatters(report.currency)
  const byAmount = report.amounts_visible
  const comparing = Boolean(report.previous_totals)
  const metric = (metrics: PerformanceMetrics | null | undefined) =>
    metrics ? (byAmount ? (amountValue(metrics.won_amount) ?? 0) : metrics.won) : 0
  const points: ChartPoint[] = report.rows.slice(0, CHART_LIMIT).map((row) => ({
    name: rowLabel(row, report, t),
    value: metric(row.current),
    ...(comparing ? { previous: metric(row.previous) } : {}),
  }))
  const label = byAmount ? t('performance.chart.amountLabel') : t('performance.chart.countLabel')
  const height = Math.max(180, points.length * (comparing ? 52 : 40) + 48)
  const formatValue = (value: number) => (byAmount ? format.money(value) : format.count(value))
  return (
    <div role="img" aria-label={label} style={{ height }} className="w-full min-w-0">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={points} layout="vertical" margin={{ top: 4, right: 16, bottom: 4, left: 4 }} barCategoryGap={8}>
          <CartesianGrid horizontal={false} stroke="var(--border)" />
          <XAxis
            type="number"
            allowDecimals={false}
            tickLine={false}
            axisLine={false}
            tick={{ fill: 'var(--muted-foreground)', fontSize: 12 }}
            tickFormatter={(value: number) => (byAmount ? new Intl.NumberFormat(activeLocale(), { notation: 'compact' }).format(value) : String(value))}
          />
          <YAxis type="category" dataKey="name" width={110} tickLine={false} axisLine={false} tick={{ fill: 'var(--muted-foreground)', fontSize: 12 }} />
          <Tooltip
            cursor={{ fill: 'var(--muted)' }}
            formatter={(value) => formatValue(Number(value))}
            contentStyle={{ background: 'var(--popover)', border: '1px solid var(--border)', borderRadius: 6, fontSize: 12 }}
          />
          {comparing && <Legend wrapperStyle={{ fontSize: 12 }} />}
          <Bar dataKey="value" name={t('performance.chart.current')} fill="var(--primary)" radius={[0, 4, 4, 0]} maxBarSize={22} />
          {comparing && (
            <Bar
              dataKey="previous"
              name={t('performance.chart.previous')}
              fill="var(--muted-foreground)"
              fillOpacity={0.45}
              radius={[0, 4, 4, 0]}
              maxBarSize={22}
            />
          )}
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}

function Cell({ children, delta }: { children: ReactNode; delta?: ReactNode }) {
  return (
    <td className="py-2 pr-3 text-right tabular-nums last:pr-0">
      <div>{children}</div>
      {delta && <div className="text-xs">{delta}</div>}
    </td>
  )
}

function PerformanceTable({ report }: { report: PerformanceReport }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builtin' })
  const format = useFormatters(report.currency)
  const percent = (value: number) => format.percent(value)
  const duration = (value: number) => format.duration(value)
  const money = (value: number) => format.money(value)
  return (
    <Table className="min-w-[720px]">
      <caption className="sr-only">{t('performance.table.caption')}</caption>
      <TableHeader>
        <TableRow className="text-left text-xs text-muted-foreground">
          <TableHead scope="col" className="pr-3">{t(`dimensions.${report.dimension}`)}</TableHead>
          <TableHead scope="col" className="pr-3 text-right">{t('performance.metrics.created')}</TableHead>
          <TableHead scope="col" className="pr-3 text-right">{t('performance.metrics.won')}</TableHead>
          {report.amounts_visible && (
            <TableHead scope="col" className="pr-3 text-right">{t('performance.metrics.wonAmount')}</TableHead>
          )}
          <TableHead scope="col" className="pr-3 text-right">{t('performance.metrics.lost')}</TableHead>
          <TableHead scope="col" className="pr-3 text-right">{t('performance.metrics.conversion')}</TableHead>
          <TableHead scope="col" className="pr-3 text-right">{t('performance.metrics.cycle')}</TableHead>
          {report.activities_visible && (
            <TableHead scope="col" className="text-right">{t('performance.metrics.tasks')}</TableHead>
          )}
        </TableRow>
      </TableHeader>
      <TableBody>
        {report.rows.map((row) => {
          const now = row.current
          const before = row.previous ?? null
          return (
            <TableRow key={row.key ?? 'none'} className="last:border-0">
              <TableHead scope="row" className={cn('max-w-48 truncate py-2 pr-3 text-left font-medium', !row.key && 'text-muted-foreground')}>
                {rowLabel(row, report, t)}
              </TableHead>
              <Cell delta={before && <Delta current={now.created} previous={before.created} format={format.count} />}>
                {format.count(now.created)}
              </Cell>
              <Cell delta={before && <Delta current={now.won} previous={before.won} format={format.count} />}>
                {format.count(now.won)}
              </Cell>
              {report.amounts_visible && (
                <Cell
                  delta={
                    before && <Delta current={amountValue(now.won_amount)} previous={amountValue(before.won_amount)} format={money} />
                  }
                >
                  {format.money(now.won_amount)}
                </Cell>
              )}
              <Cell delta={before && <Delta current={now.lost} previous={before.lost} format={format.count} inverse />}>
                {format.count(now.lost)}
              </Cell>
              <Cell
                delta={before && <Delta current={now.conversion_rate} previous={before.conversion_rate} format={percent} points />}
              >
                {format.percent(now.conversion_rate)}
              </Cell>
              <Cell
                delta={
                  before && (
                    <Delta current={now.average_cycle_seconds} previous={before.average_cycle_seconds} format={duration} inverse />
                  )
                }
              >
                {format.duration(now.average_cycle_seconds)}
              </Cell>
              {report.activities_visible && (
                <Cell
                  delta={before && <Delta current={now.activities_done} previous={before.activities_done} format={format.count} />}
                >
                  {format.count(now.activities_done ?? 0)}
                </Cell>
              )}
            </TableRow>
          )
        })}
      </TableBody>
    </Table>
  )
}

function PerformanceContent({ report }: { report: PerformanceReport }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builtin' })
  const formatting = useRecordFormatting()
  const range = (period: { from: string; to: string }) =>
    t('periodRange', { from: formatDate(period.from, formatting), to: formatDate(period.to, formatting) })
  return (
    <div className="grid min-w-0 gap-4">
      <p className="text-sm text-muted-foreground">
        {t('performance.summary', { dimension: t(`dimensions.${report.dimension}`), range: range(report.period) })}
        {report.previous_period && ` ${t('comparedWith', { range: range(report.previous_period) })}`}
      </p>
      {report.rows.length === 0 ? (
        <EmptyState icon={Users} title={t('performance.emptyState.title')} description={t('performance.emptyState.description')} />
      ) : (
        <>
          <Totals report={report} />
          <Card className="min-w-0 gap-4 py-4">
            <CardHeader className="px-4">
              <CardTitle className="text-base">
                {report.amounts_visible ? t('performance.chart.amountTitle') : t('performance.chart.countTitle')}
              </CardTitle>
              <CardDescription>
                {t('performance.chart.description', { count: Math.min(CHART_LIMIT, report.rows.length) })}
              </CardDescription>
            </CardHeader>
            <CardContent className="px-4">
              <PerformanceChart report={report} />
            </CardContent>
          </Card>
          <Card className="min-w-0 gap-4 py-4">
            <CardHeader className="px-4">
              <CardTitle className="text-base">{t('performance.table.title')}</CardTitle>
              <CardDescription>{t('performance.table.description')}</CardDescription>
            </CardHeader>
            <CardContent className="px-4">
              <PerformanceTable report={report} />
              {report.truncated && <p className="mt-3 text-xs text-muted-foreground">{t('performance.truncated')}</p>}
            </CardContent>
          </Card>
        </>
      )}
    </div>
  )
}

export function PerformanceReportPage() {
  const state = useReportFilters()
  const report = usePerformanceReport(state.filters)
  return (
    <div className="grid min-w-0 gap-4">
      <ReportFilters state={state} allPipelines withDimension />
      {report.isPending ? (
        <ReportSkeleton />
      ) : report.isError ? (
        <ErrorState onRetry={() => void report.refetch()} />
      ) : (
        <div className={cn('min-w-0 transition-opacity', report.isFetching && 'opacity-60')}>
          <PerformanceContent report={report.data} />
        </div>
      )}
    </div>
  )
}
