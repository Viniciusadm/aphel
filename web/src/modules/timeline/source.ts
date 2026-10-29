import i18n from '@/i18n'
import type { TimelineKindOption, TimelineSource } from '@/features/records/timeline'
import { TIMELINE_MODULE_KEY, timelineApi } from './api'

export const TIMELINE_KINDS = [
  'activity',
  'note',
  'deal_stage',
  'deal_change',
  'record_change',
  'attachment',
  'event',
] as const

export type TimelineKindKey = (typeof TIMELINE_KINDS)[number]

export function timelineKindOptions(): TimelineKindOption[] {
  return TIMELINE_KINDS.map((kind) => ({ kind, label: i18n.t(`timeline.kinds.${kind}`, { ns: TIMELINE_MODULE_KEY }) }))
}

export function recordTimelineSource(recordType: string, recordId: string): TimelineSource {
  const target = { recordType, recordId }
  return {
    fetchPage: ({ cursor, kinds }) => timelineApi.page(target, cursor, kinds),
    kindOptions: timelineKindOptions(),
  }
}
