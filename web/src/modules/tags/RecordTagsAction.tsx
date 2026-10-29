import type { RecordSlotProps } from '@/kernel/modules/slots'
import { RecordTagsControl } from './RecordTagsControl'

export function RecordTagsAction({ recordType, record }: RecordSlotProps) {
  return <RecordTagsControl recordType={recordType} recordId={record.id} />
}
