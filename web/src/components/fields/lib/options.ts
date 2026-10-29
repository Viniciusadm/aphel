import type { FieldDefinition, FieldOption } from '@/features/records/types'

export function sortedOptions(field: FieldDefinition): FieldOption[] {
  return [...field.options].sort((left, right) => left.position - right.position)
}

export function selectableOptions(field: FieldDefinition, current: unknown): FieldOption[] {
  const currentValues = Array.isArray(current) ? current.map(String) : current == null ? [] : [String(current)]
  return sortedOptions(field).filter((option) => option.is_active || currentValues.includes(option.value))
}

export function normalizeList(value: unknown): string[] {
  if (Array.isArray(value)) return value.map(String)
  if (value === null || value === undefined || value === '') return []
  return [String(value)]
}

export function normalizeText(value: unknown): string | null {
  if (value === null || value === undefined) return null
  return typeof value === 'string' ? value : String(value)
}
