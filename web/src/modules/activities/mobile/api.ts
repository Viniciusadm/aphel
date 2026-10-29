import { useMemo } from 'react'
import { useSession } from '@/app/session/session-context'
import {
  activitiesApi,
  activitiesKeys,
  type Activity,
  type ActivityType,
  type CalendarRange,
  type CreateActivityRequest,
  type InboxQuery,
} from '../api'
import { addDays, todayKey, zonedDayStart } from '@/features/records/calendar/dates'
import { useRecordFormatting } from '@/features/records/formatting'
import { submitWithOutbox } from '@/pwa/offline/outbox'

export const AGENDA_CACHE_KEY = 'agenda:today'
export const TASKS_CACHE_KEY = 'tasks:today'
export const TODAY_TASK_GROUPS = ['overdue', 'today'] as const
export const CHECK_IN_TYPE_KEYS = ['visit', 'meeting'] as const

export function useTodayRange(): CalendarRange {
  const { me } = useSession()
  const { timezone } = useRecordFormatting()
  const today = todayKey(timezone)
  return useMemo(
    () => ({
      from: zonedDayStart(today, timezone),
      to: zonedDayStart(addDays(today, 1), timezone),
      ownerId: me.user.id,
    }),
    [today, timezone, me.user.id],
  )
}

export const TODAY_INBOX_QUERY: InboxQuery = { owner: 'me', sort: 'due' }

export const todayQueries = {
  agenda: (range: CalendarRange) => ({
    cacheKey: AGENDA_CACHE_KEY,
    queryKey: activitiesKeys.calendar(range),
    queryFn: () => activitiesApi.calendar(range),
  }),
  tasks: () => ({
    cacheKey: TASKS_CACHE_KEY,
    queryKey: activitiesKeys.inbox(TODAY_INBOX_QUERY),
    queryFn: () => activitiesApi.inbox(TODAY_INBOX_QUERY),
  }),
}

export function pickActivityType(types: ActivityType[] | undefined, keys: readonly string[]): ActivityType | undefined {
  const active = (types ?? []).filter((type) => type.is_active)
  for (const key of keys) {
    const match = active.find((type) => type.key === key)
    if (match) return match
  }
  return active[0]
}

export function submitActivity(request: CreateActivityRequest, label: string) {
  return submitWithOutbox<Activity>({ kind: 'activity', path: '/v1/activities', body: request, label })
}
