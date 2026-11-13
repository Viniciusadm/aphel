import { ArrowDownRight, ArrowUpRight } from 'lucide-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Line,
  LineChart,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { useTheme } from '@/app/theme/theme-context'
import { type AggregateResult, decimalValue, type RecordFormatting, useRecordFormatting, variation } from '@/sdk'
import { cn } from '@/lib/utils'
import type { ReportVisualization } from './api'
import {
  groupValueLabel,
  isCurrencyMetric,
  metricLabel,
  metricValueLabel,
  variationLabel,
  type GroupLabels,
} from './format'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

const LIGHT_SERIES = ['#2a78d6', '#eb6834', '#1baf7a', '#eda100', '#e87ba4', '#008300', '#6250d6', '#e34948']
const DARK_SERIES = ['#3987e5', '#d95926', '#199e70', '#c98500', '#d55181', '#008300', '#9085e9', '#e66767']
const MAX_SERIES = 7
const PREVIOUS_SUFFIX = '__previous'
const OTHER_KEY = '__other'

type MetricColumn = AggregateResult['metrics'][number]

interface ChartSeries {
  key: string
  label: string
  color: string
  previous: boolean
}

interface ChartModel {
  data: Record<string, string | number | null>[]
  series: ChartSeries[]
  metric: MetricColumn
}

export interface ReportResultViewProps {
  result: AggregateResult
  visualization: ReportVisualization
  chartMetric?: string | null
  compact?: boolean
}

function useGroupLabels(): GroupLabels {
  const { t } = useTranslation('reports')
  const { t: tBuilder } = useTranslation('reports', { keyPrefix: 'builder' })
  return useMemo(
    () => ({ empty: t('empty'), yes: tBuilder('result.yes'), no: tBuilder('result.no') }),
    [t, tBuilder],
  )
}

function useSeriesColors(): string[] {
  const { resolvedTheme } = useTheme()
  return resolvedTheme === 'dark' ? DARK_SERIES : LIGHT_SERIES
}

function chartMetricIndex(result: AggregateResult, chartMetric?: string | null): number {
  const index = chartMetric ? result.metrics.findIndex((metric) => metric.key === chartMetric) : -1
  return index >= 0 ? index : 0
}

function buildChart(
  result: AggregateResult,
  metricIndex: number,
  colors: string[],
  formatting: RecordFormatting,
  labels: GroupLabels,
  previousLabel: string,
  otherLabel: string,
  totalLabel: string,
  metricName: string,
): ChartModel | null {
  const metric = result.metrics[metricIndex]
  if (!metric) return null
  const compare = Boolean(result.previous_period)
  const additive = metric.fn === 'count' || metric.fn === 'sum'
  const [xColumn, seriesColumn] = result.groups
  if (!xColumn) {
    const current = decimalValue(result.totals.values[metricIndex])
    const previous = decimalValue(result.totals.previous?.[metricIndex])
    const series: ChartSeries[] = [{ key: 'value', label: metricName, color: colors[0], previous: false }]
    if (compare) series.push({ key: `value${PREVIOUS_SUFFIX}`, label: previousLabel, color: colors[0], previous: true })
    return { metric, series, data: [{ label: totalLabel, value: current, [`value${PREVIOUS_SUFFIX}`]: previous }] }
  }
  const order: string[] = []
  const points = new Map<string, Record<string, string | number | null>>()
  const seriesTotals = new Map<string, { label: string; total: number }>()
  for (const row of result.rows) {
    const x = groupValueLabel(xColumn, row.groups[0], formatting, labels)
    if (!points.has(x)) {
      order.push(x)
      points.set(x, { label: x })
    }
    const point = points.get(x) as Record<string, string | number | null>
    const seriesKey = seriesColumn ? `s:${row.groups[1]?.value ?? ''}` : 'value'
    const seriesLabel = seriesColumn ? groupValueLabel(seriesColumn, row.groups[1], formatting, labels) : metricName
    const current = decimalValue(row.values[metricIndex])
    const previous = decimalValue(row.previous?.[metricIndex])
    const add = (key: string, value: number | null) => {
      const existing = point[key]
      if (typeof existing === 'number' && additive && value !== null) point[key] = existing + value
      else if (existing === undefined || existing === null) point[key] = value
    }
    add(seriesKey, current)
    if (compare) add(`${seriesKey}${PREVIOUS_SUFFIX}`, previous)
    const entry = seriesTotals.get(seriesKey) ?? { label: seriesLabel, total: 0 }
    entry.total += Math.abs(current ?? 0)
    seriesTotals.set(seriesKey, entry)
  }
  const ranked = [...seriesTotals.entries()].sort((left, right) => right[1].total - left[1].total)
  const kept = ranked.slice(0, MAX_SERIES)
  const folded = ranked.slice(MAX_SERIES)
  if (folded.length > 0 && additive) {
    for (const point of points.values()) {
      let current: number | null = null
      let previous: number | null = null
      for (const [key] of folded) {
        const value = point[key]
        if (typeof value === 'number') current = (current ?? 0) + value
        const before = point[`${key}${PREVIOUS_SUFFIX}`]
        if (typeof before === 'number') previous = (previous ?? 0) + before
      }
      point[OTHER_KEY] = current
      point[`${OTHER_KEY}${PREVIOUS_SUFFIX}`] = previous
    }
    kept.push([OTHER_KEY, { label: otherLabel, total: 0 }])
  }
  const series: ChartSeries[] = []
  kept.forEach(([key, entry], index) => {
    const color = colors[index % colors.length]
    series.push({ key, label: entry.label, color, previous: false })
    if (compare) {
      series.push({ key: `${key}${PREVIOUS_SUFFIX}`, label: `${entry.label} · ${previousLabel}`, color, previous: true })
    }
  })
  return { metric, series, data: order.map((x) => points.get(x) as Record<string, string | number | null>) }
}

