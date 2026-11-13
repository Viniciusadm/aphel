import { Check, Copy, Lock, MoreHorizontal, Plus, RadioTower } from 'lucide-react'
import { useState } from 'react'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
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
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Skeleton } from '@/components/ui/skeleton'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import {
  useChannelProviders,
  channelText,
  MESSAGING_KEY,
  MANAGE_CHANNELS_PERMISSION,
  messagingApi,
  useChannels,
  useInvalidateMessaging,
  type Channel,
  type ChannelKind,
} from './api'
import { ChannelEditorDialog } from './ChannelEditorDialog'
import { errorText } from './errors'
import { ChannelIcon } from './shared'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'

type Editing = { kind: ChannelKind; channel: Channel | null }

export function ChannelsPage() {
  const t = useModuleTranslation(MESSAGING_KEY)
  const { hasPermission } = useSession()
  const canManage = hasPermission(MANAGE_CHANNELS_PERMISSION)
  const channels = useChannels(canManage)
  const providers = useChannelProviders(canManage)
  const invalidate = useInvalidateMessaging()
  const [editing, setEditing] = useState<Editing | null>(null)
  const [deleting, setDeleting] = useState<Channel | null>(null)
  const [webhookOf, setWebhookOf] = useState<Channel | null>(null)

  const hasEmail = (providers.data ?? []).some((provider) => provider.key === 'email')

  if (!canManage) {
    return <EmptyState icon={Lock} title={t('channels.noAccessTitle')} description={t('channels.noAccessDescription')} />
  }

  const newButtons = (
    <div className="flex flex-wrap gap-2">
      {(providers.data ?? []).map((provider) => (
        <Button key={provider.key} size="sm" onClick={() => setEditing({ kind: provider.key, channel: null })}>
          <Plus aria-hidden="true" />
          {channelText(provider.label)}
        </Button>
      ))}
    </div>
  )

  const remove = async (channel: Channel) => {
    try {
      await messagingApi.deleteChannel(channel.id)
      notifySuccess(t('channels.deleted', { name: channel.name }))
      await invalidate()
    } catch (error) {
      const known = errorText(t, error)
      if (known) notifyError(error, { title: known })
      else notifyError(error)
    }
  }

  return (
    <Page>
      <PageHeader
        back={{ to: '/messaging', label: t('channels.back') }}
        title={t('channels.title')}
        description={t(hasEmail ? 'channels.description' : 'channels.descriptionWhatsapp')}
        actions={newButtons}
      />

      {channels.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-24 w-full" />
          <Skeleton className="h-24 w-full" />
        </div>
      ) : channels.isError ? (
        <ErrorState onRetry={() => void channels.refetch()} />
      ) : channels.data.data.length === 0 ? (
        <EmptyState
          icon={RadioTower}
          title={t('channels.emptyTitle')}
          description={t(hasEmail ? 'channels.emptyDescription' : 'channels.emptyDescriptionWhatsapp')}
          action={newButtons}
        />
      ) : (
        <Table aria-label={t('channels.listLabel')}>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">{t('channels.columns.name')}</TableHead>
              <TableHead scope="col" className="hidden md:table-cell">{t('channels.columns.type')}</TableHead>
              <TableHead scope="col" className="hidden lg:table-cell">{t('channels.columns.address')}</TableHead>
              <TableHead scope="col">{t('channels.columns.status')}</TableHead>
              <TableHead scope="col" className="hidden md:table-cell">{t('channels.columns.lastCheck')}</TableHead>
              <TableHead scope="col">
                <span className="sr-only">{t('channels.columns.actions')}</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {channels.data.data.map((channel) => (
              <ChannelRow
                key={channel.id}
                channel={channel}
                onEdit={() => setEditing({ kind: channel.kind, channel })}
                onDelete={() => setDeleting(channel)}
                onWebhook={() => setWebhookOf(channel)}
                onTested={() => void invalidate()}
              />
            ))}
          </TableBody>
        </Table>
      )}

      <WebhookDialog channel={webhookOf} onClose={() => setWebhookOf(null)} />
      <ChannelEditorDialog
        editing={editing}
        onOpenChange={(open) => !open && setEditing(null)}
        onSaved={(channel, created) => {
          setEditing(null)
          void invalidate()
          notifySuccess(t(created ? 'channels.created' : 'channels.saved', { name: channel.name }))
        }}
      />
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={t('channels.confirmDelete.title')}
        description={deleting ? t('channels.confirmDelete.description', { name: deleting.name }) : ''}
        confirmLabel={t('channels.confirmDelete.action')}
        variant="destructive"
        onConfirm={() => (deleting ? remove(deleting) : undefined)}
      />
    </Page>
  )
}

