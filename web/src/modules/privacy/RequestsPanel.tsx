import { PaginationSummary } from '@/components/ui/pagination-summary'
import { ClipboardList } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { ListSkeleton, useRecordFormatting } from '@/sdk'
import { formatDueAt } from './lib/shared'
import { Pagination } from '@/components/ui/pagination'
import { REQUEST_STATUSES, REQUESTS_PER_PAGE, isRequestStatus, usePrivacyRequests } from './api'
import { RequestDetailDialog } from './RequestDetailDialog'
import { StatusBadge } from './StatusBadge'

const STATUS_PARAM = 'status'
const PAGE_PARAM = 'page'
const REQUEST_PARAM = 'request'

export function RequestsPanel() {
  const { t } = useTranslation('privacy')
  const formatting = useRecordFormatting()
  const [searchParams, setSearchParams] = useSearchParams()
  const statusParam = searchParams.get(STATUS_PARAM)
  const status = isRequestStatus(statusParam) ? statusParam : undefined
  const page = Math.max(1, Number(searchParams.get(PAGE_PARAM)) || 1)
  const openId = searchParams.get(REQUEST_PARAM)
  const requests = usePrivacyRequests(status, page)

  const update = (changes: Record<string, string | undefined>) =>
    setSearchParams((current) => {
      const params = new URLSearchParams(current)
      for (const [key, value] of Object.entries(changes)) {
        if (value) params.set(key, value)
        else params.delete(key)
      }
      return params
    })

  return (
    <div className="grid min-w-0 gap-4">
      <SegmentedControl
        aria-label={t('requests.filterLabel')}
        className="self-start"
        value={status ?? 'all'}
        onValueChange={(value) => update({ [STATUS_PARAM]: value === 'all' ? undefined : value, [PAGE_PARAM]: undefined })}
        options={[
          { value: 'all', label: t('requests.allStatuses') },
          ...REQUEST_STATUSES.map((value) => ({ value, label: t(`status.${value}`) })),
        ]}
      />
      {requests.isPending ? (
        <ListSkeleton />
      ) : requests.isError ? (
        <ErrorState onRetry={() => void requests.refetch()} />
      ) : requests.data.data.length === 0 ? (
        <EmptyState
          icon={ClipboardList}
          title={status ? t('requests.emptyFiltered') : t('requests.empty')}
          description={status ? undefined : t('requests.emptyDescription')}
        />
      ) : (
        <>
          <PaginationSummary page={page} perPage={REQUESTS_PER_PAGE} total={requests.data.meta.total} />
          <Table>
            <caption className="sr-only">{t('requests.listLabel')}</caption>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">{t('requests.columns.kind')}</TableHead>
                <TableHead scope="col">{t('requests.columns.subject')}</TableHead>
                <TableHead scope="col" className="hidden md:table-cell">{t('requests.columns.items')}</TableHead>
                <TableHead scope="col" className="hidden sm:table-cell">{t('requests.columns.due')}</TableHead>
                <TableHead scope="col">{t('requests.columns.status')}</TableHead>
                <TableHead scope="col" className="text-right">
                  <span className="sr-only">{t('requests.columns.actions')}</span>
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {requests.data.data.map((request) => {
                const items = request.targets.records + request.targets.items
                return (
                  <TableRow key={request.id}>
                    <TableCell className="font-medium">{t(`kinds.${request.kind as 'export'}`)}</TableCell>
                    <TableCell className="max-w-56 truncate">{request.subject_label}</TableCell>
                    <TableCell className="hidden tabular-nums md:table-cell">{t('requests.items', { count: items })}</TableCell>
                    <TableCell className="hidden tabular-nums sm:table-cell">{formatDueAt(request.due_at, formatting.locale)}</TableCell>
                    <TableCell>
                      <span className="flex flex-wrap gap-1">
                        <StatusBadge status={request.status} />
                        {request.overdue && <Badge variant="destructive">{t('requests.overdue')}</Badge>}
                      </span>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button variant="outline" size="sm" onClick={() => update({ [REQUEST_PARAM]: request.id })}>
                        {t('actions.details')}
                      </Button>
                    </TableCell>
                  </TableRow>
                )
              })}
            </TableBody>
          </Table>
          <Pagination
            page={page}
            perPage={REQUESTS_PER_PAGE}
            total={requests.data.meta.total}
            onPageChange={(next) => update({ [PAGE_PARAM]: next > 1 ? String(next) : undefined })}
          />
        </>
      )}
      {openId && <RequestDetailDialog id={openId} onClose={() => update({ [REQUEST_PARAM]: undefined })} />}
    </div>
  )
}
