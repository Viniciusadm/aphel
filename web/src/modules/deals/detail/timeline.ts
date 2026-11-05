import { useMemo } from 'react'
import { type TimelineSource, recordTimelineSource } from '@/sdk'
import { DEAL_RECORD_TYPE } from '../resource'

export function useDealTimeline(dealId: string): TimelineSource {
  return useMemo(() => recordTimelineSource(DEAL_RECORD_TYPE, dealId), [dealId])
}
