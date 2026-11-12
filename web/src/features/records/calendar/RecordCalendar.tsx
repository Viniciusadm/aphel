import { NativeSelect } from '@/components/ui/native-select'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { createContext, useContext, useState, type DragEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { api } from '@/api'
import { notifyError } from '@/components/feedback/notify'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import { recordsKeys } from '../api'
import { recordDetailApi } from '../detail/api'
import { getFieldValue, isReadOnlyField, toRecordPayload } from '../fields'
import { useRecordFormatting } from '../formatting'
import { recordTitle, type RecordResource } from '../resources'
import { recordListParams } from '../table/query'
import type { ArchivedMode, FieldDefinition, FilterNode, Page, RecordItem } from '../types'
import { AgendaList } from './AgendaList'
import { rangeFilter } from './query'
import {
  MONTH_GRID_DAYS,
  addDays,
  addMonths,
  daysBetween,
  formatMinutes,
  formatMonthTitle,
  formatShortDay,
  groupByDay,
  isDayKey,
  monthGridStart,
  sameMonth,
  startOfMonth,
  startOfWeek,
  todayKey as currentDayKey,
  weekStartFor,
  zonedDayKey,
  zonedInstant,
  zonedMinutes,
} from '@/sdk/calendar/dates'
import { MonthGrid, type DayCellProps } from './MonthGrid'
import { SMALL_SCREEN_QUERY, useMediaQuery } from './useMediaQuery'
import { WeekColumns } from './WeekColumns'

export const RECORD_CALENDAR_PAGE_SIZE = 100
export const RECORD_CALENDAR_MAX_PAGES = 5

export interface RecordCalendarProps {
  recordType: string
  resource: RecordResource
  field: FieldDefinition
  fieldOptions?: { key: string; label: string }[]
  onFieldChange?: (key: string) => void
  filter: FilterNode | null
  search: string
  archived?: ArchivedMode
  onRecordClick?: (record: RecordItem) => void
}

interface CalendarRecords {
  items: RecordItem[]
  total: number
}

type RecordCalendarMode = 'month' | 'week'

const RECORD_CALENDAR_MODES: RecordCalendarMode[] = ['month', 'week']
const DAYS_PER_WEEK = 7
const DRAG_MIME = 'application/x-aphel-record'

interface DropState {
  enabled: boolean
  target: string | null
  setTarget: (day: string | null) => void
  drop: (recordId: string, day: string) => void
}

const DropContext = createContext<DropState | null>(null)

function DropCell({ dayKey, className, children }: DayCellProps) {
  const state = useContext(DropContext)
  if (!state?.enabled) return <div className={className}>{children}</div>
  const accept = (event: DragEvent<HTMLDivElement>) => {
    if (!event.dataTransfer.types.includes(DRAG_MIME)) return
    event.preventDefault()
    event.dataTransfer.dropEffect = 'move'
    if (state.target !== dayKey) state.setTarget(dayKey)
  }
  return (
    <div
      className={cn(className, state.target === dayKey && 'bg-primary/5 ring-2 ring-primary/40 ring-inset')}
      onDragEnter={accept}
      onDragOver={accept}
      onDragLeave={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) state.setTarget(null)
      }}
      onDrop={(event) => {
        event.preventDefault()
        state.setTarget(null)
        const recordId = event.dataTransfer.getData(DRAG_MIME)
        if (recordId) state.drop(recordId, dayKey)
      }}
    >
      {children}
    </div>
  )
}

function withFieldValue(record: RecordItem, field: FieldDefinition, value: unknown): RecordItem {
  if (field.is_system) return { ...record, [field.key]: value }
  return { ...record, custom_fields: { ...record.custom_fields, [field.key]: value } }
}

