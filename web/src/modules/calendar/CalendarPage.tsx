import { SegmentedControl } from '@/components/ui/segmented-control'
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  TouchSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from '@dnd-kit/core'
import { useQueryClient } from '@tanstack/react-query'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import {
  calendarKeys,
  useCalendarEntries,
  useRescheduleEntry,
  type CalendarRecord,
  type CalendarRange,
  type CalendarResponse,
} from './api'
import { useSlotEntries } from '@/kernel/modules/slots'
import {
  MINUTES_PER_DAY,
  sameMonth,
  todayKey as currentDayKey,
  zonedDayStart,
  zonedInstant,
} from '@/sdk/calendar/dates'
import { SMALL_SCREEN_QUERY, useMediaQuery, useRecordFormatting } from '@/sdk'
import { cn } from '@/lib/utils'
import { CalendarFilters } from './components/CalendarFilters'
import { AgendaView, DAY_DROP_PREFIX, MonthView } from './components/MonthView'
import { RescheduleDialog } from './components/RescheduleDialog'
import { COLUMN_DROP_PREFIX, TimeGridView } from './components/TimeGridView'
import { eventStart, groupEntriesByDay } from './lib/events'
import { CALENDAR_VIEW_MODES, DEFAULT_EVENT_MINUTES, shiftAnchor, snapMinutes, viewRange, viewTitle } from './lib/range'
import { useCalendarUrlState } from './useCalendarUrlState'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const DEFAULT_CREATE_MINUTES = 9 * 60
const EMPTY_ITEMS: CalendarRecord[] = []

interface CreateDefaults {
  dueAt: string | null
  durationMinutes: number | null
}

