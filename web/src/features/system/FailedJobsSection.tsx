import { RotateCcw, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Pagination } from '@/components/ui/pagination'
import { NativeSelect } from '@/features/settings/organization/components/NativeSelect'
import { cn } from '@/lib/utils'
import { FAILED_JOBS_PER_PAGE, useFailedJobs, useJobAction, useJobStats } from './api'
import { useDateTimeFormat } from './format'
import { SectionCard } from './SectionCard'
import type { FailedJob } from './types'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

export function FailedJobsSection() {
  const { t } = useTranslation('system')
  const format = useDateTimeFormat()
  const [kind, setKind] = useState('')
  const [page, setPage] = useState(1)
  const [discarding, setDiscarding] = useState<FailedJob | null>(null)
  const stats = useJobStats()
  const jobs = useFailedJobs(kind, page)
  const retry = useJobAction('retry')
  const discard = useJobAction('discard')
  const failedKinds = (stats.data?.kinds ?? []).filter((entry) => entry.failed > 0).map((entry) => entry.kind)
  const kindOptions = Array.from(new Set(kind ? [...failedKinds, kind] : failedKinds)).sort()

  const runRetry = (job: FailedJob) => {
    retry.mutate(job.id, {
      onSuccess: () => notifySuccess(t('failedJobs.retried')),
      onError: (error) => notifyError(error),
    })
  }

  const confirmDiscard = async () => {
    if (!discarding) return
    await discard.mutateAsync(discarding.id)
    notifySuccess(t('failedJobs.discarded'))
  }

  let body
  if (jobs.isPending) {
    body = <Skeleton className="h-32 w-full" />
  } else if (jobs.isError) {
    body = <ErrorState onRetry={() => void jobs.refetch()} />
  } else if (jobs.data.data.length === 0) {
    body = <p className="text-sm text-muted-foreground">{t('failedJobs.empty')}</p>
  } else {
    body = (
      <>
        <div className={cn('relative overflow-x-auto rounded-lg border', jobs.isFetching && 'opacity-60')}>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">
                  {t('failedJobs.columns.kind')}
                </TableHead>
                <TableHead scope="col">
                  {t('failedJobs.columns.error')}
                </TableHead>
                <TableHead scope="col" className="hidden whitespace-nowrap sm:table-cell">
                  {t('failedJobs.columns.failedAt')}
                </TableHead>
                <TableHead scope="col" className="text-right">
                  {t('failedJobs.columns.actions')}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {jobs.data.data.map((job) => (
                <TableRow key={job.id} className="align-top">
                  <TableCell>
                    <div className="grid gap-0.5">
                      <span className="font-mono text-xs break-all">{job.kind}</span>
                      <span className="text-xs text-muted-foreground">
                        {t('failedJobs.attempts', { attempts: job.attempts, max: job.max_attempts })}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell className="max-w-xs">
                    <span className="line-clamp-3 text-xs break-words text-muted-foreground" title={job.last_error ?? undefined}>
                      {job.last_error ?? '—'}
                    </span>
                  </TableCell>
                  <TableCell className="hidden whitespace-nowrap text-muted-foreground sm:table-cell">
                    <time dateTime={job.failed_at}>{format.format(new Date(job.failed_at))}</time>
                  </TableCell>
                  <TableCell>
                    <div className="flex justify-end gap-1">
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={retry.isPending && retry.variables === job.id}
                        onClick={() => runRetry(job)}
                      >
                        <RotateCcw aria-hidden="true" />
                        <span className="sr-only sm:not-sr-only">{t('failedJobs.retry')}</span>
                      </Button>
                      <Button variant="ghost" size="sm" onClick={() => setDiscarding(job)}>
                        <Trash2 aria-hidden="true" />
                        <span className="sr-only sm:not-sr-only">{t('failedJobs.discard')}</span>
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
        <Pagination page={page} perPage={FAILED_JOBS_PER_PAGE} total={jobs.data.meta.total} onPageChange={setPage} />
      </>
    )
  }

  return (
    <SectionCard
      title={t('failedJobs.title')}
      description={t('failedJobs.description')}
      action={
        <label className="flex min-w-0 items-center gap-2 text-sm">
          <span className="shrink-0 text-muted-foreground">{t('failedJobs.filter')}</span>
          <NativeSelect
            className="sm:w-56"
            value={kind}
            onChange={(event) => {
              setKind(event.target.value)
              setPage(1)
            }}
          >
            <option value="">{t('failedJobs.allKinds')}</option>
            {kindOptions.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </NativeSelect>
        </label>
      }
    >
      {body}
      <ConfirmDialog
        open={discarding !== null}
        onOpenChange={(open) => !open && setDiscarding(null)}
        title={t('failedJobs.discardTitle')}
        description={discarding ? t('failedJobs.discardDescription', { kind: discarding.kind }) : undefined}
        confirmLabel={t('failedJobs.discard')}
        variant="destructive"
        pending={discard.isPending}
        notifyOnError
        onConfirm={confirmDiscard}
      />
    </SectionCard>
  )
}
