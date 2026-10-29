import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { getRecordResource } from '../resources'
import type { TimelineItem } from './types'

export function GenericTimelineContent({ item }: { item: TimelineItem }) {
  const { t } = useTranslation('records-detail')
  const record = item.record
  const href = record ? getRecordResource(record.record_type)?.detailPath?.(record.record_id) : undefined
  const recordLabel = record?.title || t('timeline.relatedRecord')

  return (
    <div className="grid min-w-0 gap-1">
      <p className="text-sm font-medium break-words">{item.title}</p>
      {item.body && <p className="text-sm whitespace-pre-line break-words text-muted-foreground">{item.body}</p>}
      {record &&
        (href ? (
          <Link to={href} className="w-fit max-w-full truncate text-sm text-primary underline-offset-4 hover:underline">
            {recordLabel}
          </Link>
        ) : (
          <span className="truncate text-sm text-muted-foreground">{recordLabel}</span>
        ))}
    </div>
  )
}
