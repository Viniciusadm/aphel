import { LineChart as LineChartIcon, Settings2 } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Bar, BarChart, CartesianGrid, Legend, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatCurrency, formatNumber, useRecordFormatting } from '@/features/records/formatting'
import { NativeSelect } from '@/features/settings/organization/components/NativeSelect'
import { cn } from '@/lib/utils'
import {
  FORECAST_CATEGORIES,
  HORIZON_OPTIONS,
  categoryAmount,
  useForecast,
  type ForecastCategory,
  type ForecastMonth,
  type ForecastReport,
} from './api'
import { ForecastDealsDialog, type DrillDown } from './ForecastDealsDialog'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

function useMonthLabel() {
  const formatting = useRecordFormatting()
  return (startsOn: string, style: 'short' | 'long' = 'short') => {
    const [year, month] = startsOn.split('-').map(Number)
    return new Intl.DateTimeFormat(formatting.locale, {
      timeZone: 'UTC',
      month: style,
      year: style === 'long' ? 'numeric' : '2-digit',
    }).format(new Date(Date.UTC(year, month - 1, 1)))
  }
}

function amountValue(value: string | null | undefined): number {
  const parsed = Number(value ?? 0)
  return Number.isFinite(parsed) ? parsed : 0
}

export function ForecastPage() {
  const { t } = useTranslation('forecast')
  const { hasPermission } = useSession()
  const [months, setMonths] = useState<number | null>(null)
  const report = useForecast(months)
  const [drillDown, setDrillDown] = useState<DrillDown | null>(null)

  let body
  if (report.isPending) {
    body = (
      <div className="grid gap-3" aria-busy="true">
        <Skeleton className="h-24 w-full" />
        <Skeleton className="h-72 w-full" />
      </div>
    )
  } else if (report.isError) {
    const error = report.error
    if (isApiError(error) && error.code === 'field_not_accessible') {
      body = <EmptyState title={t('states.hiddenTitle')} description={t('states.hiddenDescription')} />
    } else if (isApiError(error) && error.status === 403) {
      body = <EmptyState title={t('states.noAccessTitle')} description={t('states.noAccessDescription')} />
    } else {
      body = <ErrorState onRetry={() => void report.refetch()} />
    }
  } else {
    body = (
      <div className={cn('grid min-w-0 gap-4 transition-opacity', report.isFetching && 'opacity-60')}>
        <ForecastContent report={report.data} onDrillDown={setDrillDown} />
      </div>
    )
  }

  return (
    <div className="flex min-w-0 flex-col gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <PageHeader title={t('page.title')} description={t('page.description')} />
        <div className="flex flex-wrap items-center gap-2">
          <label className="flex items-center gap-2 text-sm">
            <span className="shrink-0 text-muted-foreground">{t('page.horizon')}</span>
            <NativeSelect
              className="w-36"
              value={months ?? report.data?.settings.horizon_months ?? ''}
              onChange={(event) => setMonths(Number(event.target.value))}
            >
              {Array.from(new Set([...HORIZON_OPTIONS, report.data?.settings.horizon_months ?? 6]))
                .sort((a, b) => a - b)
                .map((value) => (
                  <option key={value} value={value}>
                    {t('page.months', { count: value })}
                  </option>
                ))}
            </NativeSelect>
          </label>
          {hasPermission('settings.manage') && (
            <Button asChild size="sm" variant="outline">
              <Link to="/settings/forecast">
                <Settings2 aria-hidden="true" />
                {t('page.configure')}
              </Link>
            </Button>
          )}
        </div>
      </div>
      {body}
      <ForecastDealsDialog drillDown={drillDown} onClose={() => setDrillDown(null)} />
    </div>
  )
}

interface ContentProps {
  report: ForecastReport
  onDrillDown: (drillDown: DrillDown) => void
}

