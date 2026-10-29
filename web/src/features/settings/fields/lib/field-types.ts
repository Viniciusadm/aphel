import type { components } from '@/api'

export type FieldType = components['schemas']['FieldType']

export const FIELD_TYPES = [
  'text',
  'long_text',
  'number',
  'currency',
  'date',
  'datetime',
  'select',
  'multi_select',
  'checkbox',
  'url',
  'email',
  'phone',
  'user',
  'record',
  'formula',
] as const satisfies readonly FieldType[]

export const RECORD_TYPES = ['contact', 'company', 'deal', 'activity', 'product'] as const

export type KnownRecordType = (typeof RECORD_TYPES)[number]

export const DEFAULT_RECORD_TYPE: KnownRecordType = 'contact'

export const LABEL_MAX_LENGTH = 140
export const KEY_MAX_LENGTH = 64
export const TEXT_MAX_LENGTH = 1024
export const LONG_TEXT_MAX_LENGTH = 65_535
export const MAX_DECIMALS = 6
export const PATTERN_MAX_LENGTH = 500

const COMPATIBLE_GROUPS: FieldType[][] = [
  ['text', 'email', 'url', 'phone'],
  ['number', 'currency'],
]

const UNIQUE_TYPES: FieldType[] = ['text', 'number', 'url', 'email', 'phone']
const SEARCHABLE_TYPES: FieldType[] = ['text', 'long_text', 'url', 'email', 'phone']

export function isKnownRecordType(value: string): value is KnownRecordType {
  return (RECORD_TYPES as readonly string[]).includes(value)
}

export function compatibleTypes(fieldType: FieldType): FieldType[] {
  return COMPATIBLE_GROUPS.find((group) => group.includes(fieldType)) ?? [fieldType]
}

export function supportsUnique(fieldType: FieldType): boolean {
  return UNIQUE_TYPES.includes(fieldType)
}

export function supportsSearchable(fieldType: FieldType): boolean {
  return SEARCHABLE_TYPES.includes(fieldType)
}

export function hasOptions(fieldType: FieldType): boolean {
  return fieldType === 'select' || fieldType === 'multi_select'
}

export function isNumeric(fieldType: FieldType): boolean {
  return fieldType === 'number' || fieldType === 'currency'
}

export function supportsMaxLength(fieldType: FieldType): boolean {
  return fieldType === 'text' || fieldType === 'long_text'
}

export function supportsDefaultValue(fieldType: FieldType): boolean {
  return fieldType !== 'user' && fieldType !== 'record' && fieldType !== 'formula'
}

export function isFormulaType(fieldType: FieldType): boolean {
  return fieldType === 'formula'
}

export function supportsRequired(fieldType: FieldType): boolean {
  return !isFormulaType(fieldType)
}

export function textLengthLimit(fieldType: FieldType): number {
  return fieldType === 'long_text' ? LONG_TEXT_MAX_LENGTH : TEXT_MAX_LENGTH
}
