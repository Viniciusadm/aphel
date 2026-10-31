import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import {
  DELIVERIES_PER_PAGE,
  DELIVERY_STATUSES,
  isDeliveryStatus,
  useInvalidateWebhooks,
  useWebhookDeliveries,
  useWebhookDelivery,
  webhooksApi,
  type DeliveryStatus,
  type WebhookDelivery,
  type WebhookEndpoint,
} from './api'
import { NativeSelect } from '@/components/ui/native-select'

const STATUS_VARIANTS: Record<DeliveryStatus, BadgeProps['variant']> = {
  pending: 'secondary',
  succeeded: 'success',
  failed: 'destructive',
}

function DeliveryStatusBadge({ status }: { status: string }) {
  const { t } = useTranslation('webhooks')
  if (!isDeliveryStatus(status)) return <Badge variant="outline">{status}</Badge>
  return <Badge variant={STATUS_VARIANTS[status]}>{t(`deliveries.status.${status}`)}</Badge>
}

function triggerLabelKey(trigger: string) {
  switch (trigger) {
    case 'test':
      return 'deliveries.trigger.test' as const
    case 'redelivery':
      return 'deliveries.trigger.redelivery' as const
    default:
      return 'deliveries.trigger.event' as const
  }
}

interface DeliveryLogProps {
  endpoint: WebhookEndpoint
}

export function DeliveryLog({ endpoint }: DeliveryLogProps) {
  const { t } = useTranslation('webhooks')
  const [status, setStatus] = useState<DeliveryStatus | null>(null)
  const [page, setPage] = useState(1)
  const [selected, setSelected] = useState<string | null>(null)
  const deliveries = useWebhookDeliveries(endpoint.id, status, page)
  const pages = Math.max(1, Math.ceil((deliveries.data?.meta.total ?? 0) / DELIVERIES_PER_PAGE))

  return (
    <section className="grid gap-3 rounded-md border bg-muted/20 p-3" aria-label={t('deliveries.title')}>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
        <div className="grid gap-0.5">
          <h3 className="text-sm font-semibold">{t('deliveries.title')}</h3>
          <p className="text-xs text-muted-foreground">{t('deliveries.description')}</p>
        </div>
        <label className="flex items-center gap-2 text-sm">
          {t('deliveries.filter')}
          <NativeSelect
            value={status ?? ''}
            onChange={(event) => {
              setStatus(isDeliveryStatus(event.target.value) ? event.target.value : null)
              setPage(1)
            }}
          >
            <option value="">{t('deliveries.allStatuses')}</option>
            {DELIVERY_STATUSES.map((option) => (
              <option key={option} value={option}>
                {t(`deliveries.status.${option}`)}
              </option>
            ))}
          </NativeSelect>
        </label>
      </div>
      {deliveries.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : deliveries.isError ? (
        <ErrorState onRetry={() => void deliveries.refetch()} />
      ) : deliveries.data.data.length === 0 ? (
        <p className="py-4 text-center text-sm text-muted-foreground">{t('deliveries.empty')}</p>
      ) : (
        <ul className="grid gap-1.5" aria-label={t('deliveries.listLabel')}>
          {deliveries.data.data.map((delivery) => (
            <DeliveryRow
              key={delivery.id}
              endpoint={endpoint}
              delivery={delivery}
              onOpen={() => setSelected(delivery.id)}
            />
          ))}
        </ul>
      )}
      {pages > 1 && (
        <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
          <span>{t('deliveries.page', { page, pages })}</span>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>
              {t('deliveries.previous')}
            </Button>
            <Button variant="outline" size="sm" disabled={page >= pages} onClick={() => setPage(page + 1)}>
              {t('deliveries.next')}
            </Button>
          </div>
        </div>
      )}
      <DeliveryDetailDialog endpointId={endpoint.id} deliveryId={selected} onClose={() => setSelected(null)} />
    </section>
  )
}

interface DeliveryRowProps {
  endpoint: WebhookEndpoint
  delivery: WebhookDelivery
  onOpen: () => void
}

