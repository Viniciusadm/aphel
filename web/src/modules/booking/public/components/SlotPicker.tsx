import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { CalendarX2, ChevronLeft, ChevronRight } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import { bookingKeys } from '../api'
import { addDays, dayKey, formatDayLabel, formatLongDay, formatRangeLabel, formatTime, zonedDayStart } from '../lib/time'
import type { PublicSlot, PublicSlotsResponse, SlotRange } from '../types'
import { TimeZonePicker } from './TimeZonePicker'

const DAYS_PER_WEEK = 7
const MAX_WEEKS_AHEAD = 52

interface SlotPickerProps {
  owner: string
  fetchSlots: (range: SlotRange) => Promise<PublicSlotsResponse>
  timeZone: string
  onTimeZoneChange: (timeZone: string) => void
  selected: PublicSlot | null
  onSelect: (slot: PublicSlot) => void
}

function useWeek(timeZone: string, weekOffset: number) {
  const [now] = useState(() => Date.now())
  return useMemo(() => {
    const today = dayKey(new Date(now), timeZone)
    const start = addDays(today, weekOffset * DAYS_PER_WEEK)
    const days = Array.from({ length: DAYS_PER_WEEK }, (_, index) => addDays(start, index))
    const from = new Date(Math.max(zonedDayStart(start, timeZone).getTime(), now))
    const to = zonedDayStart(addDays(start, DAYS_PER_WEEK), timeZone)
    const range: SlotRange = { from: from.toISOString(), to: to.toISOString() }
    return { days, range }
  }, [now, timeZone, weekOffset])
}

export function SlotPicker({ owner, fetchSlots, timeZone, onTimeZoneChange, selected, onSelect }: SlotPickerProps) {
  const { t } = useTranslation('booking', { keyPrefix: 'public' })
  const [weekOffset, setWeekOffset] = useState(0)
  const [chosenDay, setChosenDay] = useState<string | null>(null)
  const { days, range } = useWeek(timeZone, weekOffset)
  const slots = useQuery({
    queryKey: bookingKeys.slotRange(owner, range),
    queryFn: () => fetchSlots(range),
    placeholderData: keepPreviousData,
  })

  const slotsByDay = useMemo(() => {
    const grouped = new Map<string, PublicSlot[]>()
    for (const slot of slots.data?.slots ?? []) {
      const key = dayKey(new Date(slot.starts_at), timeZone)
      grouped.set(key, [...(grouped.get(key) ?? []), slot])
    }
    return grouped
  }, [slots.data, timeZone])

  const firstAvailable = days.find((day) => (slotsByDay.get(day)?.length ?? 0) > 0) ?? null
  const activeDay = chosenDay && days.includes(chosenDay) ? chosenDay : firstAvailable
  const daySlots = activeDay ? (slotsByDay.get(activeDay) ?? []) : []
  const weekEmpty = !slots.isPlaceholderData && slots.isSuccess && firstAvailable === null

  const moveWeek = (delta: number) => {
    setChosenDay(null)
    setWeekOffset((current) => Math.min(MAX_WEEKS_AHEAD, Math.max(0, current + delta)))
  }

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-1">
          <Button
            variant="outline"
            size="icon"
            aria-label={t('slots.previousWeek')}
            disabled={weekOffset === 0}
            onClick={() => moveWeek(-1)}
          >
            <ChevronLeft />
          </Button>
          <span className="min-w-36 text-center text-sm font-medium">
            {formatRangeLabel(days[0], days[days.length - 1])}
          </span>
          <Button
            variant="outline"
            size="icon"
            aria-label={t('slots.nextWeek')}
            disabled={weekOffset >= MAX_WEEKS_AHEAD}
            onClick={() => moveWeek(1)}
          >
            <ChevronRight />
          </Button>
        </div>
        <TimeZonePicker value={timeZone} onChange={onTimeZoneChange} />
      </div>
      <div className="grid grid-cols-7 gap-1" role="group" aria-label={t('slots.days')}>
        {days.map((day) => {
          const label = formatDayLabel(day)
          const count = slotsByDay.get(day)?.length ?? 0
          const active = day === activeDay
          return (
            <button
              key={day}
              type="button"
              disabled={count === 0}
              aria-pressed={active}
              onClick={() => setChosenDay(day)}
              className={cn(
                'flex flex-col items-center gap-0.5 rounded-md border px-1 py-2 text-xs transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-40',
                active ? 'border-primary-solid bg-primary-solid text-primary-foreground' : 'bg-background hover:bg-accent',
              )}
            >
              <span className="capitalize">{label.weekday}</span>
              <span className="text-base font-semibold">{label.day}</span>
              <span className="capitalize">{label.month}</span>
            </button>
          )
        })}
      </div>
      {slots.isPending ? (
        <div className="grid grid-cols-3 gap-2 sm:grid-cols-4">
          {Array.from({ length: 8 }, (_, index) => (
            <Skeleton key={index} className="h-9" />
          ))}
        </div>
      ) : slots.isError ? (
        <ErrorState description={t('slots.loadError')} onRetry={() => void slots.refetch()} />
      ) : weekEmpty ? (
        <EmptyState
          icon={CalendarX2}
          title={t('slots.emptyTitle')}
          description={t('slots.emptyDescription')}
          action={
            weekOffset < MAX_WEEKS_AHEAD ? (
              <Button variant="outline" size="sm" onClick={() => moveWeek(1)}>
                {t('slots.nextWeek')}
              </Button>
            ) : undefined
          }
        />
      ) : (
        activeDay && (
          <div className={cn('grid gap-2', slots.isFetching && 'opacity-60')}>
            <p className="text-sm font-medium first-letter:uppercase">{formatLongDay(activeDay)}</p>
            <ul className="grid grid-cols-3 gap-2 sm:grid-cols-4">
              {daySlots.map((slot) => {
                const isSelected = selected?.starts_at === slot.starts_at
                return (
                  <li key={slot.starts_at}>
                    <Button
                      variant={isSelected ? 'default' : 'outline'}
                      className="w-full"
                      aria-pressed={isSelected}
                      onClick={() => onSelect(slot)}
                    >
                      {formatTime(slot.starts_at, timeZone)}
                    </Button>
                  </li>
                )
              })}
            </ul>
          </div>
        )
      )}
    </div>
  )
}
