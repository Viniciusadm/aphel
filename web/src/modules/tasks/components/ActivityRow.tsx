import { Bell, CalendarClock, Clock, Repeat, UserRound } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Checkbox } from '@/components/ui/checkbox'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { ActivityMapLink } from '@/features/mobile/components/ActivityMapLink'
import { getRecordResource } from '@/features/records/resources'
import { cn } from '@/lib/utils'
import type { Activity } from '../api'
import { dueState } from '../lib/due'
import { activityResource } from '../resource'
import { ActivityTypeIcon } from './ActivityTypeIcon'

export interface ActivityRowProps {
  activity: Activity
  onToggle?: (activity: Activity, done: boolean) => void
  pending?: boolean
  showRecord?: boolean
  now?: Date
}

function LinkedRecord({ recordType, recordId, title }: { recordType: string; recordId: string; title?: string | null }) {
  const { t } = useTranslation('tasks')
  const href = getRecordResource(recordType)?.detailPath?.(recordId)
  const label = title || t('row.relatedRecord')
  if (!href) return <span className="truncate">{label}</span>
  return (
    <Link to={href} className="truncate text-primary underline-offset-4 hover:underline">
      {label}
    </Link>
  )
}

export function ActivityRow({ activity, onToggle, pending = false, showRecord = false, now }: ActivityRowProps) {
  const { t } = useTranslation('tasks')
  const formatting = useRecordFormatting()
  const done = activity.status === 'done'
  const canceled = activity.status === 'canceled'
  const due = done || canceled ? 'none' : dueState(activity.due_at, formatting.timezone, now)
  const detail = activityResource.detailPath?.(activity.id)

  return (
    <li
      data-activity-id={activity.id}
      className="flex items-start gap-3 rounded-md border bg-card px-3 py-2.5 text-card-foreground"
    >
      <Checkbox
        className="mt-1.5"
        checked={done}
        disabled={!onToggle || pending || canceled}
        onCheckedChange={(checked) => onToggle?.(activity, checked === true)}
        aria-label={done ? t('row.reopen', { title: activity.title }) : t('row.complete', { title: activity.title })}
      />
      <ActivityTypeIcon icon={activity.activity_type.icon} color={activity.activity_type.color} />
      <div className="grid min-w-0 flex-1 gap-1">
        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          {detail ? (
            <Link
              to={detail}
              className={cn(
                'min-w-0 truncate text-sm font-medium hover:underline',
                (done || canceled) && 'text-muted-foreground line-through',
              )}
            >
              {activity.title}
            </Link>
          ) : (
            <span className="min-w-0 truncate text-sm font-medium">{activity.title}</span>
          )}
          {(activity.priority === 'high' || activity.priority === 'urgent') && !done && (
            <Badge variant={activity.priority === 'urgent' ? 'destructive' : 'warning'}>
              {t(`priority.${activity.priority}`)}
            </Badge>
          )}
          {canceled && <Badge variant="secondary">{t('status.canceled')}</Badge>}
          {activity.recurrence && !activity.recurrence.ended && (
            <Repeat className="size-3.5 shrink-0 text-muted-foreground" aria-label={t('row.recurring')} role="img" />
          )}
          {activity.reminder_minutes_before != null && !done && !canceled && activity.due_at && (
            <Bell className="size-3.5 shrink-0 text-muted-foreground" aria-label={t('row.reminder')} role="img" />
          )}
        </div>
        <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
          <span>{activity.activity_type.name}</span>
          {done && activity.completed_at && (
            <span className="inline-flex items-center gap-1">
              <Clock className="size-3" aria-hidden="true" />
              {t('row.completedAt', { date: formatDateTime(activity.completed_at, formatting) })}
            </span>
          )}
          {!done && activity.due_at && (
            <span
              className={cn(
                'inline-flex items-center gap-1',
                due === 'overdue' && 'font-medium text-destructive',
                due === 'today' && 'font-medium text-foreground',
              )}
            >
              <CalendarClock className="size-3" aria-hidden="true" />
              {due === 'overdue'
                ? t('row.overdue', { date: formatDateTime(activity.due_at, formatting) })
                : t('row.due', { date: formatDateTime(activity.due_at, formatting) })}
            </span>
          )}
          {activity.owner && (
            <span className="inline-flex items-center gap-1">
              <UserRound className="size-3" aria-hidden="true" />
              {activity.owner.display_name}
            </span>
          )}
          {showRecord && activity.record && (
            <LinkedRecord
              recordType={activity.record.record_type}
              recordId={activity.record.record_id}
              title={activity.record.title}
            />
          )}
          <ActivityMapLink location={activity.location} />
        </div>
      </div>
    </li>
  )
}