export function RecordCalendar({
  recordType,
  resource,
  field,
  fieldOptions = [],
  onFieldChange,
  filter,
  search,
  archived,
  onRecordClick,
}: RecordCalendarProps) {
  const { t } = useTranslation('records-calendar')
  const formatting = useRecordFormatting()
  const navigate = useNavigate()
  const small = useMediaQuery(SMALL_SCREEN_QUERY)
  const queryClient = useQueryClient()
  const today = currentDayKey(formatting.timezone)
  const weekStartsOn = weekStartFor(formatting.locale)
  const [mode, setMode] = useState<RecordCalendarMode>('month')
  const [anchor, setAnchor] = useState(today)
  const [dropTarget, setDropTarget] = useState<string | null>(null)
  const monthKey = startOfMonth(anchor)
  const from = mode === 'month' ? monthGridStart(monthKey, weekStartsOn) : startOfWeek(anchor, weekStartsOn)
  const to = addDays(from, mode === 'month' ? MONTH_GRID_DAYS : DAYS_PER_WEEK)
  const combined = rangeFilter(field, from, to, formatting.timezone, filter)
  const canMove = !isReadOnlyField(field)

  const calendarKey = [...recordsKeys.list(recordType), 'calendar', field.key, from, to, combined, search, archived]
  const records = useQuery({
    queryKey: calendarKey,
    queryFn: async ({ signal }): Promise<CalendarRecords> => {
      const items: RecordItem[] = []
      let total = 0
      for (let page = 1; page <= RECORD_CALENDAR_MAX_PAGES; page += 1) {
        const result = await api.get<Page<RecordItem>>(resource.basePath, {
          query: recordListParams({
            page,
            perPage: RECORD_CALENDAR_PAGE_SIZE,
            filter: combined,
            sort: field.is_sortable ? [{ field: field.key, direction: 'asc' }] : undefined,
            search,
            archived,
          }),
          signal,
        })
        items.push(...result.data)
        total = result.meta.total
        if (page * RECORD_CALENDAR_PAGE_SIZE >= total) break
      }
      return { items, total }
    },
    placeholderData: keepPreviousData,
  })

  const dayOf = (record: RecordItem): string | null => {
    const value = getFieldValue(record, field)
    if (field.field_type === 'date') return isDayKey(value) ? value : null
    return typeof value === 'string' ? zonedDayKey(value, formatting.timezone) : null
  }
  const itemsByDay = groupByDay(records.data?.items ?? [], dayOf)

  const open = (record: RecordItem) => {
    if (onRecordClick) onRecordClick(record)
    else {
      const path = resource.detailPath?.(record.id)
      if (path) navigate(path)
    }
  }

  const moveTo = async (recordId: string, day: string) => {
    const record = records.data?.items.find((item) => item.id === recordId)
    if (!record || dayOf(record) === day) return
    const current = getFieldValue(record, field)
    let next: string | null = day
    if (field.field_type === 'datetime') {
      const minutes = typeof current === 'string' ? (zonedMinutes(current, formatting.timezone) ?? 0) : 0
      next = zonedInstant(day, minutes, formatting.timezone)
    }
    if (!next) return
    const previous = records.data
    queryClient.setQueryData<CalendarRecords>(calendarKey, (data) =>
      data
        ? { ...data, items: data.items.map((item) => (item.id === recordId ? withFieldValue(item, field, next) : item)) }
        : data,
    )
    try {
      await recordDetailApi.update(resource, recordId, toRecordPayload([field], { [field.key]: next }))
      void queryClient.invalidateQueries({ queryKey: recordsKeys.detail(recordType, recordId) })
    } catch (error) {
      queryClient.setQueryData(calendarKey, previous)
      notifyError(error)
    } finally {
      void queryClient.invalidateQueries({ queryKey: recordsKeys.list(recordType) })
    }
  }

  const dropState: DropState = {
    enabled: canMove,
    target: dropTarget,
    setTarget: setDropTarget,
    drop: (recordId, day) => void moveTo(recordId, day),
  }

  const renderItem = (record: RecordItem) => {
    const value = getFieldValue(record, field)
    const minutes =
      field.field_type === 'datetime' && typeof value === 'string' ? zonedMinutes(value, formatting.timezone) : null
    const title = recordTitle(resource, record)
    return (
      <button
        type="button"
        draggable={canMove && !small}
        onDragStart={(event) => {
          event.dataTransfer.setData(DRAG_MIME, record.id)
          event.dataTransfer.effectAllowed = 'move'
        }}
        onDragEnd={() => setDropTarget(null)}
        onClick={() => open(record)}
        className={cn(
          'flex w-full min-w-0 items-center gap-1 rounded border-l-2 border-primary bg-primary/10 px-1.5 py-0.5 text-left text-xs hover:bg-primary/20 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
          small && 'py-2 text-sm',
        )}
      >
        {minutes !== null && <span className="shrink-0 tabular-nums text-muted-foreground">{formatMinutes(minutes, formatting.locale)}</span>}
        <span className="truncate">{title}</span>
      </button>
    )
  }

  const shift = (direction: 1 | -1) =>
    setAnchor(mode === 'month' ? addMonths(anchor, direction) : addDays(anchor, DAYS_PER_WEEK * direction))

  const total = records.data?.total ?? 0
  const shown = records.data?.items.length ?? 0

  return (
    <div className="grid min-w-0 gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="outline" size="sm" onClick={() => setAnchor(today)}>
          {t('nav.today')}
        </Button>
        <div className="flex items-center">
          <Button variant="ghost" size="icon" aria-label={t('nav.previous')} onClick={() => shift(-1)}>
            <ChevronLeft aria-hidden />
          </Button>
          <Button variant="ghost" size="icon" aria-label={t('nav.next')} onClick={() => shift(1)}>
            <ChevronRight aria-hidden />
          </Button>
        </div>
        <h2 className="text-base font-semibold capitalize" aria-live="polite">
          {mode === 'month'
            ? formatMonthTitle(monthKey, formatting.locale)
            : `${formatShortDay(from, formatting.locale)} – ${formatShortDay(addDays(to, -1), formatting.locale)}`}
        </h2>
        {fieldOptions.length > 1 && onFieldChange ? (
          <NativeSelect
            aria-label={t('layout.field')}
            className="w-auto max-w-48"
            value={field.key}
            onChange={(event) => onFieldChange(event.target.value)}
          >
            {fieldOptions.map((option) => (
              <option key={option.key} value={option.key}>
                {option.label}
              </option>
            ))}
          </NativeSelect>
        ) : (
          <span className="text-xs text-muted-foreground">{t('records.byField', { field: field.label })}</span>
        )}
        <SegmentedControl
          aria-label={t('views.label')}
          size="sm"
          className="ml-auto"
          value={mode}
          onValueChange={setMode}
          options={RECORD_CALENDAR_MODES.map((option) => ({ value: option, label: t(`views.${option}`) }))}
        />
      </div>
      {total > shown && (
        <p role="status" className="text-sm text-muted-foreground">
          {t('records.truncated', { shown, total })}
        </p>
      )}
      {records.isError && !records.data ? (
        <ErrorState onRetry={() => void records.refetch()} />
      ) : records.isPending ? (
        <Skeleton className="h-96 w-full" />
      ) : small ? (
        <AgendaList
          days={daysBetween(from, to).filter((key) => mode === 'week' || sameMonth(key, monthKey))}
          todayKey={today}
          locale={formatting.locale}
          itemsByDay={itemsByDay}
          getKey={(record) => record.id}
          renderItem={renderItem}
          emptyLabel={t('records.empty')}
        />
      ) : (
        <DropContext.Provider value={dropState}>
          {mode === 'week' ? (
            <WeekColumns
              days={daysBetween(from, to)}
              todayKey={today}
              locale={formatting.locale}
              itemsByDay={itemsByDay}
              getKey={(record) => record.id}
              renderItem={renderItem}
              Cell={DropCell}
            />
          ) : (
            <MonthGrid
              monthKey={monthKey}
              todayKey={today}
              locale={formatting.locale}
              itemsByDay={itemsByDay}
              getKey={(record) => record.id}
              renderItem={renderItem}
              Cell={DropCell}
              weekStartsOn={weekStartsOn}
            />
          )}
        </DropContext.Provider>
      )}
    </div>
  )
}