function statusVariant(channel: Channel) {
  if (!channel.is_active) return 'outline' as const
  if (channel.status === 'error') return 'destructive' as const
  if (channel.status === 'active') return 'success' as const
  return 'secondary' as const
}

function ChannelRow({
  channel,
  onEdit,
  onDelete,
  onWebhook,
  onTested,
}: {
  channel: Channel
  onEdit: () => void
  onDelete: () => void
  onWebhook: () => void
  onTested: () => void
}) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const formatting = useRecordFormatting()
  const [testing, setTesting] = useState(false)

  const test = async () => {
    setTesting(true)
    try {
      const result = await messagingApi.testChannel(channel.id)
      if (result.ok) {
        notifySuccess(t('channels.testOk'), { description: result.detail ?? undefined })
      } else {
        notifyError(null, {
          title: result.error_code
            ? t(`errors.${result.error_code}`, { defaultValue: result.message ?? t('channels.testFailed') })
            : t('channels.testFailed'),
        })
      }
      onTested()
    } catch (error) {
      notifyError(error)
    } finally {
      setTesting(false)
    }
  }

  const lastActivity = channel.kind === 'email' ? channel.last_polled_at : channel.last_checked_at

  return (
    <TableRow>
      <TableCell>
        <div className="flex min-w-0 items-center gap-2.5">
          <ChannelIcon kind={channel.kind} className="size-5 shrink-0 text-muted-foreground" />
          <span className="truncate font-medium">{channel.name}</span>
        </div>
      </TableCell>
      <TableCell className="hidden md:table-cell">
        {t(channel.kind === 'email' ? 'kinds.email' : channel.kind === 'whatsapp' ? 'kinds.whatsapp' : 'kinds.other')}
      </TableCell>
      <TableCell className="hidden whitespace-nowrap text-muted-foreground tabular-nums lg:table-cell">{channel.address || '—'}</TableCell>
      <TableCell>
        <div className="grid gap-1">
          <Badge variant={statusVariant(channel)} className="w-fit">
            {channel.is_active ? t(`channelStatus.${channel.status}`, { defaultValue: channel.status }) : t('channelStatus.inactive')}
          </Badge>
          {channel.last_error && <span className="max-w-64 text-xs break-words text-destructive">{channel.last_error}</span>}
        </div>
      </TableCell>
      <TableCell className="hidden whitespace-nowrap text-muted-foreground md:table-cell">
        {lastActivity ? formatDateTime(lastActivity, formatting) : '—'}
      </TableCell>
      <TableCell>
        <div className="flex items-center justify-end gap-1">
          <Button size="sm" variant="outline" disabled={testing} onClick={() => void test()}>
            {t('channels.test')}
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button size="icon" variant="ghost" aria-label={t('channels.moreActions', { name: channel.name })}>
                <MoreHorizontal />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onSelect={onEdit}>{t('channels.edit')}</DropdownMenuItem>
              {channel.webhook_url && <DropdownMenuItem onSelect={onWebhook}>{t('channels.webhookAction')}</DropdownMenuItem>}
              <DropdownMenuSeparator />
              <DropdownMenuItem onSelect={onDelete} className="text-destructive">
                {t('channels.delete')}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </TableCell>
    </TableRow>
  )
}

function WebhookDialog({ channel, onClose }: { channel: Channel | null; onClose: () => void }) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const [copied, setCopied] = useState(false)
  const copy = async () => {
    if (!channel?.webhook_url) return
    try {
      await navigator.clipboard.writeText(channel.webhook_url)
      setCopied(true)
      window.setTimeout(() => setCopied(false), 2000)
    } catch (error) {
      notifyError(error)
    }
  }
  return (
    <Dialog open={channel !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{t('channels.webhookTitle', { name: channel?.name ?? '' })}</DialogTitle>
          <DialogDescription>{t('channels.webhookDescription')}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-2">
          <span className="text-sm font-medium">{t('channels.webhookUrl')}</span>
          <div className="flex items-start gap-2 rounded-md border bg-muted/40 p-3">
            <code className="min-w-0 flex-1 text-xs break-all select-all">{channel?.webhook_url ?? ''}</code>
            <Button type="button" size="sm" variant="outline" className="shrink-0" aria-label={t('channels.copyWebhook')} onClick={() => void copy()}>
              {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
              {copied ? t('channels.copied') : t('channels.copy')}
            </Button>
          </div>
        </div>
        <p className="text-sm text-muted-foreground">
          {channel?.secrets?.verify_token ? t('channels.verifyTokenConfigured') : t('channels.verifyTokenMissing')}
        </p>
        <DialogFooter>
          <Button type="button" onClick={onClose}>
            {t('actions.close')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
