import { Filter } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Bar, BarChart, LabelList, Legend, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { ErrorState } from '@/components/ui/error-state'
import { Delta, StatTile } from '@/kernel/widgets/shared'
import { formatCurrency, formatDate, formatNumber, useRecordFormatting } from '@/sdk'
import { cn } from '@/lib/utils'
import {
  type ConversionReport,
  type ConversionSnapshot,
  type ReportStage,
  type StageFunnel,
  useConversionReport,
} from './api'
import { BarList } from './BarList'
import { useReportFilters } from './filters'
import { formatDuration, formatPercent } from './format'
import { ReportFilters } from './ReportFilters'
import { ReportSkeleton } from './ReportState'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

interface Formatters {
  count: (value: number) => string
  money: (value: string | number | null | undefined) => string
  percent: (value: number | null | undefined) => string
  duration: (value: number | null | undefined) => string
}

function useFormatters(currency: string): Formatters {
  const { t } = useTranslation('deals', { keyPrefix: 'reports' })
  const formatting = useRecordFormatting()
  return {
    count: (value) => formatNumber(value, formatting),
    money: (value) => (value === null || value === undefined ? t('states.noData') : formatCurrency(value, formatting, currency)),
    percent: (value) => formatPercent(value, formatting, t('states.noData')),
    duration: (value) => formatDuration(value, formatting, t),
  }
}

function Summary({ report, current, previous }: { report: ConversionReport; current: ConversionSnapshot; previous: ConversionSnapshot | null }) {
  const { t } = useTranslation('deals', { keyPrefix: 'reports' })
  const format = useFormatters(report.currency)
  const summary = current.summary
  const before = previous?.summary
  const percent = (value: number) => format.percent(value)
  const duration = (value: number) => format.duration(value)
  return (
    <dl className="grid grid-cols-1 gap-3 min-[420px]:grid-cols-2 lg:grid-cols-5">
      <StatTile
        label={t('conversion.summary.created')}
        value={format.count(summary.created)}
        hint={report.amounts_visible ? format.money(summary.created_amount) : undefined}
        delta={before && <Delta current={summary.created} previous={before.created} format={format.count} />}
      />
      <StatTile
        label={t('conversion.summary.won')}
        value={format.count(summary.won)}
        hint={report.amounts_visible ? format.money(summary.won_amount) : undefined}
        delta={before && <Delta current={summary.won} previous={before.won} format={format.count} />}
      />
      <StatTile
        label={t('conversion.summary.lost')}
        value={format.count(summary.lost)}
        hint={t('conversion.summary.open', { count: summary.open, value: format.count(summary.open) })}
        delta={before && <Delta current={summary.lost} previous={before.lost} format={format.count} inverse />}
      />
      <StatTile
        label={t('conversion.summary.conversionRate')}
        value={format.percent(summary.conversion_rate)}
        hint={t('conversion.summary.winRate', { value: format.percent(summary.win_rate) })}
        delta={
          before && (
            <Delta current={summary.conversion_rate} previous={before.conversion_rate} format={percent} points />
          )
        }
      />
      <StatTile
        label={t('conversion.summary.averageCycle')}
        value={format.duration(summary.average_cycle_seconds)}
        hint={t('conversion.summary.averageCycleHint')}
        delta={
          before && (
            <Delta
              current={summary.average_cycle_seconds}
              previous={before.average_cycle_seconds}
              format={duration}
              inverse
            />
          )
        }
      />
    </dl>
  )
}

interface FunnelPoint {
  name: string
  reached: number
  previous?: number
}

function FunnelTooltip({ active, payload }: { active?: boolean; payload?: { payload: FunnelPoint }[] }) {
  const { t } = useTranslation('deals', { keyPrefix: 'reports' })
  const point = active ? payload?.[0]?.payload : undefined
  if (!point) return null
  return (
    <div className="rounded-md border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md">
      <p className="font-medium">{point.name}</p>
      <p>{t('conversion.funnel.deals', { count: point.reached })}</p>
      {point.previous !== undefined && (
        <p className="text-muted-foreground">{t('conversion.funnel.previousDeals', { count: point.previous })}</p>
      )}
    </div>
  )
}

