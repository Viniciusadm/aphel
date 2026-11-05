import { FileText } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link, useLocation } from 'react-router'
import { RECORD_TAB_PARAM, type TimelineItem } from '@/sdk'
import {EMAILS_TAB_KEY, EMAIL_EVENT_TYPES, EMAIL_THREAD_PARAM, type EmailEventType} from '@/sdk'
import {participantName} from '@/sdk'
import { timelineEmail } from '../lib/timeline'

function eventTypeOf(item: TimelineItem): EmailEventType {
  const value = item.metadata.event_type
  return EMAIL_EVENT_TYPES.find((type) => type === value) ?? 'email.received'
}

function useThreadHref(threadId: string): string {
  const location = useLocation()
  const params = new URLSearchParams(location.search)
  params.set(RECORD_TAB_PARAM, EMAILS_TAB_KEY)
  params.set(EMAIL_THREAD_PARAM, threadId)
  return `${location.pathname}?${params.toString()}`
}

export function EmailEventContent({ item }: { item: TimelineItem }) {
  const { t } = useTranslation('email')
  const email = timelineEmail(item)
  const eventType = eventTypeOf(item)
  const action = eventType.slice('email.'.length) as 'received' | 'sent' | 'opened' | 'clicked' | 'bounced'

  if (!email) {
    return <p className="text-sm font-medium break-words">{item.title || t(`timeline.${action}`, { from: '' })}</p>
  }

  const from = email.from ? participantName(email.from) : t('reader.unknownSender')
  return (
    <div className="grid min-w-0 gap-1">
      <p className="text-sm font-medium break-words">{t(`timeline.${action}`, { from })}</p>
      <p className="truncate text-sm">{email.subject || t('thread.noSubject')}</p>
      {email.access === 'full' && email.snippet && (
        <p className="line-clamp-3 text-sm break-words whitespace-pre-line text-muted-foreground">{email.snippet}</p>
      )}
      {email.access === 'metadata' && (
        <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <FileText className="size-3.5" aria-hidden="true" />
          {t('thread.metadataOnly')}
        </p>
      )}
      <ThreadLink threadId={email.thread_id} />
    </div>
  )
}

function ThreadLink({ threadId }: { threadId: string }) {
  const { t } = useTranslation('email')
  const href = useThreadHref(threadId)
  return (
    <Link to={href} className="w-fit text-sm text-primary underline-offset-4 hover:underline">
      {t('timeline.open')}
    </Link>
  )
}
