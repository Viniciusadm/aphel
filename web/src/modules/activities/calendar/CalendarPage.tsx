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
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import {
  activitiesKeys,
  useCalendarActivities,
  useUpdateActivity,
  type Activity,
  type CalendarRange,
  type CalendarResponse,
} from '@/modules/activities/api'
import { ActivityFormDialog } from '@/modules/activities/components/ActivityFormDialog'
import {
  MINUTES_PER_DAY,
  sameMonth,
  todayKey as currentDayKey,
  zonedDayStart,
  zonedInstant,
} from '@/features/records/calendar/dates'
import { SMALL_SCREEN_QUERY, useMediaQuery } from '@/features/records/calendar/useMediaQuery'
import { useRecordFormatting } from '@/features/records/formatting'
import { cn } from '@/lib/utils'
import { CalendarFilters } from './components/CalendarFilters'
import { AgendaView, DAY_DROP_PREFIX, MonthView } from './components/MonthView'
import { RescheduleDialog } from './components/RescheduleDialog'
import { COLUMN_DROP_PREFIX, TimeGridView } from './components/TimeGridView'
import { eventStart, groupEntriesByDay } from './lib/events'
import { CALENDAR_VIEW_MODES, DEFAULT_EVENT_MINUTES, shiftAnchor, snapMinutes, viewRange, viewTitle } from './lib/range'
import { useCalendarUrlState } from './useCalendarUrlState'

const DEFAULT_CREATE_MINUTES = 9 * 60
const EMPTY_ITEMS: Activity[] = []

interface CreateDefaults {
  dueAt: string | null
  durationMinutes: number | null
}

export function CalendarPage() {
  const { t } = useTranslation('activities', { keyPrefix: 'calendar' })
  const formatting = useRecordFormatting()
  const timeZone = formatting.timezone
  const { hasPermission } = useSession()
  const queryClient = useQueryClient()
  const [state, update] = useCalendarUrlState()
  const small = useMediaQuery(SMALL_SCREEN_QUERY)
  const today = currentDayKey(timeZone)
  const anchor = state.date ?? today
  const range = viewRange(state.mode, anchor)
  const canUpdate = hasPermission('activities.update')
  const canCreate = hasPermission('activities.create')

  const calendarRange = useMemo<CalendarRange>(
    () => ({
      from: zonedDayStart(range.from, timeZone),
      to: zonedDayStart(range.to, timeZone),
      ownerId: state.ownerId || undefined,
      teamId: state.teamId || undefined,
      activityTypeId: state.typeId || undefined,
    }),
    [range.from, range.to, timeZone, state.ownerId, state.teamId, state.typeId],
  )
  const calendar = useCalendarActivities(calendarRange)
  const items = (calendar.data?.items as Activity[] | undefined) ?? EMPTY_ITEMS
  const itemsByDay = useMemo(() => groupEntriesByDay(items, timeZone), [items, timeZone])
  const updateActivity = useUpdateActivity()
  const [rescheduling, setRescheduling] = useState<Activity | null>(null)
  const [creating, setCreating] = useState<CreateDefaults | null>(null)
  const [dragging, setDragging] = useState<Activity | null>(null)
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(TouchSensor, { activationConstraint: { delay: 250, tolerance: 6 } }),
  )

  const reschedule = async (activity: Activity, dueAt: string) => {
    setRescheduling(null)
    if (activity.due_at === dueAt) return
    const key = activitiesKeys.calendar(calendarRange)
    const previous = queryClient.getQueryData<CalendarResponse>(key)
    queryClient.setQueryData<CalendarResponse>(key, (current) =>
      current
        ? { ...current, items: current.items.map((item) => (item.id === activity.id ? { ...item, due_at: dueAt } : item)) }
        : current,
    )
    try {
      await updateActivity.mutateAsync({ activity, request: { due_at: dueAt } })
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
    setDragging((event.active.data.current?.activity as Activity | undefined) ?? null)
  }

  const onDragEnd = (event: DragEndEvent) => {
    setDragging(null)
    const activity = event.active.data.current?.activity as Activity | undefined
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
    <div className="flex w-full min-w-0 flex-col gap-4">
      <header className="grid gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">{t('page.title')}</h1>
        <p className="text-sm text-muted-foreground">{t('page.description')}</p>
      </header>
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
        <h2 className="min-w-0 flex-1 truncate text-base font-semibold capitalize" aria-live="polite">
          {viewTitle(state.mode, anchor, formatting.locale)}
        </h2>
        <div role="group" aria-label={t('views.label')} className="inline-flex rounded-md border p-0.5">
          {CALENDAR_VIEW_MODES.map((mode) => (
            <Button
              key={mode}
              variant={state.mode === mode ? 'secondary' : 'ghost'}
              size="sm"
              className="h-7"
              aria-pressed={state.mode === mode}
              onClick={() => update({ mode })}
            >
              {t(`views.${mode}`)}
            </Button>
          ))}
        </div>
      </div>
      <CalendarFilters
        ownerId={state.ownerId}
        teamId={state.teamId}
        typeId={state.typeId}
        onChange={(patch) => update(patch)}
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
              style={{ borderLeftColor: dragging.activity_type.color || undefined }}
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
      <ActivityFormDialog
        open={creating !== null}
        mode="task"
        defaults={creating ?? undefined}
        onOpenChange={(open) => !open && setCreating(null)}
      />
    </div>
  )
}
