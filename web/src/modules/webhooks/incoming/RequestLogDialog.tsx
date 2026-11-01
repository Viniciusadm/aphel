import { PaginationSummary } from '@/components/ui/pagination-summary'
import { ArrowLeft, Inbox } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { Pagination } from '@/components/ui/pagination'
import {
  REQUESTS_PER_PAGE,
  REQUEST_STATUSES,
  useRequestLog,
  useRequestLogs,
  type ActionOutcome,
  type IncomingWebhook,
  type RequestStatus,
} from './api'

const STATUS_VARIANTS: Record<RequestStatus, 'success' | 'secondary' | 'destructive' | 'outline'> = {
  succeeded: 'success',
  partial: 'secondary',
  failed: 'destructive',
  rejected: 'outline',
}

function StatusBadge({ status }: { status: RequestStatus }) {
  const { t } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  return <Badge variant={STATUS_VARIANTS[status]}>{t(`requestStatus.${status}`)}</Badge>
}

interface RequestLogDialogProps {
  webhook: IncomingWebhook | null
  onClose: () => void
}

export function RequestLogDialog({ webhook, onClose }: RequestLogDialogProps) {
  const { t } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  const [selected, setSelected] = useState<string | null>(null)

  return (
    <Dialog
      open={webhook !== null}
      onOpenChange={(open) => {
        if (!open) {
          setSelected(null)
          onClose()
        }
      }}
    >
      <DialogContent size="2xl">
        <DialogHeader>
          <DialogTitle>{t('log.title', { name: webhook?.name ?? '' })}</DialogTitle>
          <DialogDescription>{t('log.description')}</DialogDescription>
        </DialogHeader>
        {webhook &&
          (selected ? (
            <RequestDetail webhookId={webhook.id} requestId={selected} onBack={() => setSelected(null)} />
          ) : (
            <RequestList webhookId={webhook.id} onSelect={setSelected} />
          ))}
      </DialogContent>
    </Dialog>
  )
}

function RequestList({ webhookId, onSelect }: { webhookId: string; onSelect: (id: string) => void }) {
  const { t } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  const formatting = useRecordFormatting()
  const [status, setStatus] = useState<RequestStatus | null>(null)
  const [page, setPage] = useState(1)
  const requests = useRequestLogs(webhookId, status, page)

  return (
    <div className="grid gap-3">
      <NativeSelect
        aria-label={t('log.statusFilter')}
        className="sm:max-w-xs"
        value={status ?? ''}
        onChange={(event) => {
          setStatus((event.target.value || null) as RequestStatus | null)
          setPage(1)
        }}
      >
        <option value="">{t('log.allStatuses')}</option>
        {REQUEST_STATUSES.map((item) => (
          <option key={item} value={item}>
            {t(`requestStatus.${item}`)}
          </option>
        ))}
      </NativeSelect>
      {requests.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : requests.isError ? (
        <ErrorState onRetry={() => void requests.refetch()} />
      ) : requests.data.data.length === 0 ? (
        <EmptyState icon={Inbox} title={t('log.emptyTitle')} description={t('log.emptyDescription')} />
      ) : (
        <>
          <PaginationSummary page={page} perPage={REQUESTS_PER_PAGE} total={requests.data.meta.total} />
          <ul className="grid gap-2" aria-label={t('log.listLabel')}>
            {requests.data.data.map((request) => (
              <li key={request.id}>
                <button
                  type="button"
                  className="flex w-full flex-col gap-1 rounded-md border bg-card px-3 py-2 text-left hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none sm:flex-row sm:items-center sm:justify-between"
                  onClick={() => onSelect(request.id)}
                >
                  <span className="flex flex-wrap items-center gap-2 text-sm">
                    <StatusBadge status={request.status} />
                    <span>{formatDateTime(request.received_at, formatting)}</span>
                    <code className="text-xs text-muted-foreground">HTTP {request.http_status}</code>
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {request.error_code ?? t('log.actionsSummary', { count: request.results.length })}
                    {request.ip ? ` · ${request.ip}` : ''}
                  </span>
                </button>
              </li>
            ))}
          </ul>
          <Pagination page={page} perPage={REQUESTS_PER_PAGE} total={requests.data.meta.total} onPageChange={setPage} />
        </>
      )}
    </div>
  )
}