function ChartTooltip({
  active,
  payload,
  label,
  metric,
  formatting,
}: {
  active?: boolean
  payload?: { name?: string; value?: number | null; color?: string }[]
  label?: string
  metric: MetricColumn
  formatting: RecordFormatting
}) {
  if (!active || !payload || payload.length === 0) return null
  return (
    <div className="max-w-64 rounded-md border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md">
      {label && <p className="mb-1 font-medium">{label}</p>}
      <ul className="grid gap-0.5">
        {payload.map((item) => (
          <li key={item.name} className="flex items-center gap-2">
            <span className="size-2 shrink-0 rounded-full" style={{ background: item.color }} aria-hidden />
            <span className="min-w-0 flex-1 truncate text-muted-foreground">{item.name}</span>
            <span className="font-medium tabular-nums">{metricValueLabel(metric, item.value, formatting)}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}

function compactNumber(value: number, formatting: RecordFormatting, currency: boolean) {
  return new Intl.NumberFormat(formatting.locale, {
    notation: 'compact',
    maximumFractionDigits: 1,
    ...(currency ? { style: 'currency', currency: formatting.currency } : {}),
  }).format(value)
}

function CartesianReport({
  model,
  kind,
  height,
  formatting,
}: {
  model: ChartModel
  kind: 'bar' | 'line'
  height: number
  formatting: RecordFormatting
}) {
  const currency = isCurrencyMetric(model.metric)
  const axisTick = { fill: 'var(--muted-foreground)', fontSize: 12 }
  const common = {
    data: model.data,
    margin: { top: 8, right: 8, bottom: 4, left: 0 },
  }
  const xAxis = <XAxis dataKey="label" tickLine={false} axisLine={false} tick={axisTick} minTickGap={8} />
  const yAxis = (
    <YAxis
      tickLine={false}
      axisLine={false}
      tick={axisTick}
      width={64}
      tickFormatter={(value: number) => compactNumber(value, formatting, currency)}
    />
  )
  const tooltip = <Tooltip content={<ChartTooltip metric={model.metric} formatting={formatting} />} />
  const legend = model.series.length > 1 ? <Legend wrapperStyle={{ fontSize: 12 }} /> : null
  return (
    <div style={{ height }} className="w-full min-w-0">
      <ResponsiveContainer width="100%" height="100%">
        {kind === 'bar' ? (
          <BarChart {...common} barCategoryGap="20%" barGap={2}>
            <CartesianGrid vertical={false} stroke="var(--border)" />
            {xAxis}
            {yAxis}
            <Tooltip cursor={{ fill: 'var(--muted)' }} content={<ChartTooltip metric={model.metric} formatting={formatting} />} />
            {legend}
            {model.series.map((series) => (
              <Bar
                key={series.key}
                dataKey={series.key}
                name={series.label}
                fill={series.color}
                fillOpacity={series.previous ? 0.35 : 1}
                radius={[4, 4, 0, 0]}
                maxBarSize={32}
              />
            ))}
          </BarChart>
        ) : (
          <LineChart {...common}>
            <CartesianGrid vertical={false} stroke="var(--border)" />
            {xAxis}
            {yAxis}
            {tooltip}
            {legend}
            {model.series.map((series) => (
              <Line
                key={series.key}
                type="monotone"
                dataKey={series.key}
                name={series.label}
                stroke={series.color}
                strokeWidth={2}
                strokeDasharray={series.previous ? '5 4' : undefined}
                strokeOpacity={series.previous ? 0.6 : 1}
                dot={{ r: 3 }}
                activeDot={{ r: 5 }}
                connectNulls
              />
            ))}
          </LineChart>
        )}
      </ResponsiveContainer>
    </div>
  )
}

function PieReport({
  result,
  metricIndex,
  colors,
  height,
  formatting,
  labels,
  otherLabel,
}: {
  result: AggregateResult
  metricIndex: number
  colors: string[]
  height: number
  formatting: RecordFormatting
  labels: GroupLabels
  otherLabel: string
}) {
  const metric = result.metrics[metricIndex]
  const column = result.groups[0]
  const slices = useMemo(() => {
    if (!column || !metric) return []
    const values = new Map<string, number>()
    for (const row of result.rows) {
      const label = groupValueLabel(column, row.groups[0], formatting, labels)
      const value = decimalValue(row.values[metricIndex]) ?? 0
      values.set(label, (values.get(label) ?? 0) + value)
    }
    const ranked = [...values.entries()].filter(([, value]) => value > 0).sort((left, right) => right[1] - left[1])
    const kept = ranked.slice(0, MAX_SERIES).map(([name, value]) => ({ name, value }))
    const rest = ranked.slice(MAX_SERIES).reduce((sum, [, value]) => sum + value, 0)
    if (rest > 0) kept.push({ name: otherLabel, value: rest })
    return kept
  }, [column, metric, result.rows, metricIndex, formatting, labels, otherLabel])
  if (!metric) return null
  return (
    <div style={{ height }} className="w-full min-w-0">
      <ResponsiveContainer width="100%" height="100%">
        <PieChart>
          <Tooltip
            content={({ active, payload }) => (
              <ChartTooltip
                active={active}
                metric={metric}
                formatting={formatting}
                payload={payload?.map((item) => ({
                  name: String(item.name),
                  value: typeof item.value === 'number' ? item.value : null,
                  color: (item.payload as { fill?: string } | undefined)?.fill,
                }))}
              />
            )}
          />
          <Legend wrapperStyle={{ fontSize: 12 }} />
          <Pie data={slices} dataKey="value" nameKey="name" innerRadius="45%" outerRadius="80%" paddingAngle={1} isAnimationActive={false} stroke="var(--background)" strokeWidth={2}>
            {slices.map((slice, index) => (
              <Cell key={slice.name} fill={colors[index % colors.length]} />
            ))}
          </Pie>
        </PieChart>
      </ResponsiveContainer>
    </div>
  )
}

function NumberReport({ result, metricIndex, formatting }: { result: AggregateResult; metricIndex: number; formatting: RecordFormatting }) {
  const { t } = useTranslation('reports')
  const metric = result.metrics[metricIndex]
  if (!metric) return null
  const current = decimalValue(result.totals.values[metricIndex])
  const previous = result.totals.previous ? decimalValue(result.totals.previous[metricIndex]) : null
  const ratio = variation(current, previous)
  const up = ratio !== null && ratio > 0
  const down = ratio !== null && ratio < 0
  return (
    <div className="grid gap-2 py-2">
      <p className="text-sm text-muted-foreground">{metricLabel(metric, t)}</p>
      <p className="text-3xl font-semibold tracking-tight tabular-nums sm:text-4xl">
        {metricValueLabel(metric, current, formatting)}
      </p>
      {result.totals.previous && (
        <p className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
          <span>
            {t('comparison.previous')}: {metricValueLabel(metric, previous, formatting)}
          </span>
          <span className={cn('inline-flex items-center gap-1 font-medium text-foreground')}>
            {up && <ArrowUpRight className="size-4" aria-hidden />}
            {down && <ArrowDownRight className="size-4" aria-hidden />}
            {t('comparison.variation')}: {variationLabel(ratio, formatting)}
          </span>
        </p>
      )}
    </div>
  )
}

export function ReportTable({ result, compact = false }: { result: AggregateResult; compact?: boolean }) {
  const { t } = useTranslation('reports')
  const { t: tBuilder } = useTranslation('reports', { keyPrefix: 'builder' })
  const formatting = useRecordFormatting()
  const labels = useGroupLabels()
  const compare = Boolean(result.previous_period)
  const cell = compact ? 'px-2 py-1.5' : 'px-3 py-2'
  return (
    <Table>
      <TableHeader>
        <TableRow>
          {result.groups.map((group) => (
            <TableHead key={group.key} scope="col" className={cn(cell, 'font-medium')}>
              {group.label}
            </TableHead>
          ))}
          {result.metrics.map((metric) => (
            <MetricHeaders key={metric.key} label={metricLabel(metric, t)} compare={compare} cell={cell} />
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>
        {result.rows.map((row, index) => (
          <TableRow key={index}>
            {result.groups.map((group, groupIndex) => (
              <TableCell key={group.key} className={cn(cell, 'whitespace-nowrap')}>
                {groupValueLabel(group, row.groups[groupIndex], formatting, labels)}
              </TableCell>
            ))}
            {result.metrics.map((metric, metricIndex) => (
              <MetricCells
                key={metric.key}
                metric={metric}
                current={row.values[metricIndex]}
                previous={row.previous ? row.previous[metricIndex] : undefined}
                compare={compare}
                cell={cell}
                formatting={formatting}
              />
            ))}
          </TableRow>
        ))}
      </TableBody>
      {result.groups.length > 0 && (
        <tfoot className="border-t-2 bg-muted/30 font-medium">
          <TableRow>
            <TableHead scope="row" colSpan={result.groups.length} className={cn(cell, 'text-left')}>
              {tBuilder('result.total')}
            </TableHead>
            {result.metrics.map((metric, metricIndex) => (
              <MetricCells
                key={metric.key}
                metric={metric}
                current={result.totals.values[metricIndex]}
                previous={result.totals.previous ? result.totals.previous[metricIndex] : undefined}
                compare={compare}
                cell={cell}
                formatting={formatting}
              />
            ))}
          </TableRow>
        </tfoot>
      )}
    </Table>
  )
}

function MetricHeaders({ label, compare, cell }: { label: string; compare: boolean; cell: string }) {
  const { t } = useTranslation('reports')
  return (
    <>
      <th scope="col" className={cn(cell, 'text-right font-medium')}>
        {label}
      </th>
      {compare && (
        <>
          <th scope="col" className={cn(cell, 'text-right font-medium')}>
            {t('comparison.previous')}
          </th>
          <th scope="col" className={cn(cell, 'text-right font-medium')}>
            {t('comparison.variation')}
          </th>
        </>
      )}
    </>
  )
}

function MetricCells({
  metric,
  current,
  previous,
  compare,
  cell,
  formatting,
}: {
  metric: MetricColumn
  current: string | null | undefined
  previous: string | null | undefined
  compare: boolean
  cell: string
  formatting: RecordFormatting
}) {
  const ratio = variation(decimalValue(current), decimalValue(previous))
  return (
    <>
      <td className={cn(cell, 'text-right whitespace-nowrap tabular-nums')}>{metricValueLabel(metric, current, formatting)}</td>
      {compare && (
        <>
          <td className={cn(cell, 'text-right whitespace-nowrap text-muted-foreground tabular-nums')}>
            {metricValueLabel(metric, previous, formatting)}
          </td>
          <td className={cn(cell, 'text-right whitespace-nowrap tabular-nums')}>{variationLabel(ratio, formatting)}</td>
        </>
      )}
    </>
  )
}

export function ReportResultView({ result, visualization, chartMetric, compact = false }: ReportResultViewProps) {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  const { t: tReports } = useTranslation('reports')
  const formatting = useRecordFormatting()
  const labels = useGroupLabels()
  const colors = useSeriesColors()
  const metricIndex = chartMetricIndex(result, chartMetric)
  const height = compact ? 240 : 320
  const chart = useMemo(
    () =>
      buildChart(
        result,
        metricIndex,
        colors,
        formatting,
        labels,
        tReports('comparison.previous'),
        t('result.other'),
        t('result.total'),
        result.metrics[metricIndex] ? metricLabel(result.metrics[metricIndex], tReports) : '',
      ),
    [result, metricIndex, colors, formatting, labels, tReports, t],
  )

  if (visualization === 'number') {
    return <NumberReport result={result} metricIndex={metricIndex} formatting={formatting} />
  }
  if (visualization === 'table' || !chart) {
    return <ReportTable result={result} compact={compact} />
  }
  const caption = t('result.chartLabel', { metric: metricLabel(chart.metric, tReports) })
  return (
    <figure className="grid gap-3">
      <div role="img" aria-label={caption}>
        {visualization === 'pie' ? (
          <PieReport
            result={result}
            metricIndex={metricIndex}
            colors={colors}
            height={height}
            formatting={formatting}
            labels={labels}
            otherLabel={t('result.other')}
          />
        ) : (
          <CartesianReport model={chart} kind={visualization} height={height} formatting={formatting} />
        )}
      </div>
      {!compact && (
        <details className="text-sm">
          <summary className="cursor-pointer text-muted-foreground">{t('result.showTable')}</summary>
          <div className="mt-2">
            <ReportTable result={result} compact />
          </div>
        </details>
      )}
    </figure>
  )
}
