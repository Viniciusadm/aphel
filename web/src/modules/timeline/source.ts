import i18n from '@/i18n'
import type { TimelineKindOption, TimelineSource } from '@/features/records/timeline'
import { timelineApi, type TimelineTarget } from './api'
export async function timelineKindOptions(target: TimelineTarget): Promise<TimelineKindOption[]> {
  const kinds = await timelineApi.kinds(target)
  const locale = i18n.resolvedLanguage ?? i18n.language
  return kinds.map((kind) => ({ kind: kind.key, label: kind.labels[locale] ?? kind.labels[locale.split('-')[0] ?? ''] ?? kind.labels.en ?? kind.key }))
}
export function recordTimelineSource(recordType: string, recordId: string): TimelineSource {
  const target = { recordType, recordId }
  return { fetchPage: ({ cursor, kinds }) => timelineApi.page(target, cursor, kinds), fetchKindOptions: () => timelineKindOptions(target) }
}
