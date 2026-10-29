import { useTranslation } from 'react-i18next'
import { Avatar } from '@/components/ui/avatar'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import type { RecentActivityData, RecentEventItem } from '../types'
import { RecordLinkText } from '@/kernel/widgets/shared'

function stageNames(event: RecentEventItem): { from: string; to: string } | null {
  if (event.event_type !== 'deal.stage_changed' || !event.payload) return null
  const from = (event.payload.from as { name?: string } | undefined)?.name
  const to = (event.payload.to as { name?: string } | undefined)?.name
  return from && to ? { from, to } : null
}

export function RecentActivityWidget({ data }: { data: RecentActivityData }) {
  const { t, i18n } = useTranslation('dashboard')
  const formatting = useRecordFormatting()
  return (
    <ol className="grid gap-3">
      {data.items.map((event) => {
        const actor = event.actor?.display_name ?? t('widgets.recent_activity.someone')
        const key = `events.${event.event_type}`
        const action = i18n.exists(key, { ns: 'dashboard' }) ? t(key as 'events.fallback') : t('events.fallback')
        const stages = stageNames(event)
        return (
          <li key={event.id} className="flex min-w-0 gap-3 text-sm">
            <Avatar name={actor} size="md" />
            <div className="grid min-w-0 gap-0.5">
              <p className="min-w-0 break-words">
                <span className="font-medium">{actor}</span> {action}{' '}
                {event.record && <RecordLinkText record={event.record} fallback={event.record.record_type} />}
                {stages && <span className="text-muted-foreground"> ({t('events.stageChange', stages)})</span>}
              </p>
              <time dateTime={event.occurred_at} className="text-xs text-muted-foreground">
                {formatDateTime(event.occurred_at, formatting)}
              </time>
            </div>
          </li>
        )
      })}
    </ol>
  )
}
