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
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { formatDateTime, ListSkeleton, useRecordFormatting } from '@/sdk'
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
          <Button onClick={() => setCreating(true)}>
            <Plus aria-hidden="true" />
            {t('new')}
          </Button>
        }
      />
      {webhooks.isPending ? (
        <ListSkeleton rows={3} />
      ) : webhooks.isError ? (
        <ErrorState onRetry={() => void webhooks.refetch()} />
      ) : webhooks.data.length === 0 ? (
        <EmptyState
          icon={Webhook}
          title={t('emptyTitle')}
          description={t('emptyDescription')}
          action={
            <Button onClick={() => setCreating(true)}>
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
              <TableHead scope="col">{t('columns.name')}</TableHead>
              <TableHead scope="col">{t('columns.status')}</TableHead>
              <TableHead scope="col" className="hidden md:table-cell">{t('columns.signed')}</TableHead>
              <TableHead scope="col" className="hidden lg:table-cell">{t('columns.createdBy')}</TableHead>
              <TableHead scope="col" className="hidden sm:table-cell">{t('columns.lastReceived')}</TableHead>
              <TableHead scope="col" className="text-right">
                <span className="sr-only">{t('columns.actions')}</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
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
          </TableBody>
        </Table>
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
    <TableRow>
      <TableCell className="max-w-64">
        <span className="block truncate font-medium">{webhook.name}</span>
        <span className="block truncate text-xs text-muted-foreground">
          {webhook.description ||
            (actions.length === 0
              ? t('noActionsShort')
              : actions.map((action) => t(`actionTypes.${action.type}`)).join(' · '))}
        </span>
      </TableCell>
      <TableCell>
        <Badge variant={webhook.is_active ? 'success' : 'outline'}>{webhook.is_active ? t('active') : t('inactive')}</Badge>
      </TableCell>
      <TableCell className="hidden md:table-cell">{webhook.secret_configured ? t('signedYes') : t('unsigned')}</TableCell>
      <TableCell className="hidden text-muted-foreground lg:table-cell">{webhook.created_by_name ?? '—'}</TableCell>
      <TableCell className="hidden text-muted-foreground tabular-nums sm:table-cell">
        {webhook.last_received_at ? formatDateTime(webhook.last_received_at, formatting) : t('noneReceived')}
      </TableCell>
      <TableCell className="text-right">
        <div className="flex items-center justify-end gap-1">
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
              <DropdownMenuItem onSelect={onLog}>{t('viewLog')}</DropdownMenuItem>
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
      </TableCell>
    </TableRow>
  )
}
