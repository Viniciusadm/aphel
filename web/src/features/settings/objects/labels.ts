import { useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { CORE_RELATIONSHIP_TYPES, useCustomObjectsAdmin, type CustomObjectAdmin } from './api'

type CoreType = (typeof CORE_RELATIONSHIP_TYPES)[number]

function isCoreType(value: string): value is CoreType {
  return (CORE_RELATIONSHIP_TYPES as readonly string[]).includes(value)
}

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
      if (isCoreType(recordType)) {
        return plural ? t(`recordTypesPlural.${recordType}`) : t(`recordTypes.${recordType}`)
      }
      const object = custom.find((item) => item.record_type === recordType)
      if (!object) return recordType
      return plural ? object.label_plural : object.label_singular
    },
    [t, custom],
  )

  const options = useCallback(
    (include?: string): TypeOption[] => [
      ...CORE_RELATIONSHIP_TYPES.map((type) => ({
        value: type,
        singular: t(`recordTypes.${type}`),
        plural: t(`recordTypesPlural.${type}`),
      })),
      ...custom
        .filter((object: CustomObjectAdmin) => object.is_active || object.record_type === include)
        .map((object) => ({ value: object.record_type, singular: object.label_singular, plural: object.label_plural })),
    ],
    [t, custom],
  )

  return { label, options }
}