function ForecastContent({ report, onDrillDown }: ContentProps) {
  const { t } = useTranslation('forecast')
  const formatting = useRecordFormatting()
  const money = (value: string | null | undefined) => formatCurrency(value ?? 0, formatting, report.currency)
  const totals = report.totals
  const hasOpen = totals.open.count > 0
  return (
    <>
      <p className="text-sm text-muted-foreground">
        {t('page.method', { method: t(`methods.${report.settings.method}`) })}
        {report.probability_source === 'stage' && report.settings.method !== 'stage_probability' && (
          <span> {t('page.stageFallback')}</span>
        )}
      </p>
      <dl className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <Tile label={t('summary.open')} value={money(totals.open.amount)} hint={t('summary.deals', { count: totals.open.count })} />
        <Tile label={t('summary.forecast')} value={money(totals.forecast)} hint={t(`methods.${report.settings.method}`)} />
        <Tile
          label={t('summary.won')}
          value={totals.won ? money(totals.won.amount) : t('summary.unavailable')}
          hint={totals.won ? t('summary.deals', { count: totals.won.count }) : undefined}
        />
      </dl>
      {hasOpen || (totals.won && totals.won.count > 0) ? (
        <ForecastChart report={report} />
      ) : (
        <EmptyState icon={LineChartIcon} title={t('states.emptyTitle')} description={t('states.emptyDescription')} />
      )}
      <ForecastTable report={report} onDrillDown={onDrillDown} />
      <p className="text-xs text-muted-foreground">{t('page.currencyNote', { currency: report.currency })}</p>
    </>
  )
}

function Tile({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div className="flex min-w-0 flex-col gap-1 rounded-lg border bg-card p-3">
      <dt className="truncate text-xs font-medium text-muted-foreground">{label}</dt>
      <dd className="truncate text-xl font-semibold tabular-nums">{value}</dd>
      {hint && <dd className="truncate text-xs text-muted-foreground">{hint}</dd>}
    </div>
  )
}

interface ChartRow {
  label: string
  forecast: number
  open: number
  won: number | null
}

function ForecastChart({ report }: { report: ForecastReport }) {
  const { t } = useTranslation('forecast')
  const formatting = useRecordFormatting()
  const monthLabel = useMonthLabel()
  const rows: ChartRow[] = report.months.map((month) => ({
    label: monthLabel(month.starts_on),
    forecast: amountValue(month.forecast),
    open: amountValue(month.open.amount),
    won: month.won ? amountValue(month.won.amount) : null,
  }))
  const compact = (value: number) =>
    new Intl.NumberFormat(formatting.locale, { notation: 'compact', maximumFractionDigits: 1 }).format(value)
  const showWon = report.totals.won !== null && report.totals.won !== undefined
  return (
    <figure className="grid min-w-0 gap-2 rounded-lg border bg-card p-3">
      <figcaption className="text-sm font-medium">{t('chart.title')}</figcaption>
      <div role="img" aria-label={t('chart.label')} className="h-72 w-full min-w-0">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={rows} margin={{ top: 8, right: 8, bottom: 4, left: 0 }}>
            <CartesianGrid vertical={false} stroke="var(--border)" />
            <XAxis dataKey="label" tickLine={false} axisLine={false} tick={{ fill: 'var(--muted-foreground)', fontSize: 12 }} />
            <YAxis
              width={48}
              tickLine={false}
              axisLine={false}
              tickFormatter={compact}
              tick={{ fill: 'var(--muted-foreground)', fontSize: 12 }}
            />
            <Tooltip
              cursor={{ fill: 'var(--muted)' }}
              formatter={(value) => formatCurrency(Number(value), formatting, report.currency)}
              contentStyle={{
                background: 'var(--popover)',
                border: '1px solid var(--border)',
                borderRadius: 6,
                color: 'var(--popover-foreground)',
                fontSize: 12,
              }}
            />
            <Legend wrapperStyle={{ fontSize: 12 }} />
            <Bar dataKey="open" name={t('chart.open')} fill="var(--muted-foreground)" fillOpacity={0.35} radius={[4, 4, 0, 0]} maxBarSize={28} />
            <Bar dataKey="forecast" name={t('chart.forecast')} fill="var(--primary)" radius={[4, 4, 0, 0]} maxBarSize={28} />
            {showWon && (
              <Bar dataKey="won" name={t('chart.won')} fill="var(--success)" radius={[4, 4, 0, 0]} maxBarSize={28} />
            )}
          </BarChart>
        </ResponsiveContainer>
      </div>
    </figure>
  )
}

