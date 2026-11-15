import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { entityLabel, useEntities } from '@/sdk/entities'

export interface RecordTypeOption {
  value: string
  label: string
}

export function useRecordTypeOptions(): RecordTypeOption[] {
  const { i18n } = useTranslation()
  const entities = useEntities()
  return useMemo(
    () =>
      (entities.data ?? [])
        .filter((entity) => 'ui.plural_labels' in entity.traits)
        .map((entity) => ({
          value: entity.record_type,
          label: entityLabel(entity, i18n.language, entity.record_type),
        })),
    [entities.data, i18n.language],
  )
}

export function useIsKnownRecordType(recordType: string): boolean {
  const entities = useEntities()
  return (entities.data ?? []).some((entity) => entity.record_type === recordType)
}
