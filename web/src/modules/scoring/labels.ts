import { useTranslation } from 'react-i18next'
import { useCustomObjects } from '@/features/custom-records'
import { SCORING_CORE_RECORD_TYPES } from './types'

type CoreRecordType = (typeof SCORING_CORE_RECORD_TYPES)[number]

export function isCoreScoringType(recordType: string): recordType is CoreRecordType {
  return (SCORING_CORE_RECORD_TYPES as readonly string[]).includes(recordType)
}

export function useScoringRecordTypeLabel() {
  const { t } = useTranslation('scoring')
  const objects = useCustomObjects()
  return (recordType: string): string => {
    if (isCoreScoringType(recordType)) return t(`recordTypes.${recordType}`)
    return objects.data?.find((object) => object.record_type === recordType)?.label_plural ?? recordType
  }
}

export function useScoringRecordTypes(current?: string): string[] {
  const objects = useCustomObjects()
  const types: string[] = [...SCORING_CORE_RECORD_TYPES]
  for (const object of objects.data ?? []) types.push(object.record_type)
  if (current && !types.includes(current)) types.push(current)
  return types
}
