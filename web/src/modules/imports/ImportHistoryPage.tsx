import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { fileDownloadUrl } from '@/features/files'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { Pagination } from '@/features/settings/components/Pagination'
import { SectionHeader } from '@/features/settings/components/SectionHeader'
import { HISTORY_PAGE_SIZE, isImportable, isRunning, useImportHistory, type ImportJob } from './api'

type BadgeVariant = 'secondary' | 'destructive' | 'success' | 'warning' | 'outline'

const STATUS_VARIANTS: Record<string, BadgeVariant> = {
  draft: 'outline',
  queued: 'warning',
  running: 'secondary',
  completed: 'success',
  failed: 'destructive',
}

export function ImportHistoryPage() {
  const { t } = useTranslation('imports')
  const formatting = useRecordFormatting()
  const [page, setPage] = useState(1)
  const imports = useImportHistory(page)

  const recordTypeLabel = (recordType: string) =>
    isImportable(recordType) ? t(`recordTypes.${recordType}`) : recordType

  const totalsText = (job: ImportJob) =>
    job.status === 'draft'
      ? t('history.rows', { count: job.row_count })
      : t('history.totals', {
          imported: job.totals.imported,
          updated: job.totals.updated,
          failed: job.totals.failed,
          total: job.totals.total,
        })

  return (
    <section className="flex min-w-0 flex-col gap-4">
      <SectionHeader
        title={t('history.title')}
        description={t('history.description')}
        action={
          <Button asChild>
            <Link to="/imports/new">{t('history.new')}</Link>
          </Button>
        }
      />
      {imports.isPending ? (
        <ListSkeleton />
      ) : imports.isError ? (
        <ErrorState onRetry={() => void imports.refetch()} />
      ) : imports.data.data.length === 0 ? (
        <EmptyState title={t('history.empty')} description={t('history.emptyDescription')} />
      ) : (
        <>
          <div className="relative overflow-x-auto rounded-lg border">
            <table className="w-full text-sm">
              <thead className="bg-muted/50 text-left text-muted-foreground">
                <tr>
                  <th scope="col" className="px-4 py-2 font-medium">
                    {t('history.columns.file')}
                  </th>
                  <th scope="col" className="hidden px-4 py-2 font-medium sm:table-cell">
                    {t('history.columns.recordType')}
                  </th>
                  <th scope="col" className="px-4 py-2 font-medium">
                    {t('history.columns.status')}
                  </th>
                  <th scope="col" className="hidden px-4 py-2 font-medium md:table-cell">
                    {t('history.columns.result')}
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
                {imports.data.data.map((job) => (
                  <tr key={job.id} className="border-t">
                    <td className="max-w-56 truncate px-4 py-2 font-medium">{job.file_name}</td>
                    <td className="hidden px-4 py-2 sm:table-cell">{recordTypeLabel(job.record_type)}</td>
                    <td className="px-4 py-2">
                      <Badge variant={STATUS_VARIANTS[job.status] ?? 'secondary'}>
                        {t(`status.${job.status}`, { defaultValue: job.status })}
                      </Badge>
                    </td>
                    <td className="hidden px-4 py-2 text-muted-foreground md:table-cell">{totalsText(job)}</td>
                    <td className="hidden px-4 py-2 text-muted-foreground lg:table-cell">
                      {formatDateTime(job.created_at, formatting)}
                    </td>
                    <td className="px-4 py-2">
                      <div className="flex justify-end gap-2">
                        {job.error_file_id && (
                          <Button variant="ghost" size="sm" asChild>
                            <a href={fileDownloadUrl(job.error_file_id)} download>
                              {t('history.errorFile')}
                            </a>
                          </Button>
                        )}
                        <Button variant="outline" size="sm" asChild>
                          <Link to={`/imports/${job.id}`}>
                            {job.status === 'draft' || isRunning(job.status) ? t('history.continue') : t('history.open')}
                          </Link>
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <Pagination
            page={page}
            perPage={HISTORY_PAGE_SIZE}
            total={imports.data.meta.total}
            onPageChange={setPage}
          />
        </>
      )}
    </section>
  )
}