function ForecastTable({ report, onDrillDown }: ContentProps) {
  const { t } = useTranslation('forecast')
  const formatting = useRecordFormatting()
  const monthLabel = useMonthLabel()
  const money = (value: string | null | undefined) => formatCurrency(value ?? 0, formatting, report.currency)
  const categories = report.totals.categories ? FORECAST_CATEGORIES : []
  const showWon = report.totals.won !== null && report.totals.won !== undefined

  const open = (month: ForecastMonth, category?: ForecastCategory) =>
    onDrillDown({ month: month.month, label: monthLabel(month.starts_on, 'long'), category })

  return (
    <Table className="min-w-[36rem]">
      <caption className="sr-only">{t('table.caption')}</caption>
      <TableHeader>
        <TableRow>
          <TableHead scope="col">
            {t('table.month')}
          </TableHead>
          <TableHead scope="col" className="text-right">
            {t('table.open')}
          </TableHead>
          <TableHead scope="col" className="text-right">
            {t('table.forecast')}
          </TableHead>
          {categories.map((category) => (
            <TableHead key={category} scope="col" className="text-right">
              {t(`categories.${category}`)}
            </TableHead>
          ))}
          {showWon && (
            <TableHead scope="col" className="text-right">
              {t('table.won')}
            </TableHead>
          )}
        </TableRow>
      </TableHeader>
      <TableBody>
        {report.months.map((month) => (
          <TableRow key={month.month} className="last:border-b-0">
            <TableHead scope="row" className="text-left capitalize">
              {monthLabel(month.starts_on, 'long')}
            </TableHead>
            <TableCell className="text-right tabular-nums">
              <DrillButton
                disabled={month.open.count === 0}
                label={t('table.drillDown', { month: monthLabel(month.starts_on, 'long') })}
                onClick={() => open(month)}
              >
                {money(month.open.amount)}
                <span className="block text-xs text-muted-foreground">
                  {t('summary.deals', { count: month.open.count })}
                </span>
              </DrillButton>
            </TableCell>
            <TableCell className="text-right font-medium tabular-nums">{money(month.forecast)}</TableCell>
            {categories.map((category) => {
              const bucket = categoryAmount(month, category)
              return (
                <TableCell key={category} className="text-right tabular-nums">
                  <DrillButton
                    disabled={!bucket || bucket.count === 0}
                    label={t('table.drillDownCategory', {
                      month: monthLabel(month.starts_on, 'long'),
                      category: t(`categories.${category}`),
                    })}
                    onClick={() => open(month, category)}
                  >
                    {money(bucket?.amount)}
                    <span className="block text-xs text-muted-foreground">
                      {formatNumber(bucket?.count ?? 0, formatting)}
                    </span>
                  </DrillButton>
                </TableCell>
              )
            })}
            {showWon && (
              <TableCell className="text-right tabular-nums text-muted-foreground">{money(month.won?.amount)}</TableCell>
            )}
          </TableRow>
        ))}
      </TableBody>
      <tfoot className="border-t bg-muted/40 font-medium">
        <TableRow>
          <TableHead scope="row" className="text-left">
            {t('table.total')}
          </TableHead>
          <TableCell className="text-right tabular-nums">{money(report.totals.open.amount)}</TableCell>
          <TableCell className="text-right tabular-nums">{money(report.totals.forecast)}</TableCell>
          {categories.map((category) => (
            <TableCell key={category} className="text-right tabular-nums">
              {money(categoryAmount(report.totals, category)?.amount)}
            </TableCell>
          ))}
          {showWon && <TableCell className="text-right tabular-nums">{money(report.totals.won?.amount)}</TableCell>}
        </TableRow>
      </tfoot>
    </Table>
  )
}

interface DrillButtonProps {
  disabled: boolean
  label: string
  onClick: () => void
  children: ReactNode
}

function DrillButton({ disabled, label, onClick, children }: DrillButtonProps) {
  if (disabled) return <span className="text-muted-foreground">{children}</span>
  return (
    <button
      type="button"
      title={label}
      onClick={onClick}
      className="w-full rounded text-right underline-offset-4 hover:underline focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
    >
      {children}
      <span className="sr-only">{label}</span>
    </button>
  )
}
