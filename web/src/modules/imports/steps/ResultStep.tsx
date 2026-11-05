import { PaginationSummary } from '@/components/ui/pagination-summary'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { CircleAlert, CircleCheck, Download, RotateCcw } from 'lucide-react'
import { useEntityLabel } from '@/sdk/entities'
import { fileDownloadUrl, recordListPath } from '@/sdk'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { errorMessage } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Pagination } from '@/components/ui/pagination'
import { ERROR_ROWS_PAGE_SIZE, useImportRows, useRetryFailed, type ImportJob } from '../api'
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
  const typeLabel = useEntityLabel(job.record_type)

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
          {job.privacy_redacted_at ? <CardDescription>{t('result.redacted')}</CardDescription> : job.failure_reason && <CardDescription>{job.failure_reason}</CardDescription>}
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
            {hasFailures && !job.privacy_redacted_at && (
              <Button variant="outline" disabled={retry.isPending} onClick={() => retry.mutate()}>
                <RotateCcw aria-hidden />
                {t('result.retry')}
              </Button>
            )}
            <Button asChild variant="ghost">
              <Link to={recordListPath(job.record_type) ?? '/'}>{t('result.goToList', { recordType: typeLabel })}</Link>
            </Button>
            <Button asChild variant="ghost">
              <Link to={`/imports/new?record_type=${encodeURIComponent(job.record_type)}`}>{t('result.newImport')}</Link>
            </Button>
          </div>
          {retry.isError && (
            <SharedFieldError>
              {errorMessage(retry.error)}
            </SharedFieldError>
          )}
          {hasFailures && <p className="text-xs text-muted-foreground">{t('result.retryHint')}</p>}
        </CardContent>
      </Card>

      {hasFailures && !job.privacy_redacted_at && (
        <>
          <Card>
            <CardHeader>
              <CardTitle>{t('result.errorsTitle')}</CardTitle>
            </CardHeader>
            <CardContent className="grid gap-3">
              {rows.data && <PaginationSummary page={page} perPage={ERROR_ROWS_PAGE_SIZE} total={rows.data.meta.total} />}
              <RowErrors rows={rows.data?.data ?? []} />
              {rows.data && rows.data.meta.total > ERROR_ROWS_PAGE_SIZE && (
                <Pagination
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
