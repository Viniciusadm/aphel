import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { BarChart3 } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/sdk'
import { callsApi, callsKeys } from './api'
import { formatCallDuration } from './lib/format'
import type { CallReport, CallReportQuery } from './types'
import { Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Field } from '@/components/ui/field'

function isoDate(date: Date): string {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

function defaultRange(): { from: string; to: string } {
  const to = new Date()
  const from = new Date(to)
  from.setDate(from.getDate() - 29)
  return { from: isoDate(from), to: isoDate(to) }
}

function rate(connected: number, calls: number): string {
  return calls > 0 ? `${Math.round((connected / calls) * 100)}%` : '—'
}

export function ReportPage() {
  const { t } = useTranslation('telephony')
  const [range, setRange] = useState(defaultRange)
  const [owner, setOwner] = useState<'all' | 'me'>('all')
  const query: CallReportQuery = useMemo(
    () => ({ from: range.from, to: range.to, owner_id: owner === 'me' ? 'me' : undefined }),
    [range, owner],
  )
  const report = useQuery({
    queryKey: callsKeys.report(query),
    queryFn: () => callsApi.report(query),
    placeholderData: keepPreviousData,
    enabled: Boolean(range.from && range.to),
  })

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-end gap-3">
        <Field label={t('report.from')}>
          {(control) => <Input
            {...control}
            type="date"
            value={range.from}
            onChange={(event) => setRange((current) => ({ ...current, from: event.target.value }))}
          />}
        </Field>
        <Field label={t('report.to')}>
          {(control) => <Input
            {...control}
            type="date"
            value={range.to}
            onChange={(event) => setRange((current) => ({ ...current, to: event.target.value }))}
          />}
        </Field>
        <Field label={t('report.owner')}>
          {(control) => <NativeSelect
            {...control}
            value={owner}
            onChange={(event) => setOwner(event.target.value === 'me' ? 'me' : 'all')}
          >
            <option value="all">{t('report.everyone')}</option>
            <option value="me">{t('report.mine')}</option>
          </NativeSelect>}
        </Field>
      </div>
      {report.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-24 w-full" />
          <Skeleton className="h-48 w-full" />
        </div>
      ) : report.isError ? (
        <ErrorState onRetry={() => void report.refetch()} />
      ) : (
        <ReportContent report={report.data} />
      )}
    </div>
  )
}

function ReportContent({ report }: { report: CallReport }) {
  const { t } = useTranslation('telephony')
  const usedKeys = new Set(report.users.flatMap((user) => user.outcomes.map((outcome) => outcome.outcome_key)))
  const columns = report.outcomes.filter((outcome) => outcome.is_active || usedKeys.has(outcome.key))
  const stats = [
    { key: 'calls', label: t('report.calls'), value: String(report.calls) },
    { key: 'connected', label: t('report.connected'), value: String(report.connected) },
    { key: 'rate', label: t('report.rate'), value: rate(report.connected, report.calls) },
    { key: 'duration', label: t('report.duration'), value: formatCallDuration(report.duration_seconds) ?? '0:00' },
  ]
  return (
    <>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {stats.map((stat) => (
          <Card key={stat.key}>
            <CardHeader className="pb-2">
              <CardDescription>{stat.label}</CardDescription>
              <CardTitle className="text-2xl tabular-nums">{stat.value}</CardTitle>
            </CardHeader>
          </Card>
        ))}
      </div>
      {report.users.length === 0 ? (
        <EmptyState icon={BarChart3} title={t('report.emptyTitle')} description={t('report.emptyDescription')} />
      ) : (
        <Card>
          <CardContent className="p-0">
            <Table>
              <TableCaption className="sr-only">{t('report.caption')}</TableCaption>
              <TableHeader>
                <TableRow>
                  <TableHead scope="col">
                    {t('report.user')}
                  </TableHead>
                  {columns.map((outcome) => (
                    <TableHead key={outcome.key} scope="col" className="text-right">
                      {outcome.name}
                    </TableHead>
                  ))}
                  <TableHead scope="col" className="text-right">
                    {t('report.calls')}
                  </TableHead>
                  <TableHead scope="col" className="text-right">
                    {t('report.rate')}
                  </TableHead>
                  <TableHead scope="col" className="text-right">
                    {t('report.duration')}
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {report.users.map((row) => {
                  const counts = new Map(row.outcomes.map((outcome) => [outcome.outcome_key, outcome.calls]))
                  return (
                    <TableRow key={row.user?.id ?? 'none'}>
                      <TableHead scope="row" className="text-left">
                        {row.user?.display_name ?? t('report.noOwner')}
                      </TableHead>
                      {columns.map((outcome) => (
                        <TableCell key={outcome.key} className="text-right tabular-nums">
                          {counts.get(outcome.key) ?? 0}
                        </TableCell>
                      ))}
                      <TableCell className="text-right font-medium tabular-nums">{row.calls}</TableCell>
                      <TableCell className="text-right tabular-nums">{rate(row.connected, row.calls)}</TableCell>
                      <TableCell className="text-right tabular-nums">{formatCallDuration(row.duration_seconds)}</TableCell>
                    </TableRow>
                  )
                })}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}
    </>
  )
}
