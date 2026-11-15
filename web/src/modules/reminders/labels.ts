import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { entityLabel, useEntities } from '@/sdk/entities'

export function useRecordTypeLabel() {
  const { i18n } = useTranslation()
  const entities = useEntities()
  return useCallback(
    (recordType: string) =>
      entityLabel(entities.data?.find((entity) => entity.record_type === recordType), i18n.language, recordType),
    [entities.data, i18n.language],
  )
}
