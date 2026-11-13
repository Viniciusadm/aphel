import { AlarmClock, CalendarClock, CalendarDays } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import { RecordLinkText } from '@/kernel/widgets/shared'
import { BucketSection } from '@/kernel/widgets/BucketSection'
import type { TaskPendingData, PendingActivityItem } from './types'
export function TaskPendingSections({ data }: { data: TaskPendingData }) {
  const { t } = useTranslation('tasks', { keyPrefix: 'dashboard' })
  const formatting = useRecordFormatting()
  const activity = (item: PendingActivityItem) => (
    <div className="grid min-w-0 gap-0.5">
      <Link to={`/tasks/${encodeURIComponent(item.id)}`} className="line-clamp-2 font-medium break-words hover:underline">
        {item.title || item.id}
      </Link>
      <p className="text-xs text-muted-foreground break-words">
        {item.due_at && t('widgets.pending_center.dueAt', { date: formatDateTime(item.due_at, formatting) })}
        {item.record && (
          <>
            {' · '}
            <RecordLinkText record={item.record} fallback={item.record.record_type} />
          </>
        )}
      </p>
    </div>
  )
  return (
    <div className="grid gap-5">
      <BucketSection emptyLabel={t('widgets.pending_center.noItems')} moreLabel={(count) => t('widgets.pending_center.more', { count })} id="pending-overdue" icon={AlarmClock} tone="danger" title={t('widgets.pending_center.overdue')} bucket={data.overdue} renderItem={activity} />
      <BucketSection emptyLabel={t('widgets.pending_center.noItems')} moreLabel={(count) => t('widgets.pending_center.more', { count })} id="pending-today" icon={CalendarClock} title={t('widgets.pending_center.dueToday')} bucket={data.due_today} renderItem={activity} />
      <BucketSection emptyLabel={t('widgets.pending_center.noItems')} moreLabel={(count) => t('widgets.pending_center.more', { count })} id="pending-tomorrow" icon={CalendarDays} title={t('widgets.pending_center.dueTomorrow')} bucket={data.due_tomorrow} renderItem={activity} />

    </div>
  )
}
