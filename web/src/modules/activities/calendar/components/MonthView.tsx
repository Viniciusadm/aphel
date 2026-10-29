import { useDroppable } from '@dnd-kit/core'
import { Plus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import type { Activity } from '@/modules/activities/api'
import { AgendaList } from '@/features/records/calendar/AgendaList'
import { formatDayTitle } from '@/features/records/calendar/dates'
import { MonthGrid, type DayCellProps } from '@/features/records/calendar/MonthGrid'
import { cn } from '@/lib/utils'
import type { CalendarEntry } from '../lib/events'
import { CalendarEvent, type CalendarEventVariant } from './CalendarEvent'

export const DAY_DROP_PREFIX = 'day:'

function DroppableDayCell({ dayKey, className, children }: DayCellProps) {
  const { setNodeRef, isOver } = useDroppable({ id: `${DAY_DROP_PREFIX}${dayKey}` })
  return (
    <div ref={setNodeRef} className={cn(className, isOver && 'bg-primary/10')}>
      {children}
    </div>
  )
}

export interface DayEventsProps {
  days: string[]
  anchor: string
  todayKey: string
  locale: string
  itemsByDay: Map<string, CalendarEntry[]>
  draggable: boolean
  onReschedule?: (activity: Activity) => void
  onCreate?: (dayKey: string) => void
  onOpenDay: (dayKey: string) => void
}

function CreateButton({ dayKey, locale, onCreate }: { dayKey: string; locale: string; onCreate: (key: string) => void }) {
  const { t } = useTranslation('activities', { keyPrefix: 'calendar' })
  return (
    <Button
      variant="ghost"
      size="icon"
      className="size-6 opacity-100 sm:opacity-0 sm:group-hover/day:opacity-100 sm:focus-visible:opacity-100"
      aria-label={t('create.onDay', { day: formatDayTitle(dayKey, locale) })}
      onClick={() => onCreate(dayKey)}
    >
      <Plus className="size-3.5" aria-hidden />
    </Button>
  )
}

export function MonthView({
  anchor,
  todayKey,
  locale,
  itemsByDay,
  draggable,
  onReschedule,
  onCreate,
  onOpenDay,
}: DayEventsProps) {
  const render = (variant: CalendarEventVariant) => (entry: CalendarEntry) => (
    <CalendarEvent
      dragId={entry.key}
      activity={entry.activity}
      variant={variant}
      draggable={draggable}
      onReschedule={onReschedule}
    />
  )
  return (
    <MonthGrid
      monthKey={anchor}
      todayKey={todayKey}
      locale={locale}
      itemsByDay={itemsByDay}
      getKey={(entry) => entry.key}
      renderItem={render('chip')}
      onDayClick={onOpenDay}
      onShowMore={onOpenDay}
      dayAction={onCreate ? (key) => <CreateButton dayKey={key} locale={locale} onCreate={onCreate} /> : undefined}
      Cell={DroppableDayCell}
    />
  )
}

export function AgendaView({
  days,
  todayKey,
  locale,
  itemsByDay,
  onReschedule,
  onCreate,
}: DayEventsProps) {
  return (
    <AgendaList
      days={days}
      todayKey={todayKey}
      locale={locale}
      itemsByDay={itemsByDay}
      getKey={(entry) => entry.key}
      renderItem={(entry) => (
        <CalendarEvent
          dragId={entry.key}
          activity={entry.activity}
          variant="agenda"
          draggable={false}
          onReschedule={onReschedule}
        />
      )}
      dayAction={onCreate ? (key) => <CreateButton dayKey={key} locale={locale} onCreate={onCreate} /> : undefined}
    />
  )
}
