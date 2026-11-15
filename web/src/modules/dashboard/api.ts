import { useInfiniteQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

export type ActivityPage = components['schemas']['ActivityPageResponse']

export interface ActivityFilters {
  days: number
  recordType: string
  actorId: string
}

export const ACTIVITY_PER_PAGE = 25

export function useActivity(filters: ActivityFilters) {
  return useInfiniteQuery({
    queryKey: ['dashboard', 'activity', filters],
    initialPageParam: '',
    queryFn: ({ pageParam }) =>
      api.get<ActivityPage>('/v1/dashboard/activity', {
        query: {
          per_page: ACTIVITY_PER_PAGE,
          days: filters.days,
          ...(filters.recordType ? { record_type: filters.recordType } : {}),
          ...(filters.actorId ? { actor_id: filters.actorId } : {}),
          ...(pageParam ? { cursor: pageParam } : {}),
        },
      }),
    getNextPageParam: (last) => last.meta.next_cursor ?? undefined,
  })
}
