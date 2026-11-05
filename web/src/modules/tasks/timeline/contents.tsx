import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { ActivityMapLink, formatDateTime, type TimelineItem, timelineLocation, useRecordFormatting } from '@/sdk'
import { Markdown } from '@/components/markdown'
import { TimelineLine as Line, TimelineRecordLink as RecordLink, asObject, asText, eventType, nameOf, payloadOf } from '@/modules/timeline'
import { activityResource } from '../resource'
import { dealChangeText } from './text'

export function ActivityTimelineContent({ item }: { item: TimelineItem }) {
  const { t } = useTranslation('tasks')
  const formatting = useRecordFormatting()
  const type = asObject(item.metadata.activity_type)
  const status = asText(item.metadata.status)
  const activityId = asText(item.metadata.activity_id)
  const href = activityId ? activityResource.detailPath?.(activityId) : undefined
  const completedAt = asText(item.metadata.completed_at)
  const dueAt = asText(item.metadata.due_at)
  return (
    <div className="grid min-w-0 gap-1">
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        {href ? (
          <Link to={href} className="min-w-0 truncate text-sm font-medium hover:underline">
            {item.title}
          </Link>
        ) : (
          <span className="min-w-0 truncate text-sm font-medium">{item.title}</span>
        )}
        {asText(type.name) && <Badge variant="outline">{asText(type.name)}</Badge>}
        {status === 'done' ? (
          <Badge variant="success">{t('status.done')}</Badge>
        ) : status === 'canceled' ? (
          <Badge variant="secondary">{t('status.canceled')}</Badge>
        ) : (
          <Badge variant="secondary">{t('status.open')}</Badge>
        )}
      </div>
      <p className="text-xs text-muted-foreground">
        {status === 'done' && completedAt
          ? t('row.completedAt', { date: formatDateTime(completedAt, formatting) })
          : dueAt
            ? t('row.due', { date: formatDateTime(dueAt, formatting) })
            : t('timeline.activityCreated')}
      </p>
      {item.body && <p className="text-sm whitespace-pre-line break-words text-muted-foreground">{item.body}</p>}
      <RecordLink record={item.record} />
      <ActivityMapLink location={timelineLocation(item.metadata.location)} />
    </div>
  )
}

export function NoteTimelineContent({ item }: { item: TimelineItem }) {
  const { t } = useTranslation('tasks')
  const isPrivate = item.metadata.visibility === 'private'
  return (
    <div className="grid min-w-0 gap-1">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-sm font-medium">{t('timeline.note')}</span>
        {isPrivate && <Badge variant="outline">{t('timeline.privateNote')}</Badge>}
        {item.metadata.pinned === true && <Badge variant="secondary">{t('timeline.pinnedNote')}</Badge>}
      </div>
      <Markdown source={item.body ?? item.title} className="text-muted-foreground" />
    </div>
  )
}

export function DealStageTimelineContent({ item }: { item: TimelineItem }) {
  const { t } = useTranslation('tasks')
  const payload = payloadOf(item)
  const from = nameOf(payload.from)
  const to = nameOf(payload.to)
  const text =
    from && to
      ? t('timeline.stageChanged', { from, to })
      : to
        ? t('timeline.stageSet', { to })
        : t('timeline.stageChangedGeneric')
  return <Line text={text} item={item} />
}

export function DealChangeTimelineContent({ item }: { item: TimelineItem }) {
  const { t } = useTranslation('tasks')
  const formatting = useRecordFormatting()
  const note = eventType(item) === 'deal.lost' ? asText(payloadOf(item).note) : null
  return (
    <div className="grid min-w-0 gap-1">
      <Line text={dealChangeText(item, t, formatting)} item={item} />
      {note && <p className="text-sm whitespace-pre-line break-words text-muted-foreground">{note}</p>}
    </div>
  )
}
