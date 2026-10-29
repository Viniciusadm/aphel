import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { getRecordResource } from '@/features/records/resources'
import type { SearchHit } from './api'

export function searchHitPath(hit: Pick<SearchHit, 'id' | 'record_type'>): string | undefined {
  return getRecordResource(hit.record_type)?.detailPath?.(hit.id)
}

export function searchResultsUrl(query: string): string {
  return `/search?q=${encodeURIComponent(query.trim())}`
}

export function useSearchRecordTypeLabel() {
  const { t } = useTranslation('search')
  return useCallback(
    (recordType: string, plural = false) =>
      plural
        ? t(`recordTypesPlural.${recordType}` as 'recordTypesPlural.contact', { defaultValue: recordType })
        : t(`recordTypes.${recordType}` as 'recordTypes.contact', { defaultValue: recordType }),
    [t],
  )
}
