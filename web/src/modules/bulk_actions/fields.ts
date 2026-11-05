import { type FieldDefinition, type FieldSchema } from '@/sdk'

function writable(field: FieldDefinition): boolean {
  return field.access === 'editable' && field.field_type !== 'formula'
}

export function bulkEditableFields(schema: FieldSchema, system: readonly string[]): FieldDefinition[] {
  return schema.fields
    .filter((field) => field.is_active && writable(field) && (field.is_system ? system.includes(field.key) : true))
    .filter((field) => field.key !== 'owner_id')
    .sort((left, right) => left.position - right.position)
}