function FunnelChart({ points, comparing }: { points: FunnelPoint[]; comparing: boolean }) {
  const { t } = useTranslation('deals', { keyPrefix: 'reports' })
  const height = Math.max(180, points.length * (comparing ? 52 : 40) + 48)
  return (
    <div role="img" aria-label={t('conversion.funnel.chartLabel')} style={{ height }} className="w-full min-w-0">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={points} layout="vertical" margin={{ top: 4, right: 32, bottom: 4, left: 4 }} barCategoryGap={8}>
          <XAxis type="number" hide allowDecimals={false} />
          <YAxis type="category" dataKey="name" width={110} tickLine={false} axisLine={false} tick={{ fill: 'var(--muted-foreground)', fontSize: 12 }} />
          <Tooltip cursor={{ fill: 'var(--muted)' }} content={<FunnelTooltip />} />
          {comparing && <Legend wrapperStyle={{ fontSize: 12 }} />}
          <Bar dataKey="reached" name={t('conversion.funnel.current')} fill="var(--primary)" radius={[0, 4, 4, 0]} maxBarSize={22}>
            <LabelList dataKey="reached" position="right" fill="var(--foreground)" fontSize={12} />
          </Bar>
          {comparing && (
            <Bar
              dataKey="previous"
              name={t('conversion.funnel.previous')}
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

function StageTable({
  stages,
  current,
  previous,
  currency,
  amountsVisible,
}: {
  stages: ReportStage[]
  current: ConversionSnapshot
  previous: ConversionSnapshot | null
  currency: string
  amountsVisible: boolean
}) {
  const { t } = useTranslation('deals', { keyPrefix: 'reports' })
  const format = useFormatters(currency)
  const byId = (snapshot: ConversionSnapshot | null) =>
    new Map<string, StageFunnel>((snapshot?.stages ?? []).map((stage) => [stage.stage_id, stage]))
  const now = byId(current)
  const before = byId(previous)
  const percent = (value: number) => format.percent(value)
  return (
    <Table className="min-w-[640px]">
      <caption className="sr-only">{t('conversion.stages.caption')}</caption>
      <TableHeader>
        <TableRow className="text-left text-xs text-muted-foreground">
          <TableHead scope="col" className="pr-3">{t('conversion.stages.stage')}</TableHead>
          <TableHead scope="col" className="pr-3 text-right">{t('conversion.stages.reached')}</TableHead>
          <TableHead scope="col" className="pr-3 text-right">{t('conversion.stages.passage')}</TableHead>
          <TableHead scope="col" className="pr-3 text-right">{t('conversion.stages.lost')}</TableHead>
          <TableHead scope="col" className="pr-3 text-right">{t('conversion.stages.averageTime')}</TableHead>
          <TableHead scope="col" className="text-right">{t('conversion.stages.openNow')}</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {stages.map((stage) => {
          const row = now.get(stage.id)
          if (!row) return null
          const old = before.get(stage.id)
          return (
            <TableRow key={stage.id} className="last:border-0">
              <TableHead scope="row" className="pr-3 text-left">{stage.name}</TableHead>
              <TableCell className="pr-3 text-right tabular-nums">
                <div>{format.count(row.reached)}</div>
                {old && <div className="text-xs"><Delta current={row.reached} previous={old.reached} format={format.count} /></div>}
              </TableCell>
              <TableCell className="pr-3 text-right tabular-nums">
                <div>{format.percent(row.passage_rate)}</div>
                <div className="text-xs text-muted-foreground">{t('conversion.stages.advanced', { value: format.count(row.advanced) })}</div>
                {old && <div className="text-xs"><Delta current={row.passage_rate} previous={old.passage_rate} format={percent} points /></div>}
              </TableCell>
              <TableCell className="pr-3 text-right tabular-nums">
                <div className={cn(row.lost > 0 && 'text-destructive')}>{format.count(row.lost)}</div>
                {amountsVisible && row.lost > 0 && <div className="text-xs text-muted-foreground">{format.money(row.lost_amount)}</div>}
              </TableCell>
              <TableCell className="pr-3 text-right tabular-nums">
                <div>{format.duration(row.average_seconds)}</div>
                <div className="text-xs text-muted-foreground">{t('conversion.stages.visits', { count: row.completed_visits })}</div>
              </TableCell>
              <TableCell className="text-right tabular-nums">
                <div>{format.count(row.open_visits)}</div>
                {row.open_visits > 0 && (
                  <div className="text-xs text-muted-foreground">
                    {t('conversion.stages.openFor', { value: format.duration(row.open_average_seconds) })}
                  </div>
                )}
              </TableCell>
            </TableRow>
          )
        })}
      </TableBody>
    </Table>
  )
}

function Losses({ report, current }: { report: ConversionReport; current: ConversionSnapshot }) {
  const { t } = useTranslation('deals', { keyPrefix: 'reports' })
  const format = useFormatters(report.currency)
  const stages = new Map((report.pipeline?.stages ?? []).map((stage) => [stage.id, stage.name]))
  const byStage = current.stages
    .filter((stage) => stage.lost > 0)
    .map((stage) => ({
      key: stage.stage_id,
      label: stages.get(stage.stage_id) ?? t('conversion.losses.unknownStage'),
      value: stage.lost,
      display: format.count(stage.lost),
      hint: report.amounts_visible ? format.money(stage.lost_amount) : undefined,
    }))
  if (current.unattributed_lost > 0) {
    byStage.push({
      key: 'unattributed',
      label: t('conversion.losses.unattributed'),
      value: current.unattributed_lost,
      display: format.count(current.unattributed_lost),
      hint: report.amounts_visible ? format.money(current.unattributed_lost_amount) : undefined,
    })
  }
  byStage.sort((left, right) => right.value - left.value)
  const reasons = current.lost_reasons.map((reason, index) => ({
    key: reason.id ?? `none-${index}`,
    label: reason.id ? (reason.name ?? t('conversion.losses.hiddenReason')) : t('conversion.losses.noReason'),
    value: reason.count,
    display: format.count(reason.count),
    hint: report.amounts_visible && reason.amount ? format.money(reason.amount) : undefined,
  }))
  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <Card className="min-w-0 gap-4 py-4">
        <CardHeader className="px-4">
          <CardTitle className="text-base">{t('conversion.losses.byStage')}</CardTitle>
          <CardDescription>{t('conversion.losses.byStageHint')}</CardDescription>
        </CardHeader>
        <CardContent className="px-4">
          {byStage.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t('conversion.losses.none')}</p>
          ) : (
            <BarList items={byStage} label={t('conversion.losses.byStage')} tone="destructive" />
          )}
        </CardContent>
      </Card>
      <Card className="min-w-0 gap-4 py-4">
        <CardHeader className="px-4">
          <CardTitle className="text-base">{t('conversion.losses.byReason')}</CardTitle>
          <CardDescription>{t('conversion.losses.byReasonHint')}</CardDescription>
        </CardHeader>
        <CardContent className="px-4">
          {reasons.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t('conversion.losses.none')}</p>
          ) : (
            <BarList items={reasons} label={t('conversion.losses.byReason')} tone="destructive" />
          )}
        </CardContent>
      </Card>
    </div>
  )
}

