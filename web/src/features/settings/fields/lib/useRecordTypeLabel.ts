import { useCustomObjectsAdmin } from '@/modules/objects/ui/api'
import { useRecordTypeOptions } from './record-types'

export function useRecordTypeLabel(recordType: string): string {
  const options = useRecordTypeOptions()
  const objects = useCustomObjectsAdmin()
  return (
    options.find((option) => option.value === recordType)?.label ??
    objects.data?.find((object) => object.record_type === recordType)?.label_plural ??
    recordType
  )
}
