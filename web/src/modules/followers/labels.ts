import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useCustomObjects } from '@/features/custom-records'
import { recordDetailHref } from '@/features/records/links'

export const CORE_RECORD_TYPES = ['contact', 'company', 'deal', 'activity', 'product', 'quote'] as const

type CoreRecordType = (typeof CORE_RECORD_TYPES)[number]

function isCoreRecordType(recordType: string): recordType is CoreRecordType {
  return (CORE_RECORD_TYPES as readonly string[]).includes(recordType)
}

export function useRecordTypeLabel() {
  const { t } = useTranslation('followers')
  const objects = useCustomObjects()
  return useCallback(
    (recordType: string) => {
      if (isCoreRecordType(recordType)) return t(`recordTypes.${recordType}`)
      return objects.data?.find((object) => object.record_type === recordType)?.label_singular ?? recordType
    },
    [objects.data, t],
  )
}

export function followedRecordHref(recordType: string, recordId: string): string | undefined {
  const href = recordDetailHref(recordType, recordId)
  if (href) return href
  if (recordType === 'quote') return `/quotes/${encodeURIComponent(recordId)}`
  const key = recordType.startsWith('co_') ? recordType.slice(3) : null
  return key ? `/objects/${encodeURIComponent(key)}/${encodeURIComponent(recordId)}` : undefined
}
