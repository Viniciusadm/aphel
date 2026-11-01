import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useCustomObjects } from '@/features/custom-records'
import { getRecordResource } from '@/features/records/resources'
import type { SearchHit } from './api'

export function searchHitPath(hit: Pick<SearchHit, 'id' | 'record_type'>): string | undefined {
  return getRecordResource(hit.record_type)?.detailPath?.(hit.id)
}

export function searchResultsUrl(query: string): string {
  return `/search?q=${encodeURIComponent(query.trim())}`
}

export function useInternalRecordTypes(): ReadonlySet<string> {
  const objects = useCustomObjects()
  return new Set((objects.data ?? []).filter((object) => object.is_internal).map((object) => object.record_type))
}

export function useSearchRecordTypeLabel() {
  const { t } = useTranslation('search')
  const objects = useCustomObjects()
  return useCallback(
    (recordType: string, plural = false) => {
      const custom = objects.data?.find((object) => object.record_type === recordType)
      const fallback = custom ? (plural ? custom.label_plural : custom.label_singular) : recordType
      return plural
        ? t(`recordTypesPlural.${recordType}` as 'recordTypesPlural.contact', { defaultValue: fallback })
        : t(`recordTypes.${recordType}` as 'recordTypes.contact', { defaultValue: fallback })
    },
    [t, objects.data],
  )
}
