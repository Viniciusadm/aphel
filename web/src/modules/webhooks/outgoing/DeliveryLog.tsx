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
import { Pagination } from '@/components/ui/pagination'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { formatDateTime, ListSkeleton, useRecordFormatting } from '@/sdk'
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
  const formatting = useRecordFormatting()
  const invalidate = useInvalidateWebhooks()
  const [status, setStatus] = useState<DeliveryStatus | null>(null)
  const [page, setPage] = useState(1)
  const [selected, setSelected] = useState<string | null>(null)
  const [sending, setSending] = useState<string | null>(null)
  const deliveries = useWebhookDeliveries(endpoint.id, status, page)

  const redeliver = async (delivery: WebhookDelivery) => {
    setSending(delivery.id)
    try {
      await webhooksApi.redeliver(endpoint.id, delivery.id)
      notifySuccess(t('toast.redelivered'))
      await invalidate()
    } catch (error) {
      notifyError(error)
    } finally {
      setSending(null)
    }
  }

  return (
    <section className="grid gap-3" aria-label={t('deliveries.title')}>
      <p className="text-sm text-muted-foreground">{t('deliveries.description')}</p>
      <SegmentedControl
        aria-label={t('deliveries.filter')}
        className="self-start"
        value={status ?? 'all'}
        onValueChange={(value) => {
          setStatus(isDeliveryStatus(value) ? value : null)
          setPage(1)
        }}
        options={[
          { value: 'all', label: t('deliveries.allStatuses') },
          ...DELIVERY_STATUSES.map((option) => ({ value: option, label: t(`deliveries.status.${option}`) })),
        ]}
      />
      {deliveries.isPending ? (
        <ListSkeleton rows={3} />
      ) : deliveries.isError ? (
        <ErrorState onRetry={() => void deliveries.refetch()} />
      ) : deliveries.data.data.length === 0 ? (
        <p className="py-6 text-center text-sm text-muted-foreground">{t(status ? 'deliveries.empty' : 'deliveries.emptyAll')}</p>
      ) : (
        <Table>
          <caption className="sr-only">{t('deliveries.listLabel')}</caption>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">{t('deliveries.columns.event')}</TableHead>
              <TableHead scope="col">{t('deliveries.columns.status')}</TableHead>
              <TableHead scope="col" className="hidden sm:table-cell">{t('deliveries.columns.origin')}</TableHead>
              <TableHead scope="col" className="hidden md:table-cell">{t('deliveries.columns.response')}</TableHead>
              <TableHead scope="col" className="hidden lg:table-cell">{t('deliveries.columns.attempts')}</TableHead>
              <TableHead scope="col">{t('deliveries.columns.when')}</TableHead>
              <TableHead scope="col" className="text-right">
                <span className="sr-only">{t('deliveries.columns.actions')}</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {deliveries.data.data.map((delivery) => (
              <TableRow key={delivery.id}>
                <TableCell className="max-w-48 truncate font-mono text-xs">{delivery.event_type}</TableCell>
                <TableCell>
                  <DeliveryStatusBadge status={delivery.status} />
                </TableCell>
                <TableCell className="hidden sm:table-cell">
                  <Badge variant="outline">{t(triggerLabelKey(delivery.trigger))}</Badge>
                </TableCell>
                <TableCell className="hidden text-muted-foreground md:table-cell">
                  {delivery.response_status ? `HTTP ${delivery.response_status}` : (delivery.error ?? t('deliveries.noResponse'))}
                  {delivery.duration_ms != null && (
                    <span className="block text-xs">{t('deliveries.duration', { ms: delivery.duration_ms })}</span>
                  )}
                </TableCell>
                <TableCell className="hidden tabular-nums lg:table-cell">
                  {t('deliveries.attempts', { attempts: delivery.attempts, max: delivery.max_attempts })}
                </TableCell>
                <TableCell className="tabular-nums">{formatDateTime(delivery.created_at, formatting)}</TableCell>
                <TableCell className="text-right">
                  <div className="flex items-center justify-end gap-1">
                    {endpoint.is_active && delivery.status !== 'pending' && (
                      <Button
                        size="sm"
                        variant="outline"
                        disabled={sending === delivery.id}
                        aria-label={t('deliveries.redeliverLabel', { event: delivery.event_type })}
                        onClick={() => void redeliver(delivery)}
                      >
                        {t('deliveries.redeliver')}
                      </Button>
                    )}
                    <Button size="sm" variant="ghost" onClick={() => setSelected(delivery.id)}>
                      {t('deliveries.details')}
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
      {deliveries.data && (
        <Pagination page={page} perPage={DELIVERIES_PER_PAGE} total={deliveries.data.meta.total} onPageChange={setPage} />
      )}
      <DeliveryDetailDialog endpointId={endpoint.id} deliveryId={selected} onClose={() => setSelected(null)} />
    </section>
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
