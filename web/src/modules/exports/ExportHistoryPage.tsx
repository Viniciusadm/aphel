import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { fileDownloadUrl } from '@/features/files'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { Pagination } from '@/components/ui/pagination'
import { HISTORY_PAGE_SIZE, isExportable, useExportHistory } from './api'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

type BadgeVariant = 'secondary' | 'destructive' | 'success' | 'warning'

const STATUS_VARIANTS: Record<string, BadgeVariant> = {
  queued: 'warning',
  running: 'secondary',
  completed: 'success',
  failed: 'destructive',
}

export function ExportHistoryPage() {
  const { t } = useTranslation('exports')
  const formatting = useRecordFormatting()
  const [page, setPage] = useState(1)
  const exports = useExportHistory(page)

  const recordTypeLabel = (recordType: string) =>
    isExportable(recordType) ? t(`recordTypes.${recordType}`) : recordType

  return (
    <section className="flex min-w-0 flex-col gap-4">
      <PageHeader title={t('history.title')} description={t('history.description')} />
      {exports.isPending ? (
        <ListSkeleton />
      ) : exports.isError ? (
        <ErrorState onRetry={() => void exports.refetch()} />
      ) : exports.data.data.length === 0 ? (
        <EmptyState title={t('history.empty')} description={t('history.emptyDescription')} />
      ) : (
        <>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">
                  {t('history.columns.recordType')}
                </TableHead>
                <TableHead scope="col" className="hidden sm:table-cell">
                  {t('history.columns.format')}
                </TableHead>
                <TableHead scope="col">
                  {t('history.columns.status')}
                </TableHead>
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
                <TableRow key={job.id}>
                  <TableCell className="font-medium">{recordTypeLabel(job.record_type)}</TableCell>
                  <TableCell className="hidden sm:table-cell">
                    {t(`formats.${job.format}`, { defaultValue: job.format })}
                  </TableCell>
                  <TableCell>
                    <Badge variant={STATUS_VARIANTS[job.status] ?? 'secondary'} title={job.failure_reason ?? undefined}>
                      {t(`history.status.${job.status}`, { defaultValue: job.status })}
                    </Badge>
                  </TableCell>
                  <TableCell className="hidden text-muted-foreground md:table-cell">
                    {job.row_count ?? '—'}
                  </TableCell>
                  <TableCell className="hidden text-muted-foreground lg:table-cell">
                    {formatDateTime(job.created_at, formatting)}
                  </TableCell>
                  <TableCell className="text-right">
                    {job.status === 'completed' && job.file_id && (
                      <Button variant="outline" size="sm" asChild>
                        <a href={fileDownloadUrl(job.file_id)} download>
                          {t('download')}
                        </a>
                      </Button>
                    )}
                  </TableCell>
                </TableRow>
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
    </section>
  )
}
