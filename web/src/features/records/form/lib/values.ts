import { activeFields, fieldConfig, getFieldValue, toRecordPayload, type RecordPayload, type RecordValues } from '../../fields'
import type { FieldDefinition, FieldSchema, RecordItem } from '../../types'
import { isBlank } from './validation'

export const READ_ONLY_SYSTEM_KEYS = ['id', 'created_at', 'updated_at', 'archived_at', 'deleted_at', 'created_by', 'updated_by']

export type RecordFormLayout = string[]

export interface ResolveFieldsOptions {
  layout?: RecordFormLayout
  readOnlyKeys?: string[]
}

export function isEditableField(field: FieldDefinition, readOnlyKeys: string[] = READ_ONLY_SYSTEM_KEYS): boolean {
  return field.is_active && !(field.is_system && readOnlyKeys.includes(field.key))
}

export function resolveFormFields(
  source: FieldSchema | FieldDefinition[],
  { layout, readOnlyKeys = READ_ONLY_SYSTEM_KEYS }: ResolveFieldsOptions = {},
): FieldDefinition[] {
  const fields = Array.isArray(source)
    ? source.filter((field) => field.is_active).sort((left, right) => left.position - right.position)
    : activeFields(source)
  const editable = fields.filter((field) => isEditableField(field, readOnlyKeys))
  if (!layout) return editable
  const byKey = new Map(editable.map((field) => [field.key, field]))
  return [...new Set(layout)].flatMap((key) => {
    const field = byKey.get(key)
    return field ? [field] : []
  })
}

export function quickCreateFields(
  schema: FieldSchema,
  layout?: RecordFormLayout,
  readOnlyKeys: string[] = READ_ONLY_SYSTEM_KEYS,
): FieldDefinition[] {
  if (layout) return resolveFormFields(schema, { layout, readOnlyKeys })
  const editable = resolveFormFields(schema, { readOnlyKeys })
  const required = editable.filter((field) => field.is_required)
  return required.length > 0 ? required : editable
}

function cloneValue(value: unknown): unknown {
  return Array.isArray(value) ? [...value] : value
}

export function initialFormValues(
  fields: FieldDefinition[],
  { record, defaultValues }: { record?: RecordItem; defaultValues?: RecordValues },
): RecordValues {
  return Object.fromEntries(
    fields.map((field) => {
      if (record) return [field.key, cloneValue(getFieldValue(record, field))]
      if (defaultValues && field.key in defaultValues) return [field.key, cloneValue(defaultValues[field.key])]
      const fallback = fieldConfig(field).default_value
      return [field.key, fallback === undefined ? null : cloneValue(fallback)]
    }),
  )
}

export function valuesEqual(left: unknown, right: unknown): boolean {
  if (isBlank(left) && isBlank(right)) return true
  if (Array.isArray(left) && Array.isArray(right)) {
    return left.length === right.length && left.every((item, index) => Object.is(item, right[index]))
  }
  if (typeof left === 'number' || typeof right === 'number') {
    return left !== null && right !== null && Number(left) === Number(right)
  }
  return Object.is(left, right)
}

export function normalizeSubmitValue(field: FieldDefinition, value: unknown): unknown {
  if (field.field_type === 'checkbox') return value === true || value === false ? value : null
  if (isBlank(value)) return null
  if ((field.field_type === 'number' || field.field_type === 'currency') && typeof value === 'string') {
    const number = Number(value)
    return Number.isFinite(number) ? number : value
  }
  return value
}

export function buildSubmitPayload(
  fields: FieldDefinition[],
  values: RecordValues,
  mode: 'create' | 'edit',
  initialValues: RecordValues,
): RecordPayload {
  const selected: RecordValues = {}
  for (const field of fields) {
    const value = normalizeSubmitValue(field, values[field.key])
    if (mode === 'create' && value === null) continue
    if (mode === 'edit' && valuesEqual(value, initialValues[field.key])) continue
    selected[field.key] = value
  }
  return toRecordPayload(fields, selected)
}
