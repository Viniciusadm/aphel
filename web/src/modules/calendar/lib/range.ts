import {
  MONTH_GRID_DAYS,
  addDays,
  addMonths,
  daysBetween,
  formatDayTitle,
  formatMonthTitle,
  formatShortDay,
  monthGridStart,
  startOfWeek,
} from '@/sdk/calendar/dates'

export type CalendarViewMode = 'month' | 'week' | 'day'

export const CALENDAR_VIEW_MODES: CalendarViewMode[] = ['month', 'week', 'day']
export const DEFAULT_EVENT_MINUTES = 30
export const SNAP_MINUTES = 15
export const CREATE_SNAP_MINUTES = 30

export interface DayRange {
  from: string
  to: string
  days: string[]
}

export function parseViewMode(raw: string | null): CalendarViewMode {
  return raw === 'week' || raw === 'day' ? raw : 'month'
}

export function viewRange(mode: CalendarViewMode, anchor: string): DayRange {
  const from = mode === 'month' ? monthGridStart(anchor) : mode === 'week' ? startOfWeek(anchor) : anchor
  const length = mode === 'month' ? MONTH_GRID_DAYS : mode === 'week' ? 7 : 1
  const to = addDays(from, length)
  return { from, to, days: daysBetween(from, to) }
}

export function shiftAnchor(mode: CalendarViewMode, anchor: string, direction: 1 | -1): string {
  if (mode === 'month') return addMonths(anchor, direction)
  return addDays(anchor, mode === 'week' ? 7 * direction : direction)
}

export function viewTitle(mode: CalendarViewMode, anchor: string, locale: string): string {
  if (mode === 'month') return formatMonthTitle(anchor, locale)
  if (mode === 'day') return formatDayTitle(anchor, locale)
  const { from, to } = viewRange('week', anchor)
  return `${formatShortDay(from, locale)} – ${formatShortDay(addDays(to, -1), locale)}`
}

export function snapMinutes(minutes: number, step: number = SNAP_MINUTES): number {
  const snapped = Math.round(minutes / step) * step
  return Math.max(0, Math.min(24 * 60 - step, snapped))
}
