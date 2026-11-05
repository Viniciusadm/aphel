import { applyFilters } from '@/kernel/modules/filters'
import { useModuleState } from '@/kernel/modules/registry'
import { useTranslation } from 'react-i18next'
import { Avatar } from '@/components/ui/avatar'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import type { RecentActivityData } from '../types'
import { RecordLinkText } from '@/kernel/widgets/shared'

export function RecentActivityWidget({ data }: { data: RecentActivityData }) {
  const { t } = useTranslation('dashboard')
  const modules = useModuleState()
  const formatting = useRecordFormatting()
  return (
    <ol className="grid gap-3">
      {data.items.map((event) => {
        const actor = event.actor?.display_name ?? t('widgets.recent_activity.someone')
        const action = applyFilters('dashboard.eventLabel', t('events.fallback'), { event }, modules)
        return (
          <li key={event.id} className="flex min-w-0 gap-3 text-sm">
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
      })}
    </ol>
  )
}
