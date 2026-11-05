import { api } from '@/api'
import type {ActivityPage} from '@/sdk'
import { useMemo } from 'react'
import { useSession } from '@/app/session/session-context'
import {activitiesApi, activitiesKeys, type Activity, type ActivityType, type CreateActivityRequest, type InboxQuery} from '@/sdk'
import { addDays, todayKey, zonedDayStart } from '@/sdk/calendar/dates'
import { useRecordFormatting } from '@/sdk'
import { submitWithOutbox } from '@/pwa/offline/outbox'

export const AGENDA_CACHE_KEY = 'agenda:today'
export const TASKS_CACHE_KEY = 'tasks:today'
export const TODAY_TASK_GROUPS = ['overdue', 'today'] as const
export const CHECK_IN_TYPE_KEYS = ['visit', 'meeting'] as const

interface TodayRange { from: string; to: string; ownerId: string }

export function useTodayRange(): TodayRange {
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
  agenda: (range: TodayRange) => ({
    cacheKey: AGENDA_CACHE_KEY,
    queryKey: [...activitiesKeys.all, 'agenda', range],
    queryFn: async () => {
      const items: Activity[] = []
      for (let page = 1; items.length < 1000; page++) {
        const result = await api.get<ActivityPage>('/v1/tasks', { query: { page, per_page: 100, sort: 'due_at', 'filter[due_at][gte]': range.from, 'filter[due_at][lt]': range.to, 'filter[owner_id][eq]': range.ownerId } })
        items.push(...result.data)
        if (!result.data.length || items.length >= result.meta.total) break
      }
      return { items }
    },
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
  return submitWithOutbox<Activity>({ kind: 'activity', path: '/v1/tasks', body: request, label })
}
