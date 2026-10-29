import { useDroppable } from '@dnd-kit/core'
import { useEffect, useRef, type MouseEvent } from 'react'
import { useTranslation } from 'react-i18next'
import type { Activity } from '@/modules/activities/api'
import {
  MINUTES_PER_DAY,
  formatDayNumber,
  formatDayTitle,
  formatMinutes,
  formatWeekdayShort,
} from '@/features/records/calendar/dates'
import { useRecordFormatting } from '@/features/records/formatting'
import { cn } from '@/lib/utils'
import { layoutDay } from '../lib/layout'
import { eventDuration, type CalendarEntry } from '../lib/events'
import { CREATE_SNAP_MINUTES, snapMinutes } from '../lib/range'
import { CalendarEvent } from './CalendarEvent'

export const COLUMN_DROP_PREFIX = 'col:'
export const HOUR_HEIGHT = 48
export const SCROLL_TO_HOUR = 7
const HOURS = Array.from({ length: 24 }, (_, hour) => hour)
const MIN_EVENT_MINUTES = 20

export interface TimeGridViewProps {
  days: string[]
  todayKey: string
  itemsByDay: Map<string, CalendarEntry[]>
  draggable: boolean
  onReschedule?: (activity: Activity) => void
  onCreate?: (dayKey: string, minutes: number) => void
  onOpenDay: (dayKey: string) => void
}

function DayColumn({
  dayKey,
  items,
  draggable,
  onReschedule,
  onCreate,
}: {
  dayKey: string
  items: CalendarEntry[]
  draggable: boolean
  onReschedule?: (activity: Activity) => void
  onCreate?: (dayKey: string, minutes: number) => void
}) {
  const { t } = useTranslation('activities', { keyPrefix: 'calendar' })
  const formatting = useRecordFormatting()
  const { setNodeRef, isOver } = useDroppable({ id: `${COLUMN_DROP_PREFIX}${dayKey}` })
  const positioned = layoutDay(
    items.map((entry) => {
      const { start, end } = entry.segment
      const minimum = end > start ? MIN_EVENT_MINUTES : Math.max(eventDuration(entry.activity), MIN_EVENT_MINUTES)
      const top = Math.min(start, MINUTES_PER_DAY - MIN_EVENT_MINUTES)
      return {
        item: entry,
        start: top,
        end: Math.min(MINUTES_PER_DAY, Math.max(end, top + minimum)),
      }
    }),
  )
  const handleClick = (event: MouseEvent<HTMLDivElement>) => {
    if (!onCreate || event.target !== event.currentTarget) return
    const rect = event.currentTarget.getBoundingClientRect()
    const minutes = ((event.clientY - rect.top) / rect.height) * MINUTES_PER_DAY
    onCreate(dayKey, snapMinutes(minutes - CREATE_SNAP_MINUTES / 2, CREATE_SNAP_MINUTES))
  }
  return (
    <div
      ref={setNodeRef}
      onClick={handleClick}
      aria-label={formatDayTitle(dayKey, formatting.locale)}
      className={cn('relative min-w-0 border-l', onCreate && 'cursor-cell', isOver && 'bg-primary/5')}
      style={{ height: HOUR_HEIGHT * 24 }}
    >
      {HOURS.map((hour) => (
        <div
          key={hour}
          aria-hidden
          className="pointer-events-none absolute inset-x-0 border-t border-border/60"
          style={{ top: hour * HOUR_HEIGHT }}
        />
      ))}
      {positioned.map((event) => (
        <CalendarEvent
          key={event.item.key}
          dragId={event.item.key}
          activity={event.item.activity}
          variant="block"
          draggable={draggable}
          onReschedule={onReschedule}
          style={{
            top: `${(event.start / MINUTES_PER_DAY) * 100}%`,
            height: `${((event.end - event.start) / MINUTES_PER_DAY) * 100}%`,
            left: `${(event.column / event.columns) * 100}%`,
            width: `${100 / event.columns}%`,
          }}
        />
      ))}
      {onCreate && <span className="sr-only">{t('create.hint')}</span>}
    </div>
  )
}

export function TimeGridView({
  days,
  todayKey,
  itemsByDay,
  draggable,
  onReschedule,
  onCreate,
  onOpenDay,
}: TimeGridViewProps) {
  const formatting = useRecordFormatting()
  const scroller = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (scroller.current) scroller.current.scrollTop = SCROLL_TO_HOUR * HOUR_HEIGHT
  }, [])
  const columns = { gridTemplateColumns: `3.5rem repeat(${days.length}, minmax(0, 1fr))` }

  return (
    <div className="min-w-0 overflow-hidden rounded-md border">
      <div className="grid border-b bg-muted/40" style={columns}>
        <div />
        {days.map((key) => (
          <button
            key={key}
            type="button"
            onClick={() => onOpenDay(key)}
            aria-label={formatDayTitle(key, formatting.locale)}
            className="flex min-w-0 flex-col items-center border-l px-1 py-1.5 text-xs hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
          >
            <span className="truncate text-muted-foreground capitalize">{formatWeekdayShort(key, formatting.locale)}</span>
            <span
              className={cn(
                'inline-flex size-7 items-center justify-center rounded-full text-sm font-medium',
                key === todayKey && 'bg-primary-solid text-primary-foreground',
              )}
            >
              {formatDayNumber(key)}
            </span>
          </button>
        ))}
      </div>
      <div ref={scroller} className="max-h-[70vh] overflow-y-auto">
        <div className="grid" style={columns}>
          <div className="relative" style={{ height: HOUR_HEIGHT * 24 }} aria-hidden>
            {HOURS.map((hour) => (
              <span
                key={hour}
                className="absolute right-1 -translate-y-1/2 text-[10px] tabular-nums text-muted-foreground"
                style={{ top: hour * HOUR_HEIGHT }}
              >
                {hour === 0 ? '' : formatMinutes(hour * 60, formatting.locale)}
              </span>
            ))}
          </div>
          {days.map((key) => (
            <DayColumn
              key={key}
              dayKey={key}
              items={itemsByDay.get(key) ?? []}
              draggable={draggable}
              onReschedule={onReschedule}
              onCreate={onCreate}
            />
          ))}
        </div>
      </div>
    </div>
  )
}
