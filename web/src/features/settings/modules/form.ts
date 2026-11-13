import { z } from 'zod'
import { isApiError } from '@/api'
import type { ModuleInfo, ModuleSettingField } from '@/kernel/modules/api'

export type SettingValue = string | boolean
export type SettingsFormValues = Record<string, SettingValue>

const URL_PATTERN = /^https?:\/\/\S+$/i
const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
const NUMBER_PATTERN = /^-?\d+(\.\d+)?$/

export type ValidationKey =
  | 'validation.required'
  | 'validation.email'
  | 'validation.url'
  | 'validation.number'
  | 'validation.range'
  | 'validation.invalid'
  | 'validation.tooLong'
  | 'validation.text'
  | 'validation.boolean'
  | 'validation.option'
  | 'validation.field'
  | 'validation.unknown'

export function numberMaxLength(min: number | null | undefined, max: number | null | undefined): number {
  const digits = Math.max(String(Math.abs(max ?? 999_999_999)).length, String(Math.abs(min ?? 0)).length)
  return digits + (min != null && min < 0 ? 1 : 0)
}

export function isSecretConfigured(module: ModuleInfo, key: string): boolean {
  return module.configured_secrets.includes(key)
}

function currentSettings(module: ModuleInfo): Record<string, unknown> {
  return module.settings as Record<string, unknown>
}

export function initialValues(module: ModuleInfo): SettingsFormValues {
  const settings = currentSettings(module)
  const values: SettingsFormValues = {}
  for (const field of module.settings_schema) {
    const current = field.type === 'secret' ? undefined : (settings[field.key] ?? (field.default as unknown))
    if (field.type === 'boolean') {
      values[field.key] = current === true
    } else {
      values[field.key] = current === undefined || current === null ? '' : String(current)
    }
  }
  return values
}

function fieldIssue(field: ModuleSettingField, value: SettingValue, secretConfigured: boolean): ValidationKey | null {
  if (field.type === 'boolean') return null
  const text = typeof value === 'string' ? value.trim() : ''
  if (!text) {
    if (!field.required) return null
    if (field.type === 'secret' && secretConfigured) return null
    return 'validation.required'
  }
  switch (field.type) {
    case 'email':
      return EMAIL_PATTERN.test(text) ? null : 'validation.email'
    case 'url':
      return URL_PATTERN.test(text) ? null : 'validation.url'
    case 'number': {
      if (!NUMBER_PATTERN.test(text)) return 'validation.number'
      const number = Number(text)
      if ((field.min != null && number < field.min) || (field.max != null && number > field.max)) {
        return 'validation.range'
      }
      return null
    }
    case 'select':
      return field.options.some((option) => option.value === text) ? null : 'validation.invalid'
    default:
      return null
  }
}

export function buildFormSchema(module: ModuleInfo, clearedSecrets: string[]) {
  return z.record(z.string(), z.union([z.string(), z.boolean()])).superRefine((values, context) => {
    for (const field of module.settings_schema) {
      const secretConfigured = isSecretConfigured(module, field.key) && !clearedSecrets.includes(field.key)
      const issue = fieldIssue(field, values[field.key] ?? '', secretConfigured)
      if (issue) context.addIssue({ code: 'custom', path: [field.key], message: issue })
    }
  })
}

export function toSettings(
  module: ModuleInfo,
  values: SettingsFormValues,
  clearedSecrets: string[],
): Record<string, unknown> {
  const settings: Record<string, unknown> = {}
  for (const field of module.settings_schema) {
    const value = values[field.key]
    if (field.type === 'boolean') {
      settings[field.key] = value === true
      continue
    }
    if (field.type === 'secret') {
      if (clearedSecrets.includes(field.key)) settings[field.key] = null
      else if (typeof value === 'string' && value.trim()) settings[field.key] = value
      continue
    }
    const text = typeof value === 'string' ? value.trim() : ''
    if (!text) settings[field.key] = null
    else settings[field.key] = field.type === 'number' ? Number(text) : text
  }
  return settings
}

export interface SettingViolation {
  field: string
  message: string
  code?: string
}

const VIOLATION_KEYS: Record<string, ValidationKey> = {
  setting_required: 'validation.required',
  setting_unknown: 'validation.unknown',
  setting_too_long: 'validation.tooLong',
  setting_not_text: 'validation.text',
  setting_invalid_email: 'validation.email',
  setting_invalid_url: 'validation.url',
  setting_not_number: 'validation.number',
  setting_out_of_range: 'validation.range',
  setting_not_boolean: 'validation.boolean',
  setting_invalid_option: 'validation.option',
  setting_invalid_field: 'validation.field',
}

export function serverViolations(error: unknown): SettingViolation[] {
  if (!isApiError(error) || typeof error.details !== 'object' || error.details === null) return []
  const fields = (error.details as { fields?: unknown }).fields
  if (!Array.isArray(fields)) return []
  return fields.filter(
    (item): item is SettingViolation =>
      typeof item === 'object' &&
      item !== null &&
      typeof (item as SettingViolation).field === 'string' &&
      typeof (item as SettingViolation).message === 'string',
  )
}

export function violationKey(violation: SettingViolation): ValidationKey {
  return (violation.code && VIOLATION_KEYS[violation.code]) || 'validation.invalid'
}

export function violationField(violation: SettingViolation): string {
  return violation.field.startsWith('settings.') ? violation.field.slice('settings.'.length) : violation.field
}
