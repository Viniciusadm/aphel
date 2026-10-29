import { z } from 'zod'
import { isApiError } from '@/api'
import { configOf, isSecretConfigured, type Integration, type IntegrationConfig, type IntegrationField } from './api'

export type FieldValue = string | boolean
export type ConfigFormValues = Record<string, FieldValue>

const URL_PATTERN = /^https?:\/\/[^\s/?#]+/i
const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+$/

export type ValidationKey =
  | 'validation.required'
  | 'validation.email'
  | 'validation.url'
  | 'validation.number'
  | 'validation.range'
  | 'validation.invalid'

function defaultOf(field: IntegrationField): unknown {
  return field.default ?? undefined
}

export function initialValues(integration: Integration): ConfigFormValues {
  const config = configOf(integration)
  const values: ConfigFormValues = {}
  for (const field of integration.fields) {
    const current = field.type === 'secret' ? undefined : (config[field.key] ?? defaultOf(field))
    if (field.type === 'boolean') {
      values[field.key] = current === true
    } else {
      values[field.key] = current === undefined || current === null ? '' : String(current)
    }
  }
  return values
}

function fieldIssue(field: IntegrationField, value: FieldValue, secretConfigured: boolean): ValidationKey | null {
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
      if (!/^-?\d+$/.test(text)) return 'validation.number'
      const number = Number(text)
      if ((field.min != null && number < field.min) || (field.max != null && number > field.max)) {
        return 'validation.range'
      }
      return null
    }
    case 'select':
      return field.options.includes(text) ? null : 'validation.invalid'
    default:
      return null
  }
}

export function buildFormSchema(integration: Integration) {
  return z.record(z.string(), z.union([z.string(), z.boolean()])).superRefine((values, context) => {
    for (const field of integration.fields) {
      const issue = fieldIssue(field, values[field.key] ?? '', isSecretConfigured(integration, field.key))
      if (issue) context.addIssue({ code: 'custom', path: [field.key], message: issue })
    }
  })
}

export function toConfig(integration: Integration, values: ConfigFormValues): IntegrationConfig {
  const config: IntegrationConfig = {}
  for (const field of integration.fields) {
    const value = values[field.key]
    if (field.type === 'boolean') {
      config[field.key] = value === true
      continue
    }
    const text = typeof value === 'string' ? value.trim() : ''
    if (!text) continue
    config[field.key] = field.type === 'number' ? Number(text) : field.type === 'secret' ? value : text
  }
  return config
}

interface Violation {
  field: string
  message: string
}

export function serverViolations(error: unknown): Violation[] {
  if (!isApiError(error) || typeof error.details !== 'object' || error.details === null) return []
  const fields = (error.details as { fields?: unknown }).fields
  if (!Array.isArray(fields)) return []
  return fields.filter(
    (item): item is Violation =>
      typeof item === 'object' &&
      item !== null &&
      typeof (item as Violation).field === 'string' &&
      typeof (item as Violation).message === 'string',
  )
}

export function violationKey(violation: Violation): ValidationKey {
  return violation.message === 'is required' ? 'validation.required' : 'validation.invalid'
}

export function violationField(violation: Violation): string | null {
  return violation.field.startsWith('config.') ? violation.field.slice('config.'.length) : null
}
