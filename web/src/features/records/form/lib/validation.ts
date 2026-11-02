import type { TFunction } from 'i18next'
import { z } from 'zod'
import { countDecimals, toNumber } from '@/components/fields/lib/number'
import { DEFAULT_CURRENCY_DECIMALS } from '@/components/fields/lib/currency'
import { fieldConfig } from '../../fields'
import type { FieldDefinition } from '../../types'

export const DEFAULT_TEXT_MAX_LENGTH = 1024

export type ValidationCode =
  | 'required'
  | 'invalid'
  | 'invalidNumber'
  | 'min'
  | 'max'
  | 'decimals'
  | 'tooLong'
  | 'pattern'
  | 'email'
  | 'url'
  | 'phone'
  | 'date'
  | 'datetime'
  | 'option'
  | 'inactiveOption'
  | 'tooManyOptions'
  | 'reference'
  | 'referenceNotFound'
  | 'unique'
  | 'boolean'
  | 'outOfRange'
  | 'unknownField'
  | 'inactiveField'
  | 'notAccessible'
  | 'readOnlyField'

export interface ValidationIssue {
  code: ValidationCode
  values?: Record<string, string | number>
}

export interface ValidationContext {
  initialValue?: unknown
}

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
const E164 = /^\+[1-9]\d{6,14}$/
const DATE = /^(\d{4})-(\d{2})-(\d{2})$/

let phoneValidator: ((value: unknown) => boolean) | null = null
let phoneValidatorLoad: Promise<void> | null = null

export function preloadPhoneValidation(): Promise<void> {
  phoneValidatorLoad ??= import('@/components/fields/lib/phone').then((module) => {
    phoneValidator = module.isPhoneValid
  })
  return phoneValidatorLoad
}

function isValidPhone(value: unknown): boolean {
  if (phoneValidator) return phoneValidator(value)
  void preloadPhoneValidation()
  return typeof value === 'string' && E164.test(value)
}

export function isBlank(value: unknown): boolean {
  if (value === null || value === undefined) return true
  if (typeof value === 'string') return value.trim() === ''
  if (Array.isArray(value)) return value.length === 0
  return false
}

function optionalNumber(value: unknown): number | null {
  return toNumber(value)
}

function isValidDate(value: string): boolean {
  const match = DATE.exec(value)
  if (!match) return false
  const [, year, month, day] = match.map(Number)
  const date = new Date(Date.UTC(year, month - 1, day))
  return date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day
}

function isValidUrl(value: string): boolean {
  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(value) ? value : `https://${value}`
  try {
    const url = new URL(candidate)
    return (url.protocol === 'http:' || url.protocol === 'https:') && url.hostname.includes('.')
  } catch {
    return false
  }
}

function matchesPattern(pattern: string, value: string): boolean {
  try {
    return new RegExp(pattern, 'u').test(value)
  } catch {
    return true
  }
}

function validateText(field: FieldDefinition, value: unknown): ValidationIssue | null {
  if (typeof value !== 'string') return { code: 'invalid' }
  const config = fieldConfig(field)
  const trimmed = value.trim()
  const limit = config.max_length ?? (field.field_type === 'text' ? DEFAULT_TEXT_MAX_LENGTH : null)
  if (limit !== null && [...trimmed].length > limit) return { code: 'tooLong', values: { max: limit } }
  if (field.field_type === 'text' && config.pattern && !matchesPattern(config.pattern, trimmed)) {
    return { code: 'pattern' }
  }
  return null
}

function validateNumber(field: FieldDefinition, value: unknown): ValidationIssue | null {
  const number = toNumber(value)
  if (number === null) return { code: 'invalidNumber' }
  const config = fieldConfig(field)
  const min = optionalNumber(config.min)
  const max = optionalNumber(config.max)
  if (min !== null && number < min) return { code: 'min', values: { min } }
  if (max !== null && number > max) return { code: 'max', values: { max } }
  const decimals = config.decimals ?? (field.field_type === 'currency' ? DEFAULT_CURRENCY_DECIMALS : null)
  if (decimals !== null && countDecimals(number) > decimals) {
    return { code: 'decimals', values: { count: decimals } }
  }
  return null
}

function validateOption(field: FieldDefinition, value: string, allowed: string[]): ValidationIssue | null {
  const option = field.options.find((item) => item.value === value)
  if (!option) return { code: 'option' }
  if (!option.is_active && !allowed.includes(value)) return { code: 'inactiveOption' }
  return null
}

function asList(value: unknown): string[] {
  if (Array.isArray(value)) return value.map(String)
  return isBlank(value) ? [] : [String(value)]
}

export function validateFieldValue(
  field: FieldDefinition,
  value: unknown,
  context: ValidationContext = {},
): ValidationIssue | null {
  if (field.field_type === 'checkbox') {
    if (value === null || value === undefined) return field.is_required ? { code: 'required' } : null
    return typeof value === 'boolean' ? null : { code: 'boolean' }
  }
  if (isBlank(value)) return field.is_required ? { code: 'required' } : null

  switch (field.field_type) {
    case 'text':
    case 'long_text':
      return validateText(field, value)
    case 'number':
    case 'currency':
      return validateNumber(field, value)
    case 'date':
      return typeof value === 'string' && isValidDate(value) ? null : { code: 'date' }
    case 'datetime':
      return typeof value === 'string' && !Number.isNaN(new Date(value).getTime()) ? null : { code: 'datetime' }
    case 'email':
      return typeof value === 'string' && EMAIL.test(value.trim()) ? validateText(field, value) : { code: 'email' }
    case 'url':
      return typeof value === 'string' && isValidUrl(value.trim()) ? validateText(field, value) : { code: 'url' }
    case 'phone':
      return isValidPhone(value) ? null : { code: 'phone' }
    case 'select':
      return typeof value === 'string' ? validateOption(field, value, asList(context.initialValue)) : { code: 'option' }
    case 'multi_select': {
      if (!Array.isArray(value)) return { code: 'option' }
      const allowed = asList(context.initialValue)
      for (const item of value) {
        const issue = typeof item === 'string' ? validateOption(field, item, allowed) : { code: 'option' as const }
        if (issue) return issue
      }
      return null
    }
    case 'user':
    case 'record':
      return typeof value === 'string' ? null : { code: 'reference' }
    case 'formula':
    default:
      return null
  }
}

export type RecordFormT = TFunction<'records-form'>

export function issueMessage(issue: ValidationIssue, t: RecordFormT): string {
  return t(`validation.${issue.code}`, issue.values ?? {})
}

export function buildRecordSchema(
  fields: FieldDefinition[],
  t: RecordFormT,
  initialValues: Record<string, unknown> = {},
) {
  const shape: Record<string, z.ZodType<unknown>> = {}
  if (fields.some((field) => field.field_type === 'phone')) void preloadPhoneValidation()
  for (const field of fields) {
    shape[field.key] = z.unknown().superRefine((value, ctx) => {
      const issue = validateFieldValue(field, value, { initialValue: initialValues[field.key] })
      if (issue) ctx.addIssue({ code: 'custom', message: issueMessage(issue, t) })
    })
  }
  return z.object(shape).passthrough()
}
