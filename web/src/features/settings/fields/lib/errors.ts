import type { ParseKeys } from 'i18next'
import type { FieldValues, Path, UseFormSetError } from 'react-hook-form'
import { NETWORK_ERROR_CODE, isApiError } from '@/api'
import { fieldViolations } from '../../lib/errors'
import type { FieldType } from './field-types'

export type FieldsMessageKey = ParseKeys<'fields'>

export const SERVER_ERROR_PATH = 'root.server'

export const RECORD_TYPE_NOT_FOUND = 'record_type_not_found'

const codeMessages: Record<string, FieldsMessageKey> = {
  [NETWORK_ERROR_CODE]: 'errors.network',
  permission_denied: 'errors.permissionDenied',
  not_found: 'errors.fieldNotFound',
  field_not_found: 'errors.fieldNotFound',
  field_option_not_found: 'errors.optionNotFound',
  [RECORD_TYPE_NOT_FOUND]: 'errors.recordTypeNotFound',
  field_key_taken: 'errors.fieldKeyTaken',
  option_value_taken: 'errors.optionValueTaken',
  system_field_locked: 'errors.systemFieldLocked',
  incompatible_field_type: 'errors.incompatibleFieldType',
  duplicate_values: 'errors.duplicateValues',
  too_many_fields: 'errors.tooManyFields',
  field_without_options: 'errors.fieldWithoutOptions',
  validation_error: 'errors.invalid',
  version_conflict: 'errors.versionConflict',
  formula_cycle: 'errors.formulaCycle',
}

const violationMessages: Record<string, FieldsMessageKey> = {
  key: 'validation.key',
  label: 'validation.label',
  field_type: 'validation.fieldType',
  is_required: 'validation.invalid',
  is_unique: 'validation.unique',
  is_searchable: 'validation.searchable',
  is_personal: 'validation.invalid',
  is_sensitive: 'validation.invalid',
  'config.min': 'validation.minMax',
  'config.max': 'validation.minMax',
  'config.decimals': 'validation.decimals',
  'config.currency': 'validation.currency',
  'config.max_length': 'validation.maxLength',
  'config.pattern': 'validation.pattern',
  'config.default_country': 'validation.country',
  'config.target_record_type': 'validation.targetRecordType',
  'config.default_value': 'validation.defaultValue',
  'config.expression': 'validation.formulaExpression',
  'config.result_type': 'validation.formulaResultType',
  'options.*.label': 'validation.optionLabel',
  'options.*.value': 'validation.optionValue',
  'options.*.color': 'validation.color',
  value: 'validation.optionValue',
  color: 'validation.color',
}

const formPaths: Record<string, string> = {
  key: 'key',
  label: 'label',
  field_type: 'fieldType',
  is_required: 'isRequired',
  is_unique: 'isUnique',
  is_searchable: 'isSearchable',
  is_personal: 'isPersonal',
  is_sensitive: 'isSensitive',
  'config.min': 'config.min',
  'config.max': 'config.max',
  'config.decimals': 'config.decimals',
  'config.currency': 'config.currency',
  'config.max_length': 'config.maxLength',
  'config.pattern': 'config.pattern',
  'config.default_country': 'config.defaultCountry',
  'config.target_record_type': 'config.targetRecordType',
  'config.expression': 'config.expression',
  'config.result_type': 'config.resultType',
}

const OPTION_PATH = /^options\.(\d+)\.(label|value|color)$/

function normalizedPath(path: string): string {
  return path.replace(/^options\.\d+\./, 'options.*.')
}

export function violationMessageKey(path: string): FieldsMessageKey {
  return violationMessages[normalizedPath(path)] ?? 'validation.invalid'
}

export function fieldFormPath(path: string, fieldType: FieldType): string | undefined {
  if (path === 'config.default_value') {
    return fieldType === 'multi_select' ? 'config.defaultOptions' : 'config.defaultValue'
  }
  const option = OPTION_PATH.exec(path)
  if (option) return `options.${option[1]}.${option[2] === 'color' ? 'color' : 'label'}`
  return formPaths[path]
}

export function errorMessageKey(error: unknown): FieldsMessageKey {
  if (!isApiError(error)) return 'errors.generic'
  return codeMessages[error.code] ?? 'errors.generic'
}

export function isRecordTypeNotFound(error: unknown): boolean {
  return isApiError(error) && error.code === RECORD_TYPE_NOT_FOUND
}

export interface FieldsErrorMapping<T extends FieldValues> {
  path: (serverPath: string) => string | undefined
  codes?: Partial<Record<string, Path<T>>>
}

export function applyFieldsError<T extends FieldValues>(
  error: unknown,
  setError: UseFormSetError<T>,
  mapping: FieldsErrorMapping<T>,
): void {
  if (isApiError(error)) {
    let mapped = false
    for (const violation of fieldViolations(error.details)) {
      const name = mapping.path(violation.field)
      if (!name) continue
      setError(
        name as Path<T>,
        { type: 'server', message: violationMessageKey(violation.field) },
        { shouldFocus: !mapped },
      )
      mapped = true
    }
    if (mapped) return

    const codeField = mapping.codes?.[error.code]
    if (codeField) {
      setError(codeField, { type: 'server', message: errorMessageKey(error) }, { shouldFocus: true })
      return
    }
  }
  setError(SERVER_ERROR_PATH as Path<T>, { type: 'server', message: errorMessageKey(error) })
}

export function asFieldsMessageKey(message: string | undefined): FieldsMessageKey | undefined {
  return message ? (message as FieldsMessageKey) : undefined
}