function DeliveryRow({ endpoint, delivery, onOpen }: DeliveryRowProps) {
  const { t } = useTranslation('webhooks')
  const formatting = useRecordFormatting()
  const invalidate = useInvalidateWebhooks()
  const [sending, setSending] = useState(false)

  const redeliver = async () => {
    setSending(true)
    try {
      await webhooksApi.redeliver(endpoint.id, delivery.id)
      notifySuccess(t('toast.redelivered'))
      await invalidate()
    } catch (error) {
      notifyError(error)
    } finally {
      setSending(false)
    }
  }

  return (
    <li className="flex flex-col gap-2 rounded-md border bg-card px-3 py-2 sm:flex-row sm:items-center sm:justify-between">
      <div className="grid min-w-0 gap-0.5">
        <div className="flex flex-wrap items-center gap-2">
          <DeliveryStatusBadge status={delivery.status} />
          <code className="font-mono text-xs break-all">{delivery.event_type}</code>
          <Badge variant="outline">{t(triggerLabelKey(delivery.trigger))}</Badge>
        </div>
        <p className="text-xs text-muted-foreground">
          {formatDateTime(delivery.created_at, formatting)}
          {' · '}
          {delivery.response_status ? `HTTP ${delivery.response_status}` : (delivery.error ?? t('deliveries.noResponse'))}
          {delivery.duration_ms != null ? ` · ${t('deliveries.duration', { ms: delivery.duration_ms })}` : ''}
          {' · '}
          {t('deliveries.attempts', { attempts: delivery.attempts, max: delivery.max_attempts })}
          {delivery.status === 'pending' && delivery.next_attempt_at && delivery.attempts > 0
            ? ` · ${t('deliveries.nextAttempt', { date: formatDateTime(delivery.next_attempt_at, formatting) })}`
            : ''}
        </p>
      </div>
      <div className="flex gap-2 self-start sm:self-center">
        <Button size="sm" variant="ghost" onClick={onOpen}>
          {t('deliveries.details')}
        </Button>
        {endpoint.is_active && delivery.status !== 'pending' && (
          <Button
            size="sm"
            variant="outline"
            disabled={sending}
            aria-label={t('deliveries.redeliverLabel', { event: delivery.event_type })}
            onClick={() => void redeliver()}
          >
            {t('deliveries.redeliver')}
          </Button>
        )}
      </div>
    </li>
  )
}

interface DeliveryDetailDialogProps {
  endpointId: string
  deliveryId: string | null
  onClose: () => void
}

function DeliveryDetailDialog({ endpointId, deliveryId, onClose }: DeliveryDetailDialogProps) {
  const { t } = useTranslation('webhooks')
  const formatting = useRecordFormatting()
  const detail = useWebhookDelivery(endpointId, deliveryId)

  return (
    <Dialog open={deliveryId !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="xl">
        <DialogHeader>
          <DialogTitle>{t('detail.title', { event: detail.data?.event_type ?? '' })}</DialogTitle>
        </DialogHeader>
        {detail.isPending ? (
          <Skeleton className="h-48 w-full" />
        ) : detail.isError ? (
          <ErrorState onRetry={() => void detail.refetch()} />
        ) : (
          <div className="grid gap-4 text-sm">
            <dl className="grid gap-x-4 gap-y-1 sm:grid-cols-[auto_1fr]">
              <dt className="text-muted-foreground">{t('deliveries.filter')}</dt>
              <dd>
                <DeliveryStatusBadge status={detail.data.status} />
              </dd>
              <dt className="text-muted-foreground">{t('detail.deliveryId')}</dt>
              <dd className="font-mono text-xs break-all">{detail.data.id}</dd>
              <dt className="text-muted-foreground">{t('detail.eventId')}</dt>
              <dd className="font-mono text-xs break-all">{detail.data.event_id}</dd>
              <dt className="text-muted-foreground">{t('detail.createdAt')}</dt>
              <dd>{formatDateTime(detail.data.created_at, formatting)}</dd>
              {detail.data.last_attempt_at && (
                <>
                  <dt className="text-muted-foreground">{t('detail.lastAttempt')}</dt>
                  <dd>{formatDateTime(detail.data.last_attempt_at, formatting)}</dd>
                </>
              )}
              {detail.data.response_status != null && (
                <>
                  <dt className="text-muted-foreground">{t('detail.responseStatus')}</dt>
                  <dd>{detail.data.response_status}</dd>
                </>
              )}
              {detail.data.error && (
                <>
                  <dt className="text-muted-foreground">{t('detail.error')}</dt>
                  <dd className="break-words text-destructive">{detail.data.error}</dd>
                </>
              )}
            </dl>
            <div className="grid gap-1">
              <h4 className="font-medium">{t('detail.requestBody')}</h4>
              <pre className="max-h-64 overflow-auto rounded-md border bg-muted p-3 text-xs">
                {JSON.stringify(detail.data.request_body, null, 2)}
              </pre>
            </div>
            {detail.data.response_body && (
              <div className="grid gap-1">
                <h4 className="font-medium">{t('detail.response')}</h4>
                <pre className="max-h-48 overflow-auto rounded-md border bg-muted p-3 text-xs break-all whitespace-pre-wrap">
                  {detail.data.response_body}
                </pre>
              </div>
            )}
          </div>
        )}
        <DialogFooter>
          <Button type="button" onClick={onClose}>
            {t('detail.close')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
