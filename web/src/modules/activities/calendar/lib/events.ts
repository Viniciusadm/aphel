import type { Activity } from '@/modules/activities/api'
import {
  MINUTES_PER_DAY,
  formatMinutes,
  groupByDay,
  spanSegments,
  zonedMinutes,
  type DaySegment,
} from '@/features/records/calendar/dates'
import { DEFAULT_EVENT_MINUTES } from './range'

export interface CalendarEntry {
  key: string
  activity: Activity
  segment: DaySegment
}

export function eventStart(activity: Activity, timeZone: string): number {
  return activity.due_at ? (zonedMinutes(activity.due_at, timeZone) ?? 0) : 0
}

export function eventDuration(activity: Activity): number {
  return activity.duration_minutes && activity.duration_minutes > 0 ? activity.duration_minutes : DEFAULT_EVENT_MINUTES
}

export function eventSegments(activity: Activity, timeZone: string): DaySegment[] {
  if (!activity.due_at) return []
  return spanSegments(activity.due_at, Math.max(0, activity.duration_minutes ?? 0), timeZone)
}

export function groupEntriesByDay(items: Activity[], timeZone: string): Map<string, CalendarEntry[]> {
  const entries = items.flatMap((activity) =>
    eventSegments(activity, timeZone).map((segment, index) => ({
      key: index === 0 ? activity.id : `${activity.id}@${segment.day}`,
      activity,
      segment,
    })),
  )
  return groupByDay(entries, (entry) => entry.segment.day)
}

export function eventTimeLabel(activity: Activity, timeZone: string, locale: string): string {
  const start = eventStart(activity, timeZone)
  const label = formatMinutes(start, locale)
  if (!activity.duration_minutes || activity.duration_minutes <= 0) return label
  const total = start + activity.duration_minutes
  const end = formatMinutes(total % MINUTES_PER_DAY, locale)
  const extraDays = Math.floor(total / MINUTES_PER_DAY)
  return extraDays > 0 ? `${label}–${end} (+${extraDays})` : `${label}–${end}`
}

export function isFinished(activity: Activity): boolean {
  return activity.status === 'done' || activity.status === 'canceled'
}
