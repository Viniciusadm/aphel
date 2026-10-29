import { useTranslation } from 'react-i18next'
import { formatTimelineTime } from './grouping'
import { useModuleState } from '@/kernel/modules/registry'
import { resolveTimelineRenderer } from './registry'
import type { TimelineItem } from './types'

export interface TimelineEntryProps {
  item: TimelineItem
  timeZone: string
  locale: string
  isLast?: boolean
}

export function TimelineEntry({ item, timeZone, locale, isLast = false }: TimelineEntryProps) {
  const { t } = useTranslation('records-detail')
  const renderer = resolveTimelineRenderer(item.kind, item.metadata.event_type, useModuleState())
  const Icon = renderer.icon

  return (
    <li data-kind={item.kind} className="relative flex gap-3 pb-5 last:pb-0">
      {!isLast && <span aria-hidden="true" className="absolute top-8 bottom-0 left-4 w-px -translate-x-1/2 bg-border" />}
      <span className="relative flex size-8 shrink-0 items-center justify-center rounded-full border bg-muted text-muted-foreground">
        <Icon className="size-4" aria-hidden="true" />
      </span>
      <div className="grid min-w-0 flex-1 gap-1 pt-1">
        <div className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5 text-xs text-muted-foreground">
          <span className="font-medium text-foreground">{item.actor?.display_name ?? t('timeline.system')}</span>
          <time dateTime={item.occurred_at}>{formatTimelineTime(item.occurred_at, timeZone, locale)}</time>
        </div>
        {renderer.render(item)}
      </div>
    </li>
  )
}
