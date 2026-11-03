import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useCustomObjects } from '@/features/custom-records/api'
import { isCustomRecordType } from '@/features/custom-records/resources'

export const CORE_EMAIL_RECORD_TYPES = ['people.person', 'organizations.organization', 'deals.deal'] as const

export function supportsEmails(recordType: string): boolean {
  return (CORE_EMAIL_RECORD_TYPES as readonly string[]).includes(recordType) || isCustomRecordType(recordType)
}

export interface RecordTypeOption {
  value: string
  label: string
}

export function useEmailRecordTypes(): RecordTypeOption[] {
  const { t } = useTranslation('email')
  const objects = useCustomObjects()
  return useMemo(
    () => [
      ...CORE_EMAIL_RECORD_TYPES.map((recordType) => ({ value: recordType, label: t(`recordTypes.${recordType}`) })),
      ...(objects.data ?? [])
        .filter((object) => object.is_active)
        .map((object) => ({ value: object.record_type, label: object.label_singular })),
    ],
    [t, objects.data],
  )
}

export function useRecordTypeLabel() {
  const options = useEmailRecordTypes()
  return (recordType: string) => options.find((option) => option.value === recordType)?.label ?? recordType
}
