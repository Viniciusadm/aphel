import { applyFilters } from '@/kernel/modules/filters'
import { useModuleState } from '@/kernel/modules/registry'
import { useTranslation } from 'react-i18next'
import { Avatar } from '@/components/ui/avatar'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import type { RecentActivityData } from '@/sdk'
import { RecordLinkText } from '@/kernel/widgets/shared'

type RecentEvent = RecentActivityData['items'][number]

export function ActivityEventRow({ event }: { event: RecentEvent }) {
  const { t } = useTranslation('dashboard')
  const modules = useModuleState()
  const formatting = useRecordFormatting()
  const actor = event.actor?.display_name ?? t('widgets.recent_activity.someone')
  const action = applyFilters('dashboard.eventLabel', t('events.fallback'), { event }, modules)
  return (
    <li className="flex min-w-0 gap-3 text-sm">
      <Avatar name={actor} size="md" />
      <div className="grid min-w-0 gap-0.5">
        <p className="min-w-0 break-words">
          <span className="font-medium">{actor}</span> {action}{' '}
          {event.record && <RecordLinkText record={event.record} fallback={event.record.record_type} />}
        </p>
        <time dateTime={event.occurred_at} className="text-xs text-muted-foreground">
          {formatDateTime(event.occurred_at, formatting)}
        </time>
      </div>
    </li>
  )
}

export function RecentActivityWidget({ data }: { data: RecentActivityData }) {
  return (
    <ol className="grid max-h-[30rem] gap-3 overflow-y-auto pr-2">
      {data.items.map((event) => (
        <ActivityEventRow key={event.id} event={event} />
      ))}
    </ol>
  )
}
