import { asValueField, fieldConfig, isFormulaField } from '../fields'
import { formatCurrency, formatDate, formatDateTime, formatNumber, type RecordFormatting } from '../formatting'
import type { FieldDefinition, FieldOption } from '../types'

export function isEmptyValue(value: unknown): boolean {
  if (value === null || value === undefined) return true
  if (typeof value === 'string') return value.trim() === ''
  if (Array.isArray(value)) return value.length === 0
  return false
}

export function optionFor(field: FieldDefinition, value: unknown): FieldOption | undefined {
  return field.options.find((option) => option.value === value)
}

export function formatFieldText(field: FieldDefinition, value: unknown, formatting: RecordFormatting): string {
  if (isEmptyValue(value)) return ''
  if (isFormulaField(field)) return formatFieldText(asValueField(field), value, formatting)
  const config = fieldConfig(field)
  switch (field.field_type) {
    case 'number':
      return formatNumber(value, formatting, config.decimals)
    case 'currency':
      return formatCurrency(value, formatting, config.currency)
    case 'date':
      return formatDate(value, formatting)
    case 'datetime':
      return formatDateTime(value, formatting)
    case 'select':
      return optionFor(field, value)?.label ?? String(value)
    case 'multi_select':
      return Array.isArray(value)
        ? value.map((item) => optionFor(field, item)?.label ?? String(item)).join(', ')
        : String(value)
    default:
      return String(value)
  }
}
