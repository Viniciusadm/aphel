import type { RecordItem } from '@/features/records/types'
import type { LeadTarget } from './DisqualifyLeadDialog'

export function leadTarget(record: RecordItem): LeadTarget {
  return { id: record.id, name: typeof record.name === 'string' ? record.name : '' }
}
