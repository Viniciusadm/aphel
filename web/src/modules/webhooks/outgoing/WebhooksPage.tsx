import { Check, Copy, MoreHorizontal, Plus, Webhook } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import {
  subscribesToAll,
  useInvalidateWebhooks,
  useWebhookEndpoints,
  webhooksApi,
  type WebhookEndpoint,
  type WebhookEndpointWithSecret,
} from './api'
import { DeliveryLog } from './DeliveryLog'
import { EndpointFormDialog } from './EndpointFormDialog'
import { WebhookDeliveryPanel } from './WebhookDeliveryPanel'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

type Editing = { endpoint: WebhookEndpoint | null } | null

export function WebhooksPage() {
  const { t } = useTranslation('webhooks')
  const endpoints = useWebhookEndpoints()
  const invalidate = useInvalidateWebhooks()
  const [editing, setEditing] = useState<Editing>(null)
  const [secret, setSecret] = useState<WebhookEndpointWithSecret | null>(null)
  const [deleting, setDeleting] = useState<WebhookEndpoint | null>(null)
  const [rotating, setRotating] = useState<WebhookEndpoint | null>(null)
  const [expanded, setExpanded] = useState<string | null>(null)

  const remove = async (endpoint: WebhookEndpoint) => {
    await webhooksApi.remove(endpoint.id)
    if (expanded === endpoint.id) setExpanded(null)
    await invalidate()
    notifySuccess(t('toast.deleted'))
  }

  const rotate = async (endpoint: WebhookEndpoint) => {
    const rotated = await webhooksApi.rotateSecret(endpoint.id)
    setSecret(rotated)
    await invalidate()
  }

  return (
    <Page>
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <Button size="sm" onClick={() => setEditing({ endpoint: null })}>
            <Plus aria-hidden="true" />
            {t('new')}
          </Button>
        }
      />
      <p className="text-xs text-muted-foreground">{t('docsHint')}</p>
      {endpoints.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-20 w-full" />
        </div>
      ) : endpoints.isError ? (
        <ErrorState onRetry={() => void endpoints.refetch()} />
      ) : endpoints.data.length === 0 ? (
        <EmptyState
          icon={Webhook}
          title={t('emptyTitle')}
          description={t('emptyDescription')}
          action={
            <Button size="sm" onClick={() => setEditing({ endpoint: null })}>
              <Plus aria-hidden="true" />
              {t('new')}
            </Button>
          }
        />
      ) : (
        <ul className="grid gap-2" aria-label={t('listLabel')}>
          {endpoints.data.map((endpoint) => (
            <EndpointRow
              key={endpoint.id}
              endpoint={endpoint}
              expanded={expanded === endpoint.id}
              onToggleDeliveries={() => setExpanded(expanded === endpoint.id ? null : endpoint.id)}
              onShowDeliveries={() => setExpanded(endpoint.id)}
              onEdit={() => setEditing({ endpoint })}
              onRotate={() => setRotating(endpoint)}
              onDelete={() => setDeleting(endpoint)}
            />
          ))}
        </ul>
      )}
      {editing && (
        <EndpointFormDialog
          endpoint={editing.endpoint}
          onClose={() => setEditing(null)}
          onCreated={(created) => {
            setEditing(null)
            setSecret(created)
            notifySuccess(t('toast.created'))
            void invalidate()
          }}
          onUpdated={() => {
            setEditing(null)
            notifySuccess(t('toast.updated'))
            void invalidate()
          }}
        />
      )}
      <SecretDialog endpoint={secret} onClose={() => setSecret(null)} />
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={t('deleteTitle')}
        description={t('deleteDescription', { url: deleting?.url ?? '' })}
        confirmLabel={t('deleteConfirm')}
        variant="destructive"
        onConfirm={() => (deleting ? remove(deleting) : undefined)}
      />
      <ConfirmDialog
        open={rotating !== null}
        onOpenChange={(open) => !open && setRotating(null)}
        title={t('rotateTitle')}
        description={t('rotateDescription')}
        confirmLabel={t('rotateConfirm')}
        onConfirm={() => (rotating ? rotate(rotating) : undefined)}
      />
      <WebhookDeliveryPanel />
    </Page>
  )
}

interface EndpointRowProps {
  endpoint: WebhookEndpoint
  expanded: boolean
  onToggleDeliveries: () => void
  onShowDeliveries: () => void
  onEdit: () => void
  onRotate: () => void
  onDelete: () => void
}

