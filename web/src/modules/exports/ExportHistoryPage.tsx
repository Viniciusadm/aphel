import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { fileDownloadUrl } from '@/features/files'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { Pagination } from '@/features/settings/components/Pagination'
import { SectionHeader } from '@/features/settings/components/SectionHeader'
import { HISTORY_PAGE_SIZE, isExportable, useExportHistory } from './api'

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
      <SectionHeader title={t('history.title')} description={t('history.description')} />
      {exports.isPending ? (
        <ListSkeleton />
      ) : exports.isError ? (
        <ErrorState onRetry={() => void exports.refetch()} />
      ) : exports.data.data.length === 0 ? (
        <EmptyState title={t('history.empty')} description={t('history.emptyDescription')} />
      ) : (
        <>
          <div className="relative overflow-x-auto rounded-lg border">
            <table className="w-full text-sm">
              <thead className="bg-muted/50 text-left text-muted-foreground">
                <tr>
                  <th scope="col" className="px-4 py-2 font-medium">
                    {t('history.columns.recordType')}
                  </th>
                  <th scope="col" className="hidden px-4 py-2 font-medium sm:table-cell">
                    {t('history.columns.format')}
                  </th>
                  <th scope="col" className="px-4 py-2 font-medium">
                    {t('history.columns.status')}
                  </th>
                  <th scope="col" className="hidden px-4 py-2 font-medium md:table-cell">
                    {t('history.columns.rows')}
                  </th>
                  <th scope="col" className="hidden px-4 py-2 font-medium lg:table-cell">
                    {t('history.columns.createdAt')}
                  </th>
                  <th scope="col" className="px-4 py-2">
                    <span className="sr-only">{t('history.columns.actions')}</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {exports.data.data.map((job) => (
                  <tr key={job.id} className="border-t">
                    <td className="px-4 py-2 font-medium">{recordTypeLabel(job.record_type)}</td>
                    <td className="hidden px-4 py-2 sm:table-cell">
                      {t(`formats.${job.format}`, { defaultValue: job.format })}
                    </td>
                    <td className="px-4 py-2">
                      <Badge variant={STATUS_VARIANTS[job.status] ?? 'secondary'} title={job.failure_reason ?? undefined}>
                        {t(`history.status.${job.status}`, { defaultValue: job.status })}
                      </Badge>
                    </td>
                    <td className="hidden px-4 py-2 text-muted-foreground md:table-cell">
                      {job.row_count ?? '—'}
                    </td>
                    <td className="hidden px-4 py-2 text-muted-foreground lg:table-cell">
                      {formatDateTime(job.created_at, formatting)}
                    </td>
                    <td className="px-4 py-2 text-right">
                      {job.status === 'completed' && job.file_id && (
                        <Button variant="outline" size="sm" asChild>
                          <a href={fileDownloadUrl(job.file_id)} download>
                            {t('download')}
                          </a>
                        </Button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
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
