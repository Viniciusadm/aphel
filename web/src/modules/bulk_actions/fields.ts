import type { FieldDefinition, FieldSchema } from '@/features/records/types'
import { BULK_SYSTEM_FIELDS, type BulkRecordType } from './api'

function writable(field: FieldDefinition): boolean {
  return field.access === 'editable' && field.field_type !== 'formula'
}

export function bulkEditableFields(schema: FieldSchema, recordType: BulkRecordType): FieldDefinition[] {
  const system = BULK_SYSTEM_FIELDS[recordType]
  return schema.fields
    .filter((field) => field.is_active && writable(field) && (field.is_system ? system.includes(field.key) : true))
    .filter((field) => field.key !== 'owner_id')
    .sort((left, right) => left.position - right.position)
}