function OutcomeRow({ outcome }: { outcome: ActionOutcome }) {
  const { t } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  return (
    <li className="grid gap-1 rounded-md border px-3 py-2 text-sm">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium">
          {t('actions.heading', { position: outcome.index + 1, type: t(`actionTypes.${outcome.type}`) })}
        </span>
        <Badge
          variant={outcome.status === 'succeeded' ? 'success' : outcome.status === 'failed' ? 'destructive' : 'outline'}
        >
          {t(`actionStatus.${outcome.status}`, { defaultValue: outcome.status })}
        </Badge>
        {outcome.operation && (
          <span className="text-xs text-muted-foreground">
            {t(`operation.${outcome.operation}`, { defaultValue: outcome.operation })}
          </span>
        )}
      </div>
      {outcome.record_id && (
        <code className="text-xs text-muted-foreground">
          {outcome.record_type}: {outcome.record_id}
        </code>
      )}
      {outcome.error && (
        <div className="grid gap-1 text-xs text-destructive">
          <span>
            <code>{outcome.error.code}</code> — {outcome.error.message}
          </span>
          {outcome.error.fields.map((field) => (
            <span key={`${field.field}-${field.message}`}>
              <code>{field.field}</code>: {field.message}
            </span>
          ))}
        </div>
      )}
    </li>
  )
}

function RequestDetail({ webhookId, requestId, onBack }: { webhookId: string; requestId: string; onBack: () => void }) {
  const { t } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  const formatting = useRecordFormatting()
  const request = useRequestLog(webhookId, requestId)

  return (
    <div className="grid gap-3">
      <Button type="button" variant="ghost" size="sm" className="self-start" onClick={onBack}>
        <ArrowLeft aria-hidden="true" />
        {t('log.back')}
      </Button>
      {request.isPending ? (
        <Skeleton className="h-40 w-full" />
      ) : request.isError ? (
        <ErrorState onRetry={() => void request.refetch()} />
      ) : (
        <>
          <dl className="grid gap-x-4 gap-y-1 text-sm sm:grid-cols-[auto_minmax(0,1fr)]">
            <dt className="text-muted-foreground">{t('log.status')}</dt>
            <dd>
              <StatusBadge status={request.data.status} />{' '}
              <code className="text-xs">HTTP {request.data.http_status}</code>
            </dd>
            <dt className="text-muted-foreground">{t('log.receivedAt')}</dt>
            <dd>{formatDateTime(request.data.received_at, formatting)}</dd>
            <dt className="text-muted-foreground">{t('log.requestId')}</dt>
            <dd className="font-mono text-xs break-all">{request.data.id}</dd>
            <dt className="text-muted-foreground">{t('log.ip')}</dt>
            <dd>{request.data.ip ?? '—'}</dd>
            <dt className="text-muted-foreground">{t('log.duration')}</dt>
            <dd>{t('log.durationValue', { ms: request.data.duration_ms })}</dd>
            {request.data.error_code && (
              <>
                <dt className="text-muted-foreground">{t('log.error')}</dt>
                <dd className="text-destructive">
                  <code>{request.data.error_code}</code> — {request.data.error_message}
                </dd>
              </>
            )}
          </dl>
          <section className="grid gap-2">
            <h3 className="text-sm font-semibold">{t('log.results')}</h3>
            {request.data.results.length === 0 ? (
              <p className="text-xs text-muted-foreground">{t('log.noResults')}</p>
            ) : (
              <ul className="grid gap-2">
                {request.data.results.map((outcome) => (
                  <OutcomeRow key={outcome.index} outcome={outcome} />
                ))}
              </ul>
            )}
          </section>
          <section className="grid gap-2">
            <h3 className="text-sm font-semibold">{t('log.headers')}</h3>
            <pre className="overflow-x-auto rounded-md border p-3 text-xs">
              {JSON.stringify(request.data.headers, null, 2)}
            </pre>
          </section>
          <section className="grid gap-2">
            <h3 className="text-sm font-semibold">
              {t('log.body', { size: request.data.body_bytes })}
              {request.data.body_truncated ? ` · ${t('log.truncated')}` : ''}
            </h3>
            <pre className="max-h-80 overflow-auto rounded-md border p-3 text-xs whitespace-pre-wrap break-all">
              {request.data.body ?? '—'}
            </pre>
          </section>
        </>
      )}
    </div>
  )
}
