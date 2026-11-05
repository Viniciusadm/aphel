import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useCustomObjects } from '@/sdk'
import {CORE_RECORD_TYPES, QUOTE_RECORD_TYPE} from '@/sdk'

const KNOWN_RECORD_TYPES: readonly string[] = [...CORE_RECORD_TYPES, QUOTE_RECORD_TYPE]

export function useRecordTypeLabel() {
  const { t } = useTranslation('approvals')
  const objects = useCustomObjects()
  return useCallback(
    (recordType: string) => {
      if (KNOWN_RECORD_TYPES.includes(recordType)) {
        return t(`recordTypes.${recordType as (typeof CORE_RECORD_TYPES)[number] | typeof QUOTE_RECORD_TYPE}`)
      }
      return objects.data?.find((object) => object.record_type === recordType)?.label_singular ?? recordType
    },
    [objects.data, t],
  )
}
