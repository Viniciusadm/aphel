import { Download, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Pagination } from '@/components/ui/pagination'
import { PaginationSummary } from '@/components/ui/pagination-summary'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { fileDownloadUrl, formatDateTime, ListSkeleton, useCustomObjectLabel, useRecordFormatting } from '@/sdk'
import { HISTORY_PAGE_SIZE, isExportable, isRunning, useDeleteExport, useExportHistory, type ExportJob } from './api'

type BadgeVariant = 'secondary' | 'destructive' | 'success' | 'warning'

const STATUS_VARIANTS: Record<string, BadgeVariant> = {
  queued: 'warning',
  running: 'secondary',
  completed: 'success',
  failed: 'destructive',
}

export function ExportHistoryPage() {
  const { t } = useTranslation('exports')
  const [page, setPage] = useState(1)
  const [deleting, setDeleting] = useState<ExportJob | null>(null)
  const exports = useExportHistory(page)
  const remove = useDeleteExport()

  return (
    <Page>
      <PageHeader title={t('history.title')} description={t('history.description')} />
      {exports.isPending ? (
        <ListSkeleton />
      ) : exports.isError ? (
        <ErrorState onRetry={() => void exports.refetch()} />
      ) : exports.data.data.length === 0 ? (
        <EmptyState title={t('history.empty')} description={t('history.emptyDescription')} />
      ) : (
        <>
          <PaginationSummary page={page} perPage={HISTORY_PAGE_SIZE} total={exports.data.meta.total} />
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">{t('history.columns.recordType')}</TableHead>
                <TableHead scope="col" className="hidden sm:table-cell">
                  {t('history.columns.format')}
                </TableHead>
                <TableHead scope="col">{t('history.columns.status')}</TableHead>
                <TableHead scope="col" className="hidden md:table-cell">
                  {t('history.columns.rows')}
                </TableHead>
                <TableHead scope="col" className="hidden lg:table-cell">
                  {t('history.columns.createdAt')}
                </TableHead>
                <TableHead scope="col">
                  <span className="sr-only">{t('history.columns.actions')}</span>
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {exports.data.data.map((job) => (
                <ExportRow key={job.id} job={job} onDelete={() => setDeleting(job)} />
              ))}
            </TableBody>
          </Table>
          <Pagination
            page={page}
            perPage={HISTORY_PAGE_SIZE}
            total={exports.data.meta.total}
            onPageChange={setPage}
          />
        </>
      )}
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={t('history.deleteTitle')}
        description={t('history.deleteDescription')}
        confirmLabel={t('history.delete')}
        variant="destructive"
        onConfirm={async () => {
          if (!deleting) return
          await remove.mutateAsync(deleting.id)
          notifySuccess(t('history.deleted'))
        }}
      />
    </Page>
  )
}

function ExportRow({ job, onDelete }: { job: ExportJob; onDelete: () => void }) {
  const { t } = useTranslation('exports')
  const formatting = useRecordFormatting()
  const customLabel = useCustomObjectLabel(job.record_type)
  const typeLabel = isExportable(job.record_type) ? t(`recordTypes.${job.record_type}`) : (customLabel ?? job.record_type)

  return (
    <TableRow>
      <TableCell className="font-medium">
        <span className="grid gap-0.5">
          <span className="capitalize">{typeLabel}</span>
          {job.status === 'failed' && <span className="text-xs font-normal text-destructive">{t('history.failedHint')}</span>}
        </span>
      </TableCell>
      <TableCell className="hidden sm:table-cell">{t(`formats.${job.format}`, { defaultValue: job.format })}</TableCell>
      <TableCell>
        <Badge variant={STATUS_VARIANTS[job.status] ?? 'secondary'}>
          {t(`history.status.${job.status}`, { defaultValue: job.status })}
        </Badge>
      </TableCell>
      <TableCell className="hidden text-muted-foreground tabular-nums md:table-cell">{job.row_count ?? '—'}</TableCell>
      <TableCell className="hidden text-muted-foreground lg:table-cell">
        {formatDateTime(job.created_at, formatting)}
      </TableCell>
      <TableCell className="text-right">
        <div className="flex justify-end gap-2">
          {job.status === 'completed' && job.file_id && (
            <Button variant="outline" size="sm" asChild>
              <a href={fileDownloadUrl(job.file_id)} download>
                <Download aria-hidden="true" />
                {t('download')}
              </a>
            </Button>
          )}
          {!isRunning(job.status) && (
            <Button variant="ghost" size="sm" onClick={onDelete} aria-label={t('history.deleteLabel')}>
              <Trash2 aria-hidden="true" />
              {t('history.delete')}
            </Button>
          )}
        </div>
      </TableCell>
    </TableRow>
  )
}
