import { api } from '@/api'
import type { TimelinePage } from '@/features/records/timeline'

export const TIMELINE_MODULE_KEY = 'timeline'

export interface TimelineTarget {
  recordType: string
  recordId: string
}

export const timelineApi = {
  page: (target: TimelineTarget, cursor: string | null, kinds: string[]) =>
    api.get<TimelinePage>('/v1/timeline', {
      query: {
        record_type: target.recordType,
        record_id: target.recordId,
        cursor: cursor ?? undefined,
        kinds: kinds.length > 0 ? kinds.join(',') : undefined,
      },
    }),
}
