import type { CalendarRecord } from '../api'
import {
  groupByDay,
  type DaySegment,
} from '@/sdk/calendar/dates'
import { eventSegments } from '@/sdk/calendar/events'
export { eventStart, eventDuration, eventSegments, eventTimeLabel } from '@/sdk/calendar/events'

export interface CalendarEntry {
  key: string
  activity: CalendarRecord
  segment: DaySegment
}

export function groupEntriesByDay(items: CalendarRecord[], timeZone: string): Map<string, CalendarEntry[]> {
  const entries = items.flatMap((activity) =>
    eventSegments(activity, timeZone).map((segment, index) => ({
      key: index === 0 ? activity.id : `${activity.id}@${segment.day}`,
      activity,
      segment,
    })),
  )
  return groupByDay(entries, (entry) => entry.segment.day)
}

export function isFinished(activity: CalendarRecord): boolean {
  return activity.finished
}

type Translate = (key: string, options?: Record<string, unknown>) => string

export function categoryLabel(t: unknown, category: { source?: string; id: string; name: string }): string {
  return category.id === category.source ? (t as Translate)(`sources.${category.source}`, { defaultValue: category.name }) : category.name
}
