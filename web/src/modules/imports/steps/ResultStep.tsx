import { CircleAlert, CircleCheck, Download, RotateCcw } from 'lucide-react'
import { useCustomObjectLabel } from '@/features/custom-records/api'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { errorMessage } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { fileDownloadUrl } from '@/features/files'
import { RecordPagination } from '@/features/records/table'
import { ERROR_ROWS_PAGE_SIZE, isImportable, useImportRows, useRetryFailed, type ImportJob } from '../api'
import { LIST_PATHS } from '../mapping'
import { RowErrors } from './RowErrors'
import { UploadStep } from './UploadStep'

const TOTAL_KEYS = ['total', 'imported', 'updated', 'skipped', 'failed'] as const

export interface ResultStepProps {
  job: ImportJob
}

export function ResultStep({ job }: ResultStepProps) {
  const { t } = useTranslation('imports')
  const retry = useRetryFailed(job.id)
  const [page, setPage] = useState(1)
  const hasFailures = job.totals.failed > 0
  const rows = useImportRows(job.id, 'failed', page, hasFailures)
  const completed = job.status === 'completed'
  const customLabel = useCustomObjectLabel(job.record_type)
  const typeLabel = isImportable(job.record_type) ? t(`recordTypes.${job.record_type}`) : (customLabel ?? job.record_type)

  return (
    <div className="grid min-w-0 gap-4">
      <Card>
        <CardHeader>
          <CardTitle className="flex flex-wrap items-center gap-2">
            {completed ? (
              <CircleCheck className="size-5 text-success" aria-hidden />
            ) : (
              <CircleAlert className="size-5 text-destructive" aria-hidden />
            )}
            {completed ? t('result.completed') : t('result.failed')}
            <Badge variant={completed ? 'success' : 'destructive'}>{t(`status.${completed ? 'completed' : 'failed'}`)}</Badge>
          </CardTitle>
          {job.failure_reason && <CardDescription>{job.failure_reason}</CardDescription>}
        </CardHeader>
        <CardContent className="grid gap-4">
          <dl className="grid grid-cols-2 gap-3 sm:grid-cols-5">
            {TOTAL_KEYS.map((key) => (
              <div key={key} className="grid gap-0.5 rounded-md border px-3 py-2">
                <dt className="text-xs text-muted-foreground">{t(`result.totals.${key}`)}</dt>
                <dd className="text-lg font-semibold tabular-nums">{job.totals[key]}</dd>
              </div>
            ))}
          </dl>
          <div className="flex flex-wrap gap-2">
            {job.error_file_id && (
              <Button asChild variant="outline">
                <a href={fileDownloadUrl(job.error_file_id)} download>
                  <Download aria-hidden />
                  {t('result.downloadErrors')}
                </a>
              </Button>
            )}
            {hasFailures && (
              <Button variant="outline" disabled={retry.isPending} onClick={() => retry.mutate()}>
                <RotateCcw aria-hidden />
                {t('result.retry')}
              </Button>
            )}
            <Button asChild variant="ghost">
              <Link to={LIST_PATHS[job.record_type] ?? '/'}>{t('result.goToList', { recordType: typeLabel })}</Link>
            </Button>
            <Button asChild variant="ghost">
              <Link to={`/imports/new?record_type=${encodeURIComponent(job.record_type)}`}>{t('result.newImport')}</Link>
            </Button>
          </div>
          {retry.isError && (
            <p role="alert" className="text-sm text-destructive">
              {errorMessage(retry.error)}
            </p>
          )}
          {hasFailures && <p className="text-xs text-muted-foreground">{t('result.retryHint')}</p>}
        </CardContent>
      </Card>

      {hasFailures && (
        <>
          <Card>
            <CardHeader>
              <CardTitle>{t('result.errorsTitle')}</CardTitle>
            </CardHeader>
            <CardContent className="grid gap-3">
              <RowErrors rows={rows.data?.data ?? []} />
              {rows.data && rows.data.meta.total > ERROR_ROWS_PAGE_SIZE && (
                <RecordPagination
                  page={page}
                  perPage={ERROR_ROWS_PAGE_SIZE}
                  total={rows.data.meta.total}
                  onPageChange={setPage}
                />
              )}
            </CardContent>
          </Card>
          <div className="grid gap-2">
            <p className="text-sm text-muted-foreground">{t('result.fixHint')}</p>
            <UploadStep recordType={job.record_type} retryOf={job.id} />
          </div>
        </>
      )}
    </div>
  )
}
