import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useLifecycleReport } from './api'
import { DEFAULT_REPORT_DAYS, LEADS_PATH } from './constants'
import type { LifecycleRecordType, LifecycleReport, LifecycleStageReport } from './types'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Field } from '@/components/ui/field'

function isoDate(date: Date): string {
  return date.toISOString().slice(0, 10)
}

function defaultPeriod(): { from: string; to: string } {
  const to = new Date()
  const from = new Date(to.getTime() - (DEFAULT_REPORT_DAYS - 1) * 86_400_000)
  return { from: isoDate(from), to: isoDate(to) }
}

export function LifecycleReportPage() {
  const { t } = useTranslation('leads')
  const [recordType, setRecordType] = useState<LifecycleRecordType>('contacts.contact')
  const [period, setPeriod] = useState(defaultPeriod)
  const query = useMemo(() => ({ record_type: recordType, ...period }), [recordType, period])
  const report = useLifecycleReport(query)

  return (
    <Page>
      <PageHeader
        back={{ to: LEADS_PATH, label: t('report.back') }}
        title={t('report.title')}
        description={t('report.description')}
      />
      <div className="flex flex-wrap items-end gap-3">
        <Field label={t('report.recordType')}>
          {(control) => <NativeSelect
            {...control}
            value={recordType}
            onChange={(event) => setRecordType(event.target.value as LifecycleRecordType)}
          >
            <option value="contact">{t('report.recordTypes.contact')}</option>
            <option value="company">{t('report.recordTypes.company')}</option>
          </NativeSelect>}
        </Field>
        <Field label={t('report.from')}>
          {(control) => <Input
            {...control}
            type="date"
            value={period.from}
            max={period.to}
            onChange={(event) => event.target.value && setPeriod((current) => ({ ...current, from: event.target.value }))}
          />}
        </Field>
        <Field label={t('report.to')}>
          {(control) => <Input
            {...control}
            type="date"
            value={period.to}
            min={period.from}
            onChange={(event) => event.target.value && setPeriod((current) => ({ ...current, to: event.target.value }))}
          />}
        </Field>
      </div>
      {report.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-40 w-full" />
          <Skeleton className="h-24 w-full" />
        </div>
      ) : report.isError ? (
        <ErrorState onRetry={() => void report.refetch()} />
      ) : (
        <ReportContent report={report.data} />
      )}
    </Page>
  )
}

function ReportContent({ report }: { report: LifecycleReport }) {
  const { t, i18n } = useTranslation('leads')
  const percent = useMemo(
    () => new Intl.NumberFormat(i18n.language, { style: 'percent', maximumFractionDigits: 1 }),
    [i18n.language],
  )
  const decimal = useMemo(() => new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 1 }), [i18n.language])
  const labels = new Map(report.stages.map((stage) => [stage.stage, stage.label]))
  const maxEntered = Math.max(1, ...report.stages.map((stage) => stage.entered))

  if (report.stages.length === 0) return <EmptyState title={t('report.empty')} />

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>{t('report.stagesTitle')}</CardTitle>
          <CardDescription>{t('report.stagesDescription')}</CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t('report.columns.stage')}</TableHead>
                <TableHead className="text-right">{t('report.columns.current')}</TableHead>
                <TableHead>{t('report.columns.entered')}</TableHead>
                <TableHead>{t('report.columns.conversion')}</TableHead>
                <TableHead className="text-right">{t('report.columns.averageDays')}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {report.stages.map((stage) => (
                <StageRow
                  key={stage.stage}
                  stage={stage}
                  nextLabel={stage.next_stage ? (labels.get(stage.next_stage) ?? stage.next_stage) : null}
                  maxEntered={maxEntered}
                  percent={percent}
                  decimal={decimal}
                />
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
      {report.record_type === 'contacts.contact' && (
        <Card>
          <CardHeader>
            <CardTitle>{t('report.disqualificationsTitle')}</CardTitle>
            <CardDescription>{t('report.disqualificationsDescription')}</CardDescription>
          </CardHeader>
          <CardContent>
            {report.disqualifications.length === 0 ? (
              <p className="text-sm text-muted-foreground">{t('report.disqualificationsEmpty')}</p>
            ) : (
              <ul className="grid gap-1 text-sm">
                {report.disqualifications.map((item) => (
                  <li key={item.reason_id ?? item.reason_name} className="flex justify-between gap-3">
                    <span>{item.reason_name}</span>
                    <span className="tabular-nums text-muted-foreground">{item.count}</span>
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>
      )}
    </>
  )
}

function StageRow({
  stage,
  nextLabel,
  maxEntered,
  percent,
  decimal,
}: {
  stage: LifecycleStageReport
  nextLabel: string | null
  maxEntered: number
  percent: Intl.NumberFormat
  decimal: Intl.NumberFormat
}) {
  const { t } = useTranslation('leads')
  const width = `${Math.round((stage.entered / maxEntered) * 100)}%`
  return (
    <TableRow className="align-middle">
      <TableCell>
        <span className="inline-flex items-center gap-2 font-medium">
          <span
            aria-hidden="true"
            className="size-2.5 rounded-full bg-muted-foreground"
            style={stage.color ? { backgroundColor: stage.color } : undefined}
          />
          {stage.label}
        </span>
      </TableCell>
      <TableCell className="text-right tabular-nums">{stage.current}</TableCell>
      <TableCell>
        <div className="flex items-center gap-2">
          <div className="h-2 w-32 overflow-hidden rounded bg-muted" aria-hidden="true">
            <div
              className="h-full rounded bg-primary"
              style={{ width, ...(stage.color ? { backgroundColor: stage.color } : {}) }}
            />
          </div>
          <span className="tabular-nums">{stage.entered}</span>
        </div>
      </TableCell>
      <TableCell>
        {stage.conversion_rate == null || !nextLabel ? (
          <span className="text-muted-foreground">—</span>
        ) : (
          <span>
            {t('report.conversion', {
              rate: percent.format(stage.conversion_rate),
              converted: stage.converted,
              next: nextLabel,
            })}
          </span>
        )}
      </TableCell>
      <TableCell className="text-right tabular-nums">
        {stage.average_days == null ? '—' : t('report.days', { days: decimal.format(stage.average_days) })}
      </TableCell>
    </TableRow>
  )
}
