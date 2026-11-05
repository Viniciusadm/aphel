import { useTranslation } from 'react-i18next'
import { type TimelineItem } from '@/sdk'
import { formatPercent } from './errors'

type Payload = Record<string, unknown>

function payloadOf(item: TimelineItem): Payload {
  const value = item.metadata.payload
  return value && typeof value === 'object' ? (value as Payload) : {}
}

function text(payload: Payload, key: string): string | null {
  const value = payload[key]
  return typeof value === 'string' && value.trim() !== '' ? value : null
}

const HEADLINES = {
  'deal.member_added': 'timeline.added',
  'deal.member_updated': 'timeline.updated',
  'deal.member_removed': 'timeline.removed',
} as const

export function DealMemberEventContent({ item }: { item: TimelineItem }) {
  const { t, i18n } = useTranslation('deals', { keyPrefix: 'team' })
  const payload = payloadOf(item)
  const eventType = typeof item.metadata.event_type === 'string' ? item.metadata.event_type : ''
  const headline = HEADLINES[eventType as keyof typeof HEADLINES] ?? HEADLINES['deal.member_updated']
  const actor = item.actor?.display_name?.trim() || t('timeline.someone')
  const user = text(payload, 'user_name') ?? t('timeline.someUser')
  const role = text(payload, 'role_name')
  const percent = formatPercent(text(payload, 'credit_percent'), i18n.language)

  return (
    <div className="grid min-w-0 gap-1">
      <p className="text-sm font-medium break-words">{t(headline, { actor, user })}</p>
      {eventType !== 'deal.member_removed' && (
        <p className="text-sm text-muted-foreground">
          {role ? t('timeline.details', { role, percent }) : t('timeline.detailsNoRole', { percent })}
        </p>
      )}
    </div>
  )
}
