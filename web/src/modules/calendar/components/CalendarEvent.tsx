import { useDraggable } from '@dnd-kit/core'
import { CalendarClock, ExternalLink, UserRound } from 'lucide-react'
import type { CSSProperties } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import type { CalendarRecord } from '../api'

import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { cn } from '@/lib/utils'
import { eventTimeLabel, isFinished } from '../lib/events'

export type CalendarEventVariant = 'chip' | 'block' | 'agenda'

export interface CalendarEventProps {
  dragId?: string
  activity: CalendarRecord
  variant: CalendarEventVariant
  draggable: boolean
  onReschedule?: (activity: CalendarRecord) => void
  style?: CSSProperties
}

const DEFAULT_COLOR = '#6366f1'

export function CalendarEvent({ dragId, activity, variant, draggable, onReschedule, style }: CalendarEventProps) {
  const { t } = useTranslation('calendar')
  const formatting = useRecordFormatting()
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: dragId ?? activity.id,
    data: { activity },
    disabled: !draggable || !activity.can_update,
  })
  const { role: _role, 'aria-describedby': _describedBy, ...dragAttributes } = attributes
  const finished = isFinished(activity)
  const color = activity.category.color || DEFAULT_COLOR
  const time = eventTimeLabel(activity, formatting.timezone, formatting.locale)

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={cn(variant === 'block' && 'absolute px-0.5', isDragging && 'opacity-40')}
    >
      <Popover>
        <PopoverTrigger asChild>
          <button
            type="button"
            {...dragAttributes}
            {...listeners}
            aria-label={t('event.label', { title: activity.title, time })}
            className={cn(
              'flex w-full min-w-0 touch-manipulation gap-1 overflow-hidden rounded border-l-[3px] bg-muted text-left text-xs text-foreground hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
              variant === 'chip' && 'items-center px-1.5 py-0.5',
              variant === 'block' && 'h-full flex-col px-1.5 py-1 shadow-xs',
              variant === 'agenda' && 'items-center gap-2 px-2 py-2 text-sm',
              draggable && activity.can_update && 'cursor-grab active:cursor-grabbing',
            )}
            style={{ borderLeftColor: color }}
          >

            <span className="shrink-0 tabular-nums text-muted-foreground">{time}</span>
            <span className={cn('min-w-0 truncate font-medium', finished && 'text-muted-foreground line-through')}>
              {activity.title}
            </span>
          </button>
        </PopoverTrigger>
        <PopoverContent className="grid gap-3">
          <div className="flex items-start gap-2">
            <div className="grid min-w-0 gap-0.5">
              <p className={cn('text-sm font-semibold break-words', finished && 'line-through')}>{activity.title}</p>
              <p className="text-xs text-muted-foreground">
                {activity.category.name}
              </p>
            </div>
          </div>
          <div className="grid gap-1 text-xs text-muted-foreground">
            {activity.due_at && (
              <span className="inline-flex items-center gap-1.5">
                <CalendarClock className="size-3.5" aria-hidden />
                {formatDateTime(activity.due_at, formatting)}
                {activity.duration_minutes ? ` · ${t('event.duration', { count: activity.duration_minutes })}` : ''}
              </span>
            )}
            {activity.owner && (
              <span className="inline-flex items-center gap-1.5">
                <UserRound className="size-3.5" aria-hidden />
                {activity.owner.display_name}
              </span>
            )}
          </div>
          <div className="flex flex-wrap gap-2">
            <Button asChild size="sm" variant="outline">
              <Link to={activity.href}>
                <ExternalLink aria-hidden />
                {t('event.open')}
              </Link>
            </Button>
            {onReschedule && activity.can_update && (
              <Button size="sm" onClick={() => onReschedule(activity)}>
                <CalendarClock aria-hidden />
                {t('event.reschedule')}
              </Button>
            )}
          </div>
        </PopoverContent>
      </Popover>
    </div>
  )
}
