import { Check, Copy, MoreHorizontal, Plus, Webhook } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
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
import { ListSkeleton } from '@/sdk'
import { toast } from '@/components/ui/toast-store'
import { formatDateTime, useRecordFormatting } from '@/sdk'
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
  const [viewing, setViewing] = useState<WebhookEndpoint | null>(null)

  const remove = async (endpoint: WebhookEndpoint) => {
    await webhooksApi.remove(endpoint.id)
    if (viewing?.id === endpoint.id) setViewing(null)
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
        description={
          <>
            {t('description')}{' '}
            <a className="underline underline-offset-4" href="/_aphel/docs/reference/webhooks-and-events/" target="_blank" rel="noreferrer">
              {t('docsLink')}
            </a>
          </>
        }
        actions={
          <Button onClick={() => setEditing({ endpoint: null })}>
            <Plus aria-hidden="true" />
            {t('new')}
          </Button>
        }
      />
      {endpoints.isPending ? (
        <ListSkeleton rows={3} />
      ) : endpoints.isError ? (
        <ErrorState onRetry={() => void endpoints.refetch()} />
      ) : endpoints.data.length === 0 ? (
        <EmptyState
          icon={Webhook}
          title={t('emptyTitle')}
          description={t('emptyDescription')}
          action={
            <Button onClick={() => setEditing({ endpoint: null })}>
              <Plus aria-hidden="true" />
              {t('new')}
            </Button>
          }
        />
      ) : (
        <Table>
          <caption className="sr-only">{t('listLabel')}</caption>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">{t('columns.destination')}</TableHead>
              <TableHead scope="col" className="hidden md:table-cell">{t('columns.events')}</TableHead>
              <TableHead scope="col">{t('columns.status')}</TableHead>
              <TableHead scope="col" className="hidden lg:table-cell">{t('columns.failures')}</TableHead>
              <TableHead scope="col" className="hidden sm:table-cell">{t('columns.lastSuccess')}</TableHead>
              <TableHead scope="col" className="text-right">
                <span className="sr-only">{t('columns.actions')}</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {endpoints.data.map((endpoint) => (
              <EndpointRow
                key={endpoint.id}
                endpoint={endpoint}
                onShowDeliveries={() => setViewing(endpoint)}
                onEdit={() => setEditing({ endpoint })}
                onRotate={() => setRotating(endpoint)}
                onDelete={() => setDeleting(endpoint)}
              />
            ))}
          </TableBody>
        </Table>
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
      <Dialog open={viewing !== null} onOpenChange={(open) => !open && setViewing(null)}>
        <DialogContent size="3xl">
          <DialogHeader>
            <DialogTitle>{t('deliveries.dialogTitle')}</DialogTitle>
            <DialogDescription className="break-all">{viewing?.url}</DialogDescription>
          </DialogHeader>
          {viewing && <DeliveryLog endpoint={viewing} />}
        </DialogContent>
      </Dialog>
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
  onShowDeliveries: () => void
  onEdit: () => void
  onRotate: () => void
  onDelete: () => void
}

function EndpointRow({ endpoint, onShowDeliveries, onEdit, onRotate, onDelete }: EndpointRowProps) {
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
    <TableRow>
      <TableCell className="max-w-72">
        <span className="block truncate font-mono text-sm" title={endpoint.url}>
          {endpoint.url}
        </span>
        {endpoint.description && (
          <span className="block truncate text-xs text-muted-foreground">{endpoint.description}</span>
        )}
      </TableCell>
      <TableCell className="hidden md:table-cell">
        {subscribesToAll(endpoint) ? (
          <Badge variant="secondary">{t('allEvents')}</Badge>
        ) : (
          <Badge variant="secondary" title={endpoint.events.join(', ')}>
            {t('eventCount', { count: endpoint.events.length })}
          </Badge>
        )}
      </TableCell>
      <TableCell>
        <Badge
          variant={endpoint.is_active ? 'success' : endpoint.disabled_reason === 'failures' ? 'destructive' : 'outline'}
          title={
            !endpoint.is_active && endpoint.disabled_reason === 'failures' && endpoint.disabled_at
              ? t('disabledByFailures', { date: formatDateTime(endpoint.disabled_at, formatting) })
              : undefined
          }
        >
          {t(statusKey)}
        </Badge>
      </TableCell>
      <TableCell className="hidden tabular-nums lg:table-cell">
        {endpoint.consecutive_failures > 0 ? endpoint.consecutive_failures : '—'}
      </TableCell>
      <TableCell className="hidden text-muted-foreground sm:table-cell">
        {endpoint.last_success_at ? formatDateTime(endpoint.last_success_at, formatting) : t('neverDelivered')}
      </TableCell>
      <TableCell className="text-right">
        <div className="flex items-center justify-end gap-1">
          <Button size="sm" variant="outline" onClick={onShowDeliveries}>
            {t('actions.deliveries')}
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button size="icon" variant="ghost" disabled={busy} aria-label={t('actions.menu', { url: endpoint.url })}>
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
      </TableCell>
    </TableRow>
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