function EndpointRow({
  endpoint,
  expanded,
  onToggleDeliveries,
  onShowDeliveries,
  onEdit,
  onRotate,
  onDelete,
}: EndpointRowProps) {
  const { t } = useTranslation('webhooks')
  const formatting = useRecordFormatting()
  const invalidate = useInvalidateWebhooks()
  const [busy, setBusy] = useState(false)
  const statusKey = endpoint.is_active
    ? 'status.active'
    : endpoint.disabled_reason === 'failures'
      ? 'status.failures'
      : 'status.manual'

  const run = async (action: () => Promise<unknown>, success: string) => {
    setBusy(true)
    try {
      await action()
      notifySuccess(success)
      await invalidate()
    } catch (error) {
      notifyError(error)
    } finally {
      setBusy(false)
    }
  }

  const sendTest = async () => {
    setBusy(true)
    try {
      const delivery = await webhooksApi.test(endpoint.id)
      if (delivery.status === 'succeeded') {
        notifySuccess(t('toast.testSucceeded', { status: delivery.response_status ?? '' }))
      } else {
        toast({
          title: t('toast.testFailed'),
          description:
            delivery.error ??
            (delivery.response_status ? t('toast.testStatus', { status: delivery.response_status }) : undefined),
          variant: 'destructive',
        })
      }
      onShowDeliveries()
      await invalidate()
    } catch (error) {
      notifyError(error)
    } finally {
      setBusy(false)
    }
  }

  const toggleActive = () =>
    run(
      () => webhooksApi.update(endpoint, { is_active: !endpoint.is_active }),
      t(endpoint.is_active ? 'toast.disabled' : 'toast.enabled'),
    )

  return (
    <li className="grid gap-3 rounded-md border bg-card px-3 py-2">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid min-w-0 gap-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-mono text-sm break-all">{endpoint.url}</span>
            <Badge
              variant={endpoint.is_active ? 'success' : endpoint.disabled_reason === 'failures' ? 'destructive' : 'outline'}
            >
              {t(statusKey)}
            </Badge>
          </div>
          {endpoint.description && <p className="text-sm text-muted-foreground break-words">{endpoint.description}</p>}
          <div className="flex flex-wrap gap-1">
            {subscribesToAll(endpoint) ? (
              <Badge variant="secondary">{t('allEvents')}</Badge>
            ) : (
              <Badge variant="secondary" title={endpoint.events.join(', ')}>
                {t('eventCount', { count: endpoint.events.length })}
              </Badge>
            )}
            {endpoint.consecutive_failures > 0 && (
              <Badge variant="warning">{t('failures', { count: endpoint.consecutive_failures })}</Badge>
            )}
            {!endpoint.secret_configured && <Badge variant="outline">{t('noSecret')}</Badge>}
          </div>
          <p className="text-xs text-muted-foreground">
            {endpoint.last_success_at
              ? t('lastSuccess', { date: formatDateTime(endpoint.last_success_at, formatting) })
              : t('neverDelivered')}
          </p>
          {!endpoint.is_active && endpoint.disabled_reason === 'failures' && endpoint.disabled_at && (
            <p className="text-sm text-destructive">
              {t('disabledByFailures', { date: formatDateTime(endpoint.disabled_at, formatting) })}
            </p>
          )}
        </div>
        <div className="flex gap-2 self-start">
          <Button size="sm" variant="outline" onClick={onToggleDeliveries} aria-expanded={expanded}>
            {t(expanded ? 'actions.hideDeliveries' : 'actions.deliveries')}
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button size="icon" variant="ghost" className="size-8" disabled={busy} aria-label={t('actions.menu', { url: endpoint.url })}>
                <MoreHorizontal aria-hidden="true" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onSelect={onEdit}>{t('actions.edit')}</DropdownMenuItem>
              <DropdownMenuItem disabled={!endpoint.is_active} onSelect={() => void sendTest()}>
                {t('actions.test')}
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => void toggleActive()}>
                {t(endpoint.is_active ? 'actions.disable' : 'actions.enable')}
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={onRotate}>{t('actions.rotate')}</DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem variant="destructive" onSelect={onDelete}>
                {t('actions.delete')}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
      {expanded && <DeliveryLog endpoint={endpoint} />}
    </li>
  )
}

function SecretDialog({ endpoint, onClose }: { endpoint: WebhookEndpointWithSecret | null; onClose: () => void }) {
  const { t } = useTranslation('webhooks')
  const [copied, setCopied] = useState(false)

  const copy = async () => {
    if (!endpoint) return
    try {
      await navigator.clipboard.writeText(endpoint.signing_secret)
      setCopied(true)
    } catch {
      notifyError(new Error(t('secret.copyFailed')))
    }
  }

  return (
    <Dialog
      open={endpoint !== null}
      onOpenChange={(open) => {
        if (!open) {
          setCopied(false)
          onClose()
        }
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('secret.title')}</DialogTitle>
          <DialogDescription>{t('secret.description')}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-2">
          <p className="font-mono text-xs break-all text-muted-foreground">{endpoint?.url}</p>
          <code data-testid="webhook-signing-secret" className="block rounded-md bg-muted p-3 text-xs break-all">
            {endpoint?.signing_secret}
          </code>
          <Button type="button" variant="outline" size="sm" className="self-start" onClick={() => void copy()}>
            {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
            {copied ? t('secret.copied') : t('secret.copy')}
          </Button>
        </div>
        <DialogFooter>
          <Button
            type="button"
            onClick={() => {
              setCopied(false)
              onClose()
            }}
          >
            {t('secret.done')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
