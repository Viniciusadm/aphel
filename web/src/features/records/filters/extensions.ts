import { useMemo, type ComponentType } from 'react'
import { useFilters } from '@/kernel/modules/filters'
import type { FieldDefinition, FieldSchema, FieldType, FilterCondition, FilterOperator, OperandKind } from '../types'

export interface FilterExtensionValueProps {
  recordType: string
  value: unknown
  onChange: (value: unknown) => void
  label: string
}

export interface FilterExtensionSummaryProps {
  recordType: string
  condition: FilterCondition
  label: string
}

export interface FilterFieldExtension {
  field: FieldDefinition
  operands?: Partial<Record<FilterOperator, OperandKind>>
  ValueInput?: ComponentType<FilterExtensionValueProps>
  Summary?: ComponentType<FilterExtensionSummaryProps>
}

declare module '@/kernel/modules/filters' {
  interface FilterMap {
    'records.filterFields': { value: FilterFieldExtension[]; context: { recordType: string } }
  }
}

export interface PseudoFieldInput {
  recordType: string
  key: string
  label: string
  fieldType?: FieldType
  operators: FilterOperator[]
}

export function pseudoFilterField({ recordType, key, label, fieldType = 'text', operators }: PseudoFieldInput): FieldDefinition {
  return {
    id: `filter-extension:${key}`,
    record_type: recordType,
    key,
    label,
    labels: {},
    field_type: fieldType,
    is_system: true,
    is_required: false,
    is_unique: false,
    is_searchable: false,
    is_personal: false,
    is_sensitive: false,
    is_active: true,
    is_filterable: true,
    is_sortable: false,
    position: Number.MAX_SAFE_INTEGER,
    config: {},
    options: [],
    operators,
    access: 'read_only',
  }
}

const NO_EXTENSIONS: FilterFieldExtension[] = []

export function useFilterExtensions(recordType: string | null | undefined, enabled = true): FilterFieldExtension[] {
  const context = useMemo(() => ({ recordType: recordType ?? '' }), [recordType])
  const extensions = useFilters('records.filterFields', NO_EXTENSIONS, context)
  return enabled && recordType ? extensions : NO_EXTENSIONS
}

export function extendFilterSchema(schema: FieldSchema, extensions: FilterFieldExtension[]): FieldSchema {
  if (extensions.length === 0) return schema
  const keys = new Set(schema.fields.map((field) => field.key))
  const extra = extensions.filter((extension) => !keys.has(extension.field.key))
  if (extra.length === 0) return schema
  const known = new Set(schema.operators.map((definition) => definition.operator))
  const operators = [...schema.operators]
  for (const extension of extra) {
    for (const operator of extension.field.operators) {
      if (known.has(operator)) continue
      known.add(operator)
      operators.push({ operator, operand: extension.operands?.[operator] ?? 'single' })
    }
  }
  return { ...schema, fields: [...schema.fields, ...extra.map((extension) => extension.field)], operators }
}

export function useFilterSchema<T extends FieldSchema | undefined>(
  schema: T,
  enabled = true,
): { schema: T; extensions: FilterFieldExtension[] } {
  const extensions = useFilterExtensions(schema?.record_type, enabled)
  const extended = useMemo(
    () => (schema ? extendFilterSchema(schema, extensions) : schema) as T,
    [schema, extensions],
  )
  return { schema: extended, extensions }
}

export function findExtension(extensions: FilterFieldExtension[], key: string): FilterFieldExtension | undefined {
  return extensions.find((extension) => extension.field.key === key)
}
