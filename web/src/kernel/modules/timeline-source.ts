import type { TimelineSource } from '@/features/records/timeline/types'
import { activeContributions, useModuleState } from './registry'

export type TimelineSourceFactory = (recordType: string, recordId: string) => TimelineSource

export function useModuleTimelineSource(): TimelineSourceFactory | undefined {
  const current = useModuleState()
  return activeContributions<TimelineSourceFactory>('timelineSource', current).at(-1)?.value
}