function ConversionContent({ report }: { report: ConversionReport }) {
  const { t } = useTranslation('deals', { keyPrefix: 'reports' })
  const formatting = useRecordFormatting()
  const current = report.current
  const pipeline = report.pipeline
  if (!pipeline || !current) {
    return <EmptyState icon={Filter} title={t('conversion.noPipeline.title')} description={t('conversion.noPipeline.description')} />
  }
  const previous = report.previous ?? null
  const openStages = pipeline.stages.filter((stage) => stage.kind === 'open')
  const before = new Map((previous?.stages ?? []).map((stage) => [stage.stage_id, stage]))
  const points: FunnelPoint[] = [
    ...openStages.map((stage) => {
      const row = current.stages.find((item) => item.stage_id === stage.id)
      return {
        name: stage.name,
        reached: row?.reached ?? 0,
        ...(previous ? { previous: before.get(stage.id)?.reached ?? 0 } : {}),
      }
    }),
    {
      name: t('conversion.funnel.won'),
      reached: current.summary.won,
      ...(previous ? { previous: previous.summary.won } : {}),
    },
  ]
  const range = (snapshot: ConversionSnapshot) =>
    t('periodRange', { from: formatDate(snapshot.period.from, formatting), to: formatDate(snapshot.period.to, formatting) })
  return (
    <div className="grid min-w-0 gap-4">
      <p className="text-sm text-muted-foreground">
        {t('conversion.cohort', { pipeline: pipeline.name, range: range(current) })}
        {previous && ` ${t('comparedWith', { range: range(previous) })}`}
      </p>
      {current.summary.created === 0 && current.stages.every((stage) => stage.completed_visits + stage.open_visits === 0) ? (
        <EmptyState title={t('conversion.empty.title')} description={t('conversion.empty.description')} />
      ) : (
        <>
          <Summary report={report} current={current} previous={previous} />
          <Card className="min-w-0 gap-4 py-4">
            <CardHeader className="px-4">
              <CardTitle className="text-base">{t('conversion.funnel.title')}</CardTitle>
              <CardDescription>{t('conversion.funnel.description')}</CardDescription>
            </CardHeader>
            <CardContent className="grid min-w-0 gap-4 px-4">
              <FunnelChart points={points} comparing={previous !== null} />
              <StageTable
                stages={openStages}
                current={current}
                previous={previous}
                currency={report.currency}
                amountsVisible={report.amounts_visible}
              />
            </CardContent>
          </Card>
          <Losses report={report} current={current} />
          <p className="text-xs text-muted-foreground">{t('conversion.footnote')}</p>
        </>
      )}
    </div>
  )
}

export function ConversionReportPage() {
  const { t } = useTranslation('deals', { keyPrefix: 'reports' })
  const state = useReportFilters()
  const { period, pipelineId, compare } = state.filters
  const report = useConversionReport({ period, pipelineId, compare })
  return (
    <Page>
      <PageHeader title={t('conversion.title')} description={t('conversion.description')} />
      <ReportFilters state={state} />
      {report.isPending ? (
        <ReportSkeleton />
      ) : report.isError ? (
        <ErrorState onRetry={() => void report.refetch()} />
      ) : (
        <div className={cn('min-w-0 transition-opacity', report.isFetching && 'opacity-60')}>
          <ConversionContent report={report.data} />
        </div>
      )}
    </Page>
  )
}
