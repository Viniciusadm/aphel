import { useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { relationshipRecordTypes, useCustomObjectsAdmin, type CustomObjectAdmin } from './api'

export interface TypeOption {
  value: string
  singular: string
  plural: string
}

export function useRecordTypeLabels() {
  const { t } = useTranslation('objects')
  const objects = useCustomObjectsAdmin()
  const custom = useMemo(() => objects.data ?? [], [objects.data])

  const label = useCallback(
    (recordType: string, plural = false): string => {
      if (relationshipRecordTypes().includes(recordType)) {
        return plural ? t(`recordTypesPlural.${recordType}`, { defaultValue: recordType }) : t(`recordTypes.${recordType}`, { defaultValue: recordType })
      }
      const object = custom.find((item) => item.record_type === recordType)
      if (!object) return recordType
      return plural ? object.label_plural : object.label_singular
    },
    [t, custom],
  )

  const options = useCallback(
    (include?: string): TypeOption[] => [
      ...relationshipRecordTypes().map((type) => ({
        value: type,
        singular: label(type),
        plural: label(type, true),
      })),
      ...custom
        .filter((object: CustomObjectAdmin) => object.is_active || object.record_type === include)
        .map((object) => ({ value: object.record_type, singular: object.label_singular, plural: object.label_plural })),
    ],
    [label, custom],
  )

  return { label, options }
}
