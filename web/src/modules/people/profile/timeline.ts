import { useMemo } from 'react'
import { api } from '@/api'
import { recordTimelineSource } from '@/modules/timeline'
import { type TimelineFetchPage, type TimelinePage, type TimelineSource } from '@/sdk'
export type ProfileRecordType = 'people.person' | 'organizations.organization'

export function timelineFetchPage(recordType: ProfileRecordType, recordId: string): TimelineFetchPage {
  return ({ cursor, kinds }) =>
    api.get<TimelinePage>('/v1/timeline', {
      query: {
        record_type: recordType,
        record_id: recordId,
        cursor: cursor ?? undefined,
        kinds: kinds.length > 0 ? kinds.join(',') : undefined,
      },
    })
}

export function useProfileTimeline(recordType: ProfileRecordType, recordId: string): TimelineSource {
  return useMemo(() => recordTimelineSource(recordType, recordId), [recordType, recordId])
}
