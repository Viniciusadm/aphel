import { PaginationSummary } from '@/components/ui/pagination-summary'
import { ClipboardList } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { formatDate, ListSkeleton, NativeSelect, useRecordFormatting } from '@/sdk'
import { Pagination } from '@/components/ui/pagination'
import { REQUEST_STATUSES, REQUESTS_PER_PAGE, isRequestStatus, usePrivacyRequests, type PrivacyRequest } from './api'
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
      <div className="flex flex-wrap items-end gap-3">
        <label className="grid gap-1 text-sm">
          <span className="text-muted-foreground">{t('requests.filterLabel')}</span>
          <NativeSelect
            value={status ?? ''}
            onChange={(event) => update({ [STATUS_PARAM]: event.target.value || undefined, [PAGE_PARAM]: undefined })}
          >
            <option value="">{t('requests.allStatuses')}</option>
            {REQUEST_STATUSES.map((value) => (
              <option key={value} value={value}>
                {t(`status.${value}`)}
              </option>
            ))}
          </NativeSelect>
        </label>
      </div>
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
          <ul className="grid gap-2">
            {requests.data.data.map((request) => (
              <RequestRow
                key={request.id}
                request={request}
                dueLabel={formatDate(request.due_at, formatting)}
                onOpen={() => update({ [REQUEST_PARAM]: request.id })}
              />
            ))}
          </ul>
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

interface RequestRowProps {
  request: PrivacyRequest
  dueLabel: string
  onOpen: () => void
}

function RequestRow({ request, dueLabel, onOpen }: RequestRowProps) {
  const { t } = useTranslation('privacy')
  const items = request.targets.records + request.targets.items
  return (
    <li className="flex min-w-0 flex-col gap-2 rounded-lg border bg-card p-3 sm:flex-row sm:items-center sm:justify-between">
      <div className="grid min-w-0 gap-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-medium">{t(`kinds.${request.kind as 'export'}`)}</span>
          <StatusBadge status={request.status} />
          {request.overdue && <Badge variant="destructive">{t('requests.overdue')}</Badge>}
        </div>
        <p className="text-sm break-all text-muted-foreground">
          {t('requests.subject', { label: request.subject_label })} · {t('requests.items', { count: items })}
        </p>
        <p className="text-xs text-muted-foreground">{t('requests.due', { date: dueLabel })}</p>
      </div>
      <Button variant="outline" size="sm" className="self-start sm:self-center" onClick={onOpen}>
        {t('actions.details')}
      </Button>
    </li>
  )
}
