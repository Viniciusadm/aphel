import { useMemo } from 'react'
import type { TimelineSource } from '@/features/records/timeline'
import { recordTimelineSource } from '@/modules/timeline'
import { DEAL_RECORD_TYPE } from '../resource'

export function useDealTimeline(dealId: string): TimelineSource {
  return useMemo(() => recordTimelineSource(DEAL_RECORD_TYPE, dealId), [dealId])
}
