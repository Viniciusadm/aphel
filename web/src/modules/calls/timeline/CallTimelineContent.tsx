import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { activityResource } from '@/modules/tasks/resource'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { getRecordResource } from '@/features/records/resources'
import type { TimelineItem } from '@/features/records/timeline'
import { CallSummary } from '../components/CallSummary'
import type { CallDirection } from '../types'

function asObject(value: unknown): Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value) ? (value as Record<string, unknown>) : {}
}

function asText(value: unknown): string | null {
  return typeof value === 'string' && value.trim() !== '' ? value : null
}

export function CallTimelineContent({ item }: { item: TimelineItem }) {
  const { t } = useTranslation('calls')
  const formatting = useRecordFormatting()
  const call = asObject(item.metadata.call)
  const outcome = asObject(call.outcome)
  const activityId = asText(item.metadata.activity_id)
  const href = activityId ? activityResource.detailPath?.(activityId) : undefined
  const completedAt = asText(item.metadata.completed_at)
  const direction: CallDirection = call.direction === 'inbound' ? 'inbound' : 'outbound'
  const recordHref = item.record ? getRecordResource(item.record.record_type)?.detailPath?.(item.record.record_id) : undefined
  return (
    <div className="grid min-w-0 gap-1">
      {href ? (
        <Link to={href} className="min-w-0 truncate text-sm font-medium hover:underline">
          {item.title}
        </Link>
      ) : (
        <span className="min-w-0 truncate text-sm font-medium">{item.title}</span>
      )}
      <CallSummary
        direction={direction}
        outcomeKey={asText(outcome.key) ?? ''}
        outcomeName={asText(outcome.name)}
        connected={outcome.is_connected === true}
        durationSeconds={typeof call.duration_seconds === 'number' ? call.duration_seconds : null}
      />
      {completedAt && (
        <p className="text-xs text-muted-foreground">
          {t('summary.loggedAt', { date: formatDateTime(completedAt, formatting) })}
        </p>
      )}
      {item.body && <p className="text-sm whitespace-pre-line break-words text-muted-foreground">{item.body}</p>}
      {item.record &&
        (recordHref ? (
          <Link to={recordHref} className="w-fit max-w-full truncate text-sm text-primary underline-offset-4 hover:underline">
            {item.record.title ?? t('summary.relatedRecord')}
          </Link>
        ) : (
          <span className="truncate text-sm text-muted-foreground">{item.record.title ?? t('summary.relatedRecord')}</span>
        ))}
    </div>
  )
}
