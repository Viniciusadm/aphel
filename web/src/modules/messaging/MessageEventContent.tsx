import { Link } from 'react-router'
import { type TimelineItem } from '@/sdk'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { MESSAGING_KEY, inboxPath } from './api'

function text(payload: Record<string, unknown>, key: string): string | null {
  const value = payload[key]
  return typeof value === 'string' && value.trim() ? value : null
}

function payloadOf(item: TimelineItem): Record<string, unknown> {
  const payload = item.metadata.payload
  return typeof payload === 'object' && payload !== null ? (payload as Record<string, unknown>) : {}
}

export function MessageEventContent({ item }: { item: TimelineItem }) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const payload = payloadOf(item)
  const eventType = String(item.metadata.event_type ?? '')
  const action = eventType.split('.')[1] ?? 'received'
  const channel = text(payload, 'channel_name') ?? t('kinds.other')
  const peer = text(payload, 'peer_name') ?? text(payload, 'peer') ?? ''
  const excerpt = text(payload, 'excerpt')
  const subject = text(payload, 'subject')
  const messageType = text(payload, 'message_type') ?? 'text'
  const conversationId = text(payload, 'conversation_id')
  return (
    <div className="grid min-w-0 gap-1">
      <p className="text-sm font-medium break-words">{t(`timeline.${action}`, { channel, peer })}</p>
      {subject && <p className="truncate text-sm">{subject}</p>}
      {excerpt ? (
        <p className="line-clamp-4 text-sm break-words whitespace-pre-line text-muted-foreground">{excerpt}</p>
      ) : (
        <p className="text-sm text-muted-foreground italic">
          {t(`messageTypes.${messageType}`, { defaultValue: t('messageTypes.other') })}
        </p>
      )}
      {conversationId && (
        <Link
          to={inboxPath(conversationId)}
          className="w-fit text-sm text-primary underline-offset-4 hover:underline"
        >
          {t('timeline.open')}
        </Link>
      )}
    </div>
  )
}

export function ConversationEventContent({ item }: { item: TimelineItem }) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const payload = payloadOf(item)
  const eventType = String(item.metadata.event_type ?? '')
  const action = eventType.split('.')[1] ?? 'assigned'
  const status = text(payload, 'status')
  const conversationId = text(payload, 'conversation_id')
  return (
    <div className="grid min-w-0 gap-1">
      <p className="text-sm font-medium break-words">
        {t(`timeline.conversation.${action}`, { peer: text(payload, 'peer') ?? '', status: status ? t(`status.${status}`) : '' })}
      </p>
      {conversationId && (
        <Link
          to={inboxPath(conversationId)}
          className="w-fit text-sm text-primary underline-offset-4 hover:underline"
        >
          {t('timeline.open')}
        </Link>
      )}
    </div>
  )
}
