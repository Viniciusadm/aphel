import { MINUTES_PER_DAY, formatMinutes, spanSegments, zonedMinutes, type DaySegment } from './dates'
export interface ScheduledRecord { due_at?: string | null; duration_minutes?: number | null }
const DEFAULT_EVENT_MINUTES = 30
export function eventStart(activity: ScheduledRecord, timeZone: string): number {
  return activity.due_at ? (zonedMinutes(activity.due_at, timeZone) ?? 0) : 0
}

export function eventDuration(activity: ScheduledRecord): number {
  return activity.duration_minutes && activity.duration_minutes > 0 ? activity.duration_minutes : DEFAULT_EVENT_MINUTES
}

export function eventSegments(activity: ScheduledRecord, timeZone: string): DaySegment[] {
  if (!activity.due_at) return []
  return spanSegments(activity.due_at, Math.max(0, activity.duration_minutes ?? 0), timeZone)
}

export function eventTimeLabel(activity: ScheduledRecord, timeZone: string, locale: string): string {
  const start = eventStart(activity, timeZone)
  const label = formatMinutes(start, locale)
  if (!activity.duration_minutes || activity.duration_minutes <= 0) return label
  const total = start + activity.duration_minutes
  const end = formatMinutes(total % MINUTES_PER_DAY, locale)
  const extraDays = Math.floor(total / MINUTES_PER_DAY)
  return extraDays > 0 ? `${label}–${end} (+${extraDays})` : `${label}–${end}`
}

