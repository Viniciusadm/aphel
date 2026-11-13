import { Copy, Lock, MoreHorizontal, Plus, RadioTower } from 'lucide-react'
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
import { Skeleton } from '@/components/ui/skeleton'
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
        <ul className="grid gap-2" aria-label={t('channels.listLabel')}>
          {channels.data.data.map((channel) => (
            <ChannelRow
              key={channel.id}
              channel={channel}
              onEdit={() => setEditing({ kind: channel.kind, channel })}
              onDelete={() => setDeleting(channel)}
              onTested={() => void invalidate()}
            />
          ))}
        </ul>
      )}

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
  onTested,
}: {
  channel: Channel
  onEdit: () => void
  onDelete: () => void
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

  const copyWebhook = async () => {
    if (!channel.webhook_url) return
    try {
      await navigator.clipboard.writeText(channel.webhook_url)
      notifySuccess(t('channels.copied'))
    } catch (error) {
      notifyError(error)
    }
  }

  const lastActivity = channel.kind === 'email' ? channel.last_polled_at : channel.last_checked_at

  return (
    <li className="grid gap-2 rounded-md border bg-card px-3 py-3">
      <div className="flex min-w-0 items-start gap-3">
        <ChannelIcon kind={channel.kind} className="mt-0.5 size-5 text-muted-foreground" />
        <div className="grid min-w-0 flex-1 gap-1">
          <div className="flex flex-wrap items-center gap-2">
            <p className="truncate font-medium">{channel.name}</p>
            <Badge variant="outline">{t(channel.kind === 'email' ? 'kinds.email' : channel.kind === 'whatsapp' ? 'kinds.whatsapp' : 'kinds.other')}</Badge>
            <Badge variant={statusVariant(channel)}>
              {channel.is_active ? t(`channelStatus.${channel.status}`, { defaultValue: channel.status }) : t('channelStatus.inactive')}
            </Badge>
          </div>
          <p className="truncate text-sm text-muted-foreground">{channel.address || '—'}</p>
          {lastActivity && (
            <p className="text-xs text-muted-foreground">
              {t(channel.kind === 'email' ? 'channels.lastPolled' : 'channels.lastChecked', {
                date: formatDateTime(lastActivity, formatting),
              })}
            </p>
          )}
          {channel.last_error && <p className="text-xs break-words text-destructive">{channel.last_error}</p>}
        </div>
        <div className="flex shrink-0 items-center gap-1">
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
              <DropdownMenuSeparator />
              <DropdownMenuItem onSelect={onDelete} className="text-destructive">
                {t('channels.delete')}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
      {channel.webhook_url && (
        <div className="grid gap-1 rounded-md bg-muted px-3 py-2 text-xs">
          <span className="font-medium">{t('channels.webhookUrl')}</span>
          <div className="flex min-w-0 items-center gap-2">
            <code className="min-w-0 flex-1 truncate">{channel.webhook_url}</code>
            <Button
              size="icon"
              variant="ghost"
              className="size-7"
              onClick={() => void copyWebhook()}
              aria-label={t('channels.copyWebhook')}
            >
              <Copy className="size-3.5" />
            </Button>
          </div>
          <span className="text-muted-foreground">
            {channel.secrets?.verify_token ? t('channels.verifyTokenConfigured') : t('channels.verifyTokenMissing')}
          </span>
        </div>
      )}
    </li>
  )
}
