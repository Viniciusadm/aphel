import { NativeSelect } from '@/components/ui/native-select'
import { Inbox, Lock, MessageCircle, Search, Settings2 } from 'lucide-react'
import { useState } from 'react'
import { Link, useSearchParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { cn } from '@/lib/utils'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import {
  CONVERSATION_STATUSES,
  CONVERSATIONS_PER_PAGE,
  MESSAGING_KEY,
  MANAGE_CHANNELS_PERMISSION,
  VIEW_PERMISSION,
  peerLabel,
  useChannels,
  useConversations,
  type AssigneeFilter,
  type Conversation,
  type ConversationFilters,
  type ConversationStatus,
} from './api'
import { ConversationPanel } from './ConversationPanel'
import { ChannelIcon, StatusBadge } from './shared'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const ALL = '__all__'
const ASSIGNEE_FILTERS: AssigneeFilter[] = ['any', 'me', 'none']

export function InboxPage() {
  const t = useModuleTranslation(MESSAGING_KEY)
  const { hasPermission } = useSession()
  const [searchParams, setSearchParams] = useSearchParams()
  const selected = searchParams.get('conversation')
  const [filters, setFilters] = useState<ConversationFilters>({
    channelId: null,
    status: 'open',
    assignee: 'any',
    unread: false,
    search: '',
    page: 1,
  })
  const canView = hasPermission(VIEW_PERMISSION)
  const channels = useChannels(canView)
  const conversations = useConversations(filters, canView)

  if (!canView) {
    return <EmptyState icon={Lock} title={t('inbox.noAccessTitle')} description={t('inbox.noAccessDescription')} />
  }

  const update = (patch: Partial<ConversationFilters>) => setFilters((current) => ({ ...current, page: 1, ...patch }))
  const select = (id: string | null) => {
    const next = new URLSearchParams(searchParams)
    if (id) next.set('conversation', id)
    else next.delete('conversation')
    setSearchParams(next)
  }
  const total = conversations.data?.meta.total ?? 0
  const pages = Math.max(1, Math.ceil(total / CONVERSATIONS_PER_PAGE))

  return (
    <Page>
<PageHeader title={t('inbox.title')} description={t('inbox.description')} actions={<>
        {hasPermission(MANAGE_CHANNELS_PERMISSION) && (
          <Button asChild size="sm" variant="outline">
            <Link to="/messaging/channels">
              <Settings2 aria-hidden="true" />
              {t('inbox.manageChannels')}
            </Link>
          </Button>
        )}
      
      </>} />

      <div className="grid gap-3 md:h-[calc(100dvh-11rem)] md:min-h-[520px] md:grid-cols-[minmax(260px,360px)_minmax(0,1fr)]">
        <aside
          className={cn('min-h-0 flex-col gap-2 rounded-md border bg-card p-2', selected ? 'hidden md:flex' : 'flex')}
          aria-label={t('inbox.listLabel')}
        >
          <div className="relative">
            <Search
              className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
              aria-hidden="true"
            />
            <Input
              value={filters.search}
              onChange={(event) => update({ search: event.target.value })}
              placeholder={t('inbox.search')}
              aria-label={t('inbox.search')}
              className="pl-8"
            />
          </div>
          <div className="grid grid-cols-2 gap-2">
            <NativeSelect value={filters.status ?? ALL}
              onValueChange={(value) => update({ status: value === ALL ? null : (value as ConversationStatus) })} aria-label={t('inbox.filters.status')}>
                <option value={ALL}>{t('inbox.filters.allStatuses')}</option>
                {CONVERSATION_STATUSES.map((status) => (
                  <option key={status} value={status}>
                    {t(`status.${status}`)}
                  </option>
                ))}
              </NativeSelect>
            <NativeSelect value={filters.assignee} onValueChange={(value) => update({ assignee: value as AssigneeFilter })} aria-label={t('inbox.filters.assignee')}>
                {ASSIGNEE_FILTERS.map((assignee) => (
                  <option key={assignee} value={assignee}>
                    {t(`inbox.filters.assignees.${assignee}`)}
                  </option>
                ))}
              </NativeSelect>
            <NativeSelect value={filters.channelId ?? ALL}
              onValueChange={(value) => update({ channelId: value === ALL ? null : value })} aria-label={t('inbox.filters.channel')}>
                <option value={ALL}>{t('inbox.filters.allChannels')}</option>
                {(channels.data?.data ?? []).map((channel) => (
                  <option key={channel.id} value={channel.id}>
                    {channel.name}
                  </option>
                ))}
              </NativeSelect>
            <Button
              type="button"
              variant={filters.unread ? 'secondary' : 'outline'}
              aria-pressed={filters.unread}
              onClick={() => update({ unread: !filters.unread })}
            >
              {t('inbox.filters.unread')}
            </Button>
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto">
            {conversations.isPending ? (
              <div className="grid gap-2 p-1" aria-busy="true">
                <Skeleton className="h-16 w-full" />
                <Skeleton className="h-16 w-full" />
                <Skeleton className="h-16 w-full" />
              </div>
            ) : conversations.isError ? (
              <ErrorState onRetry={() => void conversations.refetch()} />
            ) : conversations.data.data.length === 0 ? (
              <EmptyState icon={Inbox} title={t('inbox.emptyTitle')} description={t('inbox.emptyDescription')} />
            ) : (
              <ul className="grid gap-1">
                {conversations.data.data.map((conversation) => (
                  <ConversationRow
                    key={conversation.id}
                    conversation={conversation}
                    active={conversation.id === selected}
                    onSelect={() => select(conversation.id)}
                  />
                ))}
              </ul>
            )}
          </div>
          {pages > 1 && (
            <div className="flex items-center justify-between gap-2 border-t pt-2 text-xs text-muted-foreground">
              <Button
                size="sm"
                variant="ghost"
                disabled={filters.page <= 1}
                onClick={() => setFilters((current) => ({ ...current, page: current.page - 1 }))}
              >
                {t('inbox.previous')}
              </Button>
              <span>{t('inbox.page', { page: filters.page, pages })}</span>
              <Button
                size="sm"
                variant="ghost"
                disabled={filters.page >= pages}
                onClick={() => setFilters((current) => ({ ...current, page: current.page + 1 }))}
              >
                {t('inbox.next')}
              </Button>
            </div>
          )}
        </aside>

        <div className={cn('min-h-0 min-w-0 rounded-md border bg-card', selected ? 'flex' : 'hidden md:flex')}>
          {selected ? (
            <ConversationPanel key={selected} conversationId={selected} onBack={() => select(null)} />
          ) : (
            <EmptyState
              className="m-4 flex-1 border-none"
              icon={MessageCircle}
              title={t('inbox.selectTitle')}
              description={t('inbox.selectDescription')}
            />
          )}
        </div>
      </div>
    </Page>
  )
}

function ConversationRow({
  conversation,
  active,
  onSelect,
}: {
  conversation: Conversation
  active: boolean
  onSelect: () => void
}) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const formatting = useRecordFormatting()
  const unread = conversation.unread_count > 0
  return (
    <li>
      <button
        type="button"
        onClick={onSelect}
        aria-current={active ? 'true' : undefined}
        className={cn(
          'grid w-full gap-1 rounded-md px-2 py-2 text-left transition-colors hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
          active && 'bg-accent',
        )}
      >
        <span className="flex min-w-0 items-center gap-2">
          <ChannelIcon kind={conversation.channel.kind} className="text-muted-foreground" />
          <span className={cn('min-w-0 flex-1 truncate text-sm', unread ? 'font-semibold' : 'font-medium')}>
            {peerLabel(conversation)}
          </span>
          <span className="shrink-0 text-xs text-muted-foreground">
            {formatDateTime(conversation.last_message_at, formatting)}
          </span>
        </span>
        {conversation.subject && <span className="truncate text-xs font-medium">{conversation.subject}</span>}
        <span className="flex min-w-0 items-center gap-2">
          <span className="min-w-0 flex-1 truncate text-xs text-muted-foreground">
            {conversation.last_message_direction === 'out' && `${t('inbox.you')}: `}
            {conversation.last_message_preview ?? ''}
          </span>
          {unread && (
            <Badge aria-label={t('inbox.unreadCount', { count: conversation.unread_count })}>
              {conversation.unread_count}
            </Badge>
          )}
          <StatusBadge status={conversation.status} />
        </span>
      </button>
    </li>
  )
}
