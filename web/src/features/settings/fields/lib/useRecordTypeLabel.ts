import { useTranslation } from 'react-i18next'
import { useCustomObjectsAdmin } from '../../objects/api'
import { isKnownRecordType } from './field-types'

export function useRecordTypeLabel(recordType: string): string {
  const { t } = useTranslation('fields')
  const objects = useCustomObjectsAdmin()
  if (isKnownRecordType(recordType)) return t(`recordTypes.${recordType}`)
  return objects.data?.find((object) => object.record_type === recordType)?.label_plural ?? recordType
}
