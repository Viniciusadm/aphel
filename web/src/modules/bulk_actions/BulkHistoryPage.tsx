import { PaginationSummary } from '@/components/ui/pagination-summary'
import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { NativeSelect } from '@/components/ui/native-select'
import { formatDateTime, ListSkeleton, useRecordFormatting } from '@/sdk'
import { Pagination } from '@/components/ui/pagination'
import { BulkActionSummary } from './BulkActionSummary'
import { BulkResult } from './BulkResult'
import { HISTORY_PAGE_SIZE, bulkApi, bulkKeys, useBulkHistory, type BulkOperation } from './api'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'

type BadgeVariant = 'secondary' | 'destructive' | 'success' | 'warning'

const STATUS_VARIANTS: Record<string, BadgeVariant> = {
  queued: 'warning',
  running: 'secondary',
  completed: 'success',
  failed: 'destructive',
}

const STATUSES = ['queued', 'running', 'completed', 'failed'] as const
const OPERATION_PARAM = 'operation'
const ALL = '__all__'

export function BulkHistoryPage() {
  const { t } = useTranslation('bulk_actions')
  const formatting = useRecordFormatting()
  const [page, setPage] = useState(1)
  const [status, setStatus] = useState('')
  const [searchParams, setSearchParams] = useSearchParams()
  const history = useBulkHistory(page, status)
  const selectedId = searchParams.get(OPERATION_PARAM)
  const selected = useQuery({
    queryKey: bulkKeys.detail(selectedId ?? ''),
    queryFn: () => bulkApi.get(selectedId ?? ''),
    enabled: Boolean(selectedId),
  })

  const select = (operation: BulkOperation | null) => {
    const next = new URLSearchParams(searchParams)
    if (operation) next.set(OPERATION_PARAM, operation.id)
    else next.delete(OPERATION_PARAM)
    setSearchParams(next, { replace: true })
  }

  const recordTypeLabel = (recordType: string) =>
    t(`history.recordTypes.${recordType}`, { defaultValue: recordType })
  const actionLabel = (action: string) => t(`history.actionNames.${action}`, { defaultValue: action })
  const filtered = status !== ''

  return (
    <Page>
      <PageHeader title={t('history.title')} description={t('history.description')} />
      <div className="flex flex-wrap gap-2">
        <NativeSelect
          value={status || ALL}
          onValueChange={(value) => {
            setStatus(value === ALL ? '' : value)
            setPage(1)
          }}
          className="w-52"
          aria-label={t('history.columns.status')}
        >
          <option value={ALL}>{t('history.allStatuses')}</option>
          {STATUSES.map((value) => (
            <option key={value} value={value}>
              {t(`history.status.${value}`)}
            </option>
          ))}
        </NativeSelect>
      </div>
      {history.isPending ? (
        <ListSkeleton />
      ) : history.isError ? (
        <ErrorState onRetry={() => void history.refetch()} />
      ) : history.data.data.length === 0 ? (
        <EmptyState
          title={filtered ? t('history.emptyFiltered') : t('history.empty')}
          description={filtered ? t('history.emptyFilteredDescription') : t('history.emptyDescription')}
        />
      ) : (
        <>
          <PaginationSummary page={page} perPage={HISTORY_PAGE_SIZE} total={history.data.meta.total} />
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">{t('history.columns.action')}</TableHead>
                <TableHead scope="col" className="hidden sm:table-cell">
                  {t('history.columns.recordType')}
                </TableHead>
                <TableHead scope="col">{t('history.columns.status')}</TableHead>
                <TableHead scope="col" className="hidden md:table-cell">
                  {t('history.columns.records')}
                </TableHead>
                <TableHead scope="col" className="hidden lg:table-cell">
                  {t('history.columns.createdAt')}
                </TableHead>
                <TableHead scope="col">
                  <span className="sr-only">{t('history.columns.details')}</span>
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {history.data.data.map((operation) => (
                <TableRow key={operation.id} className="cursor-pointer" onClick={() => select(operation)}>
                  <TableCell className="grid gap-0.5">
                    <button
                      type="button"
                      className="w-fit text-left font-medium underline-offset-4 hover:underline"
                      onClick={(event) => {
                        event.stopPropagation()
                        select(operation)
                      }}
                    >
                      {actionLabel(operation.action)}
                    </button>
                    <BulkActionSummary operation={operation} />
                  </TableCell>
                  <TableCell className="hidden sm:table-cell">{recordTypeLabel(operation.record_type)}</TableCell>
                  <TableCell>
                    <Badge variant={STATUS_VARIANTS[operation.status] ?? 'secondary'}>
                      {t(`history.status.${operation.status}`, { defaultValue: operation.status })}
                    </Badge>
                  </TableCell>
                  <TableCell className="hidden text-muted-foreground tabular-nums md:table-cell">
                    {t('history.counts', {
                      succeeded: operation.succeeded,
                      issues: operation.skipped + operation.failed,
                      total: operation.total,
                    })}
                  </TableCell>
                  <TableCell className="hidden text-muted-foreground lg:table-cell">
                    {formatDateTime(operation.created_at, formatting)}
                  </TableCell>
                  <TableCell className="text-right">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={(event) => {
                        event.stopPropagation()
                        select(operation)
                      }}
                    >
                      {t('history.open')}
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          <Pagination
            page={page}
            perPage={HISTORY_PAGE_SIZE}
            total={history.data.meta.total}
            onPageChange={setPage}
          />
        </>
      )}
      <Dialog open={Boolean(selectedId)} onOpenChange={(open) => (open ? undefined : select(null))}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {selected.data ? actionLabel(selected.data.action) : t('history.detailTitle')}
            </DialogTitle>
            <DialogDescription>
              {selected.data
                ? `${recordTypeLabel(selected.data.record_type)} · ${formatDateTime(selected.data.created_at, formatting)}`
                : t('history.detailTitle')}
            </DialogDescription>
          </DialogHeader>
          {selected.isPending ? (
            <ListSkeleton />
          ) : selected.isError ? (
            <ErrorState onRetry={() => void selected.refetch()} />
          ) : (
            <>
              <BulkActionSummary operation={selected.data} className="text-sm" />
              <BulkResult key={selected.data.id} operation={selected.data} recordType={selected.data.record_type} />
            </>
          )}
        </DialogContent>
      </Dialog>
    </Page>
  )
}
