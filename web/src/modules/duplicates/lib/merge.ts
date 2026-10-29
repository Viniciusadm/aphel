import { isEmptyValue } from '@/features/records/display/values'
import { activeFields, getFieldValue, isReadOnlyField } from '@/features/records/fields'
import type { FieldDefinition, FieldSchema, RecordItem } from '@/features/records/types'

export type MergeSide = 'primary' | 'duplicate'

export function mergeableFields(schema: FieldSchema, mergeFields: readonly string[]): FieldDefinition[] {
  return activeFields(schema).filter((field) =>
    field.is_system ? mergeFields.includes(field.key) : !isReadOnlyField(field),
  )
}

function sameValue(left: unknown, right: unknown): boolean {
  if (isEmptyValue(left) && isEmptyValue(right)) return true
  return JSON.stringify(left) === JSON.stringify(right)
}

export interface MergeRow {
  field: FieldDefinition
  primary: unknown
  duplicate: unknown
  same: boolean
}

export function mergeRows(fields: FieldDefinition[], primary: RecordItem, duplicate: RecordItem): MergeRow[] {
  return fields.map((field) => {
    const primaryValue = getFieldValue(primary, field)
    const duplicateValue = getFieldValue(duplicate, field)
    return { field, primary: primaryValue, duplicate: duplicateValue, same: sameValue(primaryValue, duplicateValue) }
  })
}

export function defaultChoices(rows: MergeRow[]): Record<string, MergeSide> {
  return Object.fromEntries(
    rows
      .filter((row) => !row.same)
      .map((row) => [
        row.field.key,
        isEmptyValue(row.primary) && !isEmptyValue(row.duplicate) ? 'duplicate' : 'primary',
      ]),
  )
}

export function movedTotal(moved: Record<string, number>): number {
  return Object.values(moved).reduce((total, count) => total + count, 0)
}
