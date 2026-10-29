import type { CustomFieldValues, FieldConfig, FieldDefinition, FieldSchema, FieldType, RecordItem } from './types'

export function fieldConfig(field: FieldDefinition): FieldConfig {
  return (field.config ?? {}) as FieldConfig
}

export function activeFields(schema: FieldSchema): FieldDefinition[] {
  return schema.fields.filter((field) => field.is_active).sort((left, right) => left.position - right.position)
}

export function isFormulaField(field: FieldDefinition): boolean {
  return field.field_type === 'formula'
}

const FORMULA_VALUE_TYPES: Record<string, FieldType> = {
  number: 'number',
  currency: 'currency',
  date: 'date',
  text: 'text',
  boolean: 'checkbox',
}

export function valueFieldType(field: FieldDefinition): FieldType {
  if (!isFormulaField(field)) return field.field_type
  const resultType = fieldConfig(field).result_type
  return (resultType && FORMULA_VALUE_TYPES[resultType]) || 'text'
}

export function asValueField(field: FieldDefinition): FieldDefinition {
  return isFormulaField(field) ? { ...field, field_type: valueFieldType(field) } : field
}

export function isReadOnlyField(field: FieldDefinition): boolean {
  return field.access === 'read_only' || isFormulaField(field)
}

export function findField(schema: FieldSchema, key: string): FieldDefinition | undefined {
  return schema.fields.find((field) => field.key === key)
}

export function getFieldValue(record: RecordItem, field: FieldDefinition): unknown {
  if (field.is_system) return record[field.key] ?? null
  return record.custom_fields?.[field.key] ?? null
}

export type RecordValues = Record<string, unknown>

export function recordValues(record: RecordItem, fields: FieldDefinition[]): RecordValues {
  return Object.fromEntries(fields.map((field) => [field.key, getFieldValue(record, field)]))
}

export interface RecordPayload {
  custom_fields?: CustomFieldValues
  [key: string]: unknown
}

export function toRecordPayload(fields: FieldDefinition[], values: RecordValues): RecordPayload {
  const payload: RecordPayload = {}
  const custom: CustomFieldValues = {}
  for (const field of fields) {
    if (!(field.key in values)) continue
    const value = values[field.key]
    if (field.is_system) payload[field.key] = value
    else custom[field.key] = value
  }
  if (Object.keys(custom).length > 0) payload.custom_fields = custom
  return payload
}

export function errorPathToFieldKey(path: string): string {
  return path.startsWith('custom_fields.') ? path.slice('custom_fields.'.length) : path
}

export interface FieldViolation {
  field: string
  message: string
}

export function fieldViolations(details: unknown): FieldViolation[] {
  if (typeof details !== 'object' || details === null) return []
  const fields = (details as { fields?: unknown }).fields
  if (!Array.isArray(fields)) return []
  return fields.filter(
    (item): item is FieldViolation =>
      typeof item === 'object' &&
      item !== null &&
      typeof (item as FieldViolation).field === 'string' &&
      typeof (item as FieldViolation).message === 'string',
  )
}
