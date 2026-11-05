import { ArrowDownLeft, ArrowUpRight, FileText } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import { VisibilityBadge } from '../components/VisibilityBadge'
import { joinParticipants, threadCounterparts } from '../lib/format'
import type { EmailThread } from '../types'

interface ThreadListProps {
  threads: EmailThread[]
  onOpen: (thread: EmailThread) => void
}

export function ThreadList({ threads, onOpen }: ThreadListProps) {
  return (
    <ul className="grid divide-y rounded-lg border">
      {threads.map((thread) => (
        <li key={thread.id}>
          <ThreadRow thread={thread} onOpen={() => onOpen(thread)} />
        </li>
      ))}
    </ul>
  )
}

function ThreadRow({ thread, onOpen }: { thread: EmailThread; onOpen: () => void }) {
  const { t } = useTranslation('email')
  const formatting = useRecordFormatting()
  const DirectionIcon = thread.last_direction === 'inbound' ? ArrowDownLeft : ArrowUpRight
  const counterparts = threadCounterparts(thread)
  return (
    <button
      type="button"
      onClick={onOpen}
      className="grid w-full gap-1 p-3 text-left transition-colors hover:bg-accent/50 focus-visible:bg-accent/50 focus-visible:outline-none"
    >
      <div className="flex items-start gap-2">
        <DirectionIcon
          className="mt-0.5 size-4 shrink-0 text-muted-foreground"
          aria-label={t(`direction.${thread.last_direction ?? 'inbound'}`)}
        />
        <p className="min-w-0 flex-1 truncate font-medium">{thread.subject || t('thread.noSubject')}</p>
        <time dateTime={thread.last_message_at ?? undefined} className="shrink-0 text-xs text-muted-foreground">
          {formatDateTime(thread.last_message_at, formatting)}
        </time>
      </div>
      <p className="truncate pl-6 text-sm text-muted-foreground">
        {counterparts.length > 0 ? joinParticipants(counterparts) : thread.mailbox_address}
      </p>
      {thread.snippet && <p className="line-clamp-2 pl-6 text-sm break-words">{thread.snippet}</p>}
      <div className="flex flex-wrap items-center gap-1.5 pl-6 text-xs text-muted-foreground">
        <span>{t('thread.owner', { name: thread.owner.display_name })}</span>
        <span aria-hidden="true">·</span>
        <span>{t('thread.messageCount', { count: thread.message_count })}</span>
        {thread.access === 'full' && <VisibilityBadge visibility={thread.visibility} />}
        {thread.access === 'metadata' && (
          <Badge variant="secondary">
            <FileText className="size-3" aria-hidden="true" />
            {t('thread.metadataOnly')}
          </Badge>
        )}
      </div>
    </button>
  )
}
