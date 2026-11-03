import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

export type CalendarRecord = components['schemas']['CalendarRecord']
export type CalendarResponse = components['schemas']['CalendarResponse']

export interface CalendarRange {
  from: string
  to: string
  ownerId?: string
  teamId?: string
  categoryId?: string
  source?: string
}

export const calendarKeys = {
  all: ['calendar'] as const,
  range: (range: CalendarRange) => ['calendar', range] as const,
}

export function useCalendarEntries(range: CalendarRange) {
  return useQuery({
    queryKey: calendarKeys.range(range),
    queryFn: ({ signal }) => api.get<CalendarResponse>('/v1/calendar', {
      query: {
        from: range.from,
        to: range.to,
        owner_id: range.ownerId,
        team_id: range.teamId,
        category_id: range.categoryId,
        source: range.source,
      },
      signal,
    }),
    placeholderData: keepPreviousData,
  })
}

export function useRescheduleEntry() {
  const client = useQueryClient()
  return useMutation({
    mutationFn: ({ activity, dueAt }: { activity: CalendarRecord; dueAt: string }) =>
      api.patch<void>(
        `/v1/calendar/${encodeURIComponent(activity.source)}/${encodeURIComponent(activity.record_id)}`,
        { due_at: dueAt },
        { headers: { 'If-Match': activity.updated_at } },
      ),
    onSettled: () => client.invalidateQueries({ queryKey: calendarKeys.all }),
  })
}
