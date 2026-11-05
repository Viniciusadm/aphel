import { MoreHorizontal, Plus, Webhook } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
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
import { formatDateTime, useRecordFormatting } from '@/sdk'
import {
  incomingWebhooksApi,
  useIncomingWebhooks,
  useInvalidateIncomingWebhooks,
  webhookActions,
  type IncomingWebhook,
  type IssuedIncomingWebhook,
} from './api'
import { CredentialsDialog } from './CredentialsDialog'
import { RequestLogDialog } from './RequestLogDialog'
import { WebhookEditorDialog } from './WebhookEditorDialog'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

type PendingConfirmation =
  | { kind: 'delete'; webhook: IncomingWebhook }
  | { kind: 'regenerateToken'; webhook: IncomingWebhook }
  | { kind: 'rotateSecret'; webhook: IncomingWebhook }
  | { kind: 'removeSecret'; webhook: IncomingWebhook }

function exampleOnly(webhook: IncomingWebhook): IssuedIncomingWebhook {
  return { ...webhook, token: null, path: null, signing_secret: null }
}

export function IncomingWebhooksPage() {
  const { t } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  const webhooks = useIncomingWebhooks()
  const invalidate = useInvalidateIncomingWebhooks()
  const [editing, setEditing] = useState<IncomingWebhook | null>(null)
  const [creating, setCreating] = useState(false)
  const [issued, setIssued] = useState<IssuedIncomingWebhook | null>(null)
  const [logOf, setLogOf] = useState<IncomingWebhook | null>(null)
  const [pending, setPending] = useState<PendingConfirmation | null>(null)

  const confirm = async (confirmation: PendingConfirmation) => {
    const { webhook } = confirmation
    switch (confirmation.kind) {
      case 'delete':
        await incomingWebhooksApi.remove(webhook.id)
        notifySuccess(t('deleted', { name: webhook.name }))
        break
      case 'regenerateToken':
        setIssued(await incomingWebhooksApi.regenerateToken(webhook.id))
        break
      case 'rotateSecret':
        setIssued(await incomingWebhooksApi.rotateSecret(webhook.id))
        break
      case 'removeSecret':
        await incomingWebhooksApi.removeSecret(webhook.id)
        notifySuccess(t('secretRemoved', { name: webhook.name }))
        break
    }
    await invalidate()
  }

  return (
    <Page>
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <Button size="sm" onClick={() => setCreating(true)}>
            <Plus aria-hidden="true" />
            {t('new')}
          </Button>
        }
      />
      {webhooks.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-20 w-full" />
        </div>
      ) : webhooks.isError ? (
        <ErrorState onRetry={() => void webhooks.refetch()} />
      ) : webhooks.data.length === 0 ? (
        <EmptyState icon={Webhook} title={t('emptyTitle')} description={t('emptyDescription')} />
      ) : (
        <ul className="grid gap-2" aria-label={t('listLabel')}>
          {webhooks.data.map((webhook) => (
            <WebhookRow
              key={webhook.id}
              webhook={webhook}
              onEdit={() => setEditing(webhook)}
              onLog={() => setLogOf(webhook)}
              onExample={() => setIssued(exampleOnly(webhook))}
              onConfirm={(kind) => setPending({ kind, webhook })}
            />
          ))}
        </ul>
      )}

      <WebhookEditorDialog
        open={creating || editing !== null}
        webhook={editing}
        onOpenChange={(open) => {
          if (!open) {
            setCreating(false)
            setEditing(null)
          }
        }}
        onSaved={({ webhook, issued: created }) => {
          setCreating(false)
          setEditing(null)
          void invalidate()
          if (created) setIssued(created)
          else notifySuccess(t('saved', { name: webhook.name }))
        }}
      />
      <CredentialsDialog issued={issued} onClose={() => setIssued(null)} />
      <RequestLogDialog webhook={logOf} onClose={() => setLogOf(null)} />
      <ConfirmDialog
        open={pending !== null}
        onOpenChange={(open) => !open && setPending(null)}
        title={pending ? t(`confirm.${pending.kind}.title`) : ''}
        description={pending ? t(`confirm.${pending.kind}.description`, { name: pending.webhook.name }) : ''}
        confirmLabel={pending ? t(`confirm.${pending.kind}.action`) : undefined}
        variant={pending?.kind === 'delete' || pending?.kind === 'regenerateToken' ? 'destructive' : 'default'}
        onConfirm={() => (pending ? confirm(pending) : undefined)}
      />
    </Page>
  )
}

interface WebhookRowProps {
  webhook: IncomingWebhook
  onEdit: () => void
  onLog: () => void
  onExample: () => void
  onConfirm: (kind: PendingConfirmation['kind']) => void
}

function WebhookRow({ webhook, onEdit, onLog, onExample, onConfirm }: WebhookRowProps) {
  const { t } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  const formatting = useRecordFormatting()
  const actions = webhookActions(webhook)

  return (
    <li className="flex flex-col gap-2 rounded-md border bg-card px-3 py-2 sm:flex-row sm:items-start sm:justify-between">
      <div className="grid min-w-0 gap-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-medium break-words">{webhook.name}</span>
          <code className="rounded bg-muted px-1.5 py-0.5 text-xs">{webhook.token_prefix}…</code>
          <Badge variant={webhook.is_active ? 'success' : 'outline'}>
            {webhook.is_active ? t('active') : t('inactive')}
          </Badge>
          {webhook.secret_configured && <Badge variant="secondary">{t('signed')}</Badge>}
        </div>
        {webhook.description && <p className="text-sm text-muted-foreground break-words">{webhook.description}</p>}
        <div className="flex flex-wrap gap-1">
          {actions.length === 0 ? (
            <span className="text-xs text-muted-foreground">{t('noActions')}</span>
          ) : (
            actions.map((action, index) => (
              <Badge key={`${action.type}-${index}`} variant="outline" className="text-[11px]">
                {index + 1}. {t(`actionTypes.${action.type}`)}
              </Badge>
            ))
          )}
        </div>
        <p className="text-xs text-muted-foreground">
          {webhook.created_by_name ? `${t('createdBy', { name: webhook.created_by_name })} · ` : ''}
          {webhook.last_received_at
            ? t('lastReceived', { date: formatDateTime(webhook.last_received_at, formatting) })
            : t('neverReceived')}
        </p>
      </div>
      <div className="flex gap-2 self-start">
        <Button size="sm" variant="outline" onClick={onLog}>
          {t('viewLog')}
        </Button>
        <Button size="sm" variant="outline" onClick={onEdit}>
          {t('edit')}
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button size="icon" variant="ghost" aria-label={t('moreActions', { name: webhook.name })}>
              <MoreHorizontal aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onSelect={onExample}>{t('menu.example')}</DropdownMenuItem>
            <DropdownMenuItem onSelect={() => onConfirm('regenerateToken')}>
              {t('menu.regenerateToken')}
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => onConfirm('rotateSecret')}>
              {webhook.secret_configured ? t('menu.rotateSecret') : t('menu.createSecret')}
            </DropdownMenuItem>
            {webhook.secret_configured && (
              <DropdownMenuItem onSelect={() => onConfirm('removeSecret')}>{t('menu.removeSecret')}</DropdownMenuItem>
            )}
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive" onSelect={() => onConfirm('delete')}>
              {t('menu.delete')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  )
}
