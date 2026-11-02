import { MessagesSquare } from 'lucide-react'
import { Link } from 'react-router'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { MESSAGING_KEY, inboxPath, peerLabel, useConversations, type ConversationFilters } from './api'
import { ChannelIcon, StatusBadge } from './shared'

export function ConversationsTab({ recordType, recordId }: RecordTabRenderProps) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const formatting = useRecordFormatting()
  const filters: ConversationFilters = {
    channelId: null,
    status: null,
    assignee: 'any',
    unread: false,
    search: '',
    page: 1,
    ...(recordType === 'deals.deal' ? { dealId: recordId } : { contactId: recordId }),
  }
  const conversations = useConversations(filters)

  return (
    <section className="flex flex-col gap-3" aria-label={t('tab.title')}>
      <div className="grid gap-0.5">
        <h2 className="text-base font-semibold">{t('tab.title')}</h2>
        <p className="text-sm text-muted-foreground">{t(`tab.description.${recordType === 'deals.deal' ? 'deal' : 'contact'}`)}</p>
      </div>
      {conversations.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-14 w-full" />
          <Skeleton className="h-14 w-full" />
        </div>
      ) : conversations.isError ? (
        <ErrorState onRetry={() => void conversations.refetch()} />
      ) : conversations.data.data.length === 0 ? (
        <EmptyState icon={MessagesSquare} title={t('tab.emptyTitle')} description={t('tab.emptyDescription')} />
      ) : (
        <ul className="grid gap-2">
          {conversations.data.data.map((conversation) => (
            <li key={conversation.id}>
              <Link
                to={inboxPath(conversation.id)}
                className="flex min-w-0 items-start gap-3 rounded-md border bg-card px-3 py-2 transition-colors hover:bg-accent"
              >
                <ChannelIcon kind={conversation.channel.kind} className="mt-0.5 text-muted-foreground" />
                <span className="grid min-w-0 flex-1 gap-0.5">
                  <span className="flex min-w-0 items-center gap-2">
                    <span className="truncate text-sm font-medium">{conversation.subject || peerLabel(conversation)}</span>
                    <StatusBadge status={conversation.status} />
                  </span>
                  <span className="truncate text-xs text-muted-foreground">{conversation.last_message_preview ?? ''}</span>
                  <span className="text-xs text-muted-foreground">
                    {conversation.channel.name} · {formatDateTime(conversation.last_message_at, formatting)}
                  </span>
                </span>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