export function CalendarPage() {
  const { t } = useTranslation('calendar')
  const formatting = useRecordFormatting()
  const timeZone = formatting.timezone
  const creators = useSlotEntries('calendar.create')
  const Creator = creators[0]?.Component
  const queryClient = useQueryClient()
  const [state, update] = useCalendarUrlState()
  const small = useMediaQuery(SMALL_SCREEN_QUERY)
  const today = currentDayKey(timeZone)
  const anchor = state.date ?? today
  const range = viewRange(state.mode, anchor)
  const canCreate = creators.length > 0

  const calendarRange = useMemo<CalendarRange>(
    () => ({
      from: zonedDayStart(range.from, timeZone),
      to: zonedDayStart(range.to, timeZone),
      ownerId: state.ownerId || undefined,
      teamId: state.teamId || undefined,
      categoryId: state.typeId || undefined,
      source: state.sourceId || undefined,
    }),
    [range.from, range.to, timeZone, state.ownerId, state.teamId, state.typeId, state.sourceId],
  )
  const calendar = useCalendarEntries(calendarRange)
  const items = (calendar.data?.items as CalendarRecord[] | undefined) ?? EMPTY_ITEMS
  const canUpdate = items.some((item) => item.can_update)
  const itemsByDay = useMemo(() => groupEntriesByDay(items, timeZone), [items, timeZone])
  const updateEntry = useRescheduleEntry()
  const [rescheduling, setRescheduling] = useState<CalendarRecord | null>(null)
  const [creating, setCreating] = useState<CreateDefaults | null>(null)
  const [dragging, setDragging] = useState<CalendarRecord | null>(null)
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(TouchSensor, { activationConstraint: { delay: 250, tolerance: 6 } }),
  )

  const reschedule = async (activity: CalendarRecord, dueAt: string) => {
    setRescheduling(null)
    if (!activity.can_update || activity.due_at === dueAt) return
    const key = calendarKeys.range(calendarRange)
    const previous = queryClient.getQueryData<CalendarResponse>(key)
    queryClient.setQueryData<CalendarResponse>(key, (current) =>
      current
        ? { ...current, items: current.items.map((item) => (item.id === activity.id ? { ...item, due_at: dueAt } : item)) }
        : current,
    )
    try {
      await updateEntry.mutateAsync({ activity, dueAt })
      toast({ title: t('reschedule.done', { title: activity.title }), variant: 'success' })
    } catch (error) {
      queryClient.setQueryData(key, previous)
      toast({
        title: isApiError(error) && error.status === 409 ? t('reschedule.conflict') : t('reschedule.failed'),
        variant: 'destructive',
      })
    }
  }

  const onDragStart = (event: DragStartEvent) => {
    setDragging((event.active.data.current?.activity as CalendarRecord | undefined) ?? null)
  }

  const onDragEnd = (event: DragEndEvent) => {
    setDragging(null)
    const activity = event.active.data.current?.activity as CalendarRecord | undefined
    const target = event.over?.id
    if (!activity || typeof target !== 'string' || !event.over) return
    let dueAt: string | null = null
    if (target.startsWith(DAY_DROP_PREFIX)) {
      dueAt = zonedInstant(target.slice(DAY_DROP_PREFIX.length), eventStart(activity, timeZone), timeZone)
    } else if (target.startsWith(COLUMN_DROP_PREFIX)) {
      const translated = event.active.rect.current.translated
      const rect = event.over.rect
      if (!translated || rect.height <= 0) return
      const minutes = snapMinutes(((translated.top - rect.top) / rect.height) * MINUTES_PER_DAY)
      dueAt = zonedInstant(target.slice(COLUMN_DROP_PREFIX.length), minutes, timeZone)
    }
    if (dueAt) void reschedule(activity, dueAt)
  }

  const openDay = (key: string) => update({ mode: 'day', date: key })
  const createOnDay = canCreate
    ? (key: string) =>
        setCreating({ dueAt: zonedInstant(key, DEFAULT_CREATE_MINUTES, timeZone), durationMinutes: null })
    : undefined
  const createAt = canCreate
    ? (key: string, minutes: number) =>
        setCreating({ dueAt: zonedInstant(key, minutes, timeZone), durationMinutes: DEFAULT_EVENT_MINUTES })
    : undefined
  const onReschedule = canUpdate ? setRescheduling : undefined
  const useAgenda = small && state.mode !== 'day'
  const agendaDays = state.mode === 'month' ? range.days.filter((key) => sameMonth(key, anchor)) : range.days

  let body
  if (calendar.isError && !calendar.data) {
    body = <ErrorState onRetry={() => void calendar.refetch()} />
  } else if (calendar.isPending) {
    body = <Skeleton className="h-[32rem] w-full" />
  } else if (useAgenda) {
    body = (
      <AgendaView
        days={agendaDays}
        anchor={anchor}
        todayKey={today}
        locale={formatting.locale}
        itemsByDay={itemsByDay}
        draggable={false}
        onReschedule={onReschedule}
        onCreate={createOnDay}
        onOpenDay={openDay}
      />
    )
  } else if (state.mode === 'month') {
    body = (
      <MonthView
        days={range.days}
        anchor={anchor}
        todayKey={today}
        locale={formatting.locale}
        itemsByDay={itemsByDay}
        draggable={canUpdate}
        onReschedule={onReschedule}
        onCreate={createOnDay}
        onOpenDay={openDay}
      />
    )
  } else {
    body = (
      <TimeGridView
        key={`${state.mode}-${range.from}`}
        days={range.days}
        todayKey={today}
        itemsByDay={itemsByDay}
        draggable={canUpdate}
        onReschedule={onReschedule}
        onCreate={createAt}
        onOpenDay={openDay}
      />
    )
  }

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <Button variant="outline" size="sm" onClick={() => update({ date: null })}>
          {t('nav.today')}
        </Button>
        <div className="flex items-center">
          <Button
            variant="ghost"
            size="icon"
            aria-label={t('nav.previous')}
            onClick={() => update({ date: shiftAnchor(state.mode, anchor, -1) })}
          >
            <ChevronLeft aria-hidden />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            aria-label={t('nav.next')}
            onClick={() => update({ date: shiftAnchor(state.mode, anchor, 1) })}
          >
            <ChevronRight aria-hidden />
          </Button>
        </div>
        <h2 className="min-w-0 flex-1 truncate text-base font-semibold first-letter:uppercase" aria-live="polite">
          {viewTitle(state.mode, anchor, formatting.locale)}
        </h2>
        <SegmentedControl
          aria-label={t('views.label')}
          size="sm"
          value={state.mode}
          onValueChange={(mode) => update({ mode })}
          options={CALENDAR_VIEW_MODES.map((mode) => ({ value: mode, label: t(`views.${mode}`) }))}
        />
      </div>
      <CalendarFilters
        ownerId={state.ownerId}
        teamId={state.teamId}
        typeId={state.typeId}
        sourceId={state.sourceId}
        onChange={(patch) => update(patch)}
        categories={calendar.data?.categories ?? []}
      />
      {calendar.data?.truncated && (
        <p role="status" className="text-sm text-muted-foreground">
          {t('truncated', { shown: items.length, total: calendar.data.total })}
        </p>
      )}
      {canUpdate && !useAgenda && (
        <p className={cn('hidden text-xs text-muted-foreground sm:block')}>{t('dragHint')}</p>
      )}
      <DndContext sensors={sensors} onDragStart={onDragStart} onDragEnd={onDragEnd} onDragCancel={() => setDragging(null)}>
        {body}
        <DragOverlay dropAnimation={null}>
          {dragging ? (
            <div
              className="w-48 truncate rounded border-l-[3px] bg-popover px-2 py-1 text-xs font-medium shadow-md"
              style={{ borderLeftColor: dragging.category.color || undefined }}
            >
              {dragging.title}
            </div>
          ) : null}
        </DragOverlay>
      </DndContext>
      <RescheduleDialog
        activity={rescheduling}
        onOpenChange={(open) => !open && setRescheduling(null)}
        onConfirm={(activity, dueAt) => void reschedule(activity, dueAt)}
      />
      {Creator && <Creator open={creating !== null} dueAt={creating?.dueAt ?? null} durationMinutes={creating?.durationMinutes ?? null} onOpenChange={(open) => { if (!open) setCreating(null) }} />}
    </Page>
  )
}
