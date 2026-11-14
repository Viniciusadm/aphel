import { entityLabel, useEntities } from '@/sdk/entities'
import { PaginationSummary } from '@/components/ui/pagination-summary'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { fileDownloadUrl, formatDateTime, ListSkeleton, useRecordFormatting } from '@/sdk'
import { Pagination } from '@/components/ui/pagination'
import {HISTORY_PAGE_SIZE, isRunning, useImportHistory, type ImportJob} from '@/sdk'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'

type BadgeVariant = 'secondary' | 'destructive' | 'success' | 'warning' | 'outline'

const STATUS_VARIANTS: Record<string, BadgeVariant> = {
  draft: 'outline',
  queued: 'warning',
  running: 'secondary',
  completed: 'success',
  failed: 'destructive',
}

export function ImportHistoryPage() {
  const { t, i18n } = useTranslation('imports')
  const entities = useEntities()
  const formatting = useRecordFormatting()
  const [page, setPage] = useState(1)
  const imports = useImportHistory(page)

  const recordTypeLabel = (recordType: string) =>
    entityLabel(entities.data?.find((entity) => entity.record_type === recordType), i18n.language, recordType)

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
    <Page>
      <PageHeader
        title={t('history.title')}
        description={t('history.description')}
        actions={
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
          <PaginationSummary page={page} perPage={HISTORY_PAGE_SIZE} total={imports.data.meta.total} />
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">
                  {t('history.columns.file')}
                </TableHead>
                <TableHead scope="col" className="hidden sm:table-cell">
                  {t('history.columns.recordType')}
                </TableHead>
                <TableHead scope="col">
                  {t('history.columns.status')}
                </TableHead>
                <TableHead scope="col" className="hidden md:table-cell">
                  {t('history.columns.result')}
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
              {imports.data.data.map((job) => (
                <TableRow key={job.id}>
                  <TableCell className="max-w-56 truncate font-medium">{job.file_name}</TableCell>
                  <TableCell className="hidden sm:table-cell">{recordTypeLabel(job.record_type)}</TableCell>
                  <TableCell>
                    <Badge variant={job.status === 'completed' && job.totals.failed > 0 ? 'warning' : (STATUS_VARIANTS[job.status] ?? 'secondary')}>
                      {t(`status.${job.status === 'completed' && job.totals.failed > 0 ? 'completedWithErrors' : job.status}`, { defaultValue: job.status })}
                    </Badge>
                  </TableCell>
                  <TableCell className="hidden text-muted-foreground md:table-cell">{totalsText(job)}</TableCell>
                  <TableCell className="hidden text-muted-foreground lg:table-cell">
                    {formatDateTime(job.created_at, formatting)}
                  </TableCell>
                  <TableCell>
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
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          <Pagination
            page={page}
            perPage={HISTORY_PAGE_SIZE}
            total={imports.data.meta.total}
            onPageChange={setPage}
          />
        </>
      )}
    </Page>
  )
}
