import { zodResolver } from '@hookform/resolvers/zod'
import type { Resolver } from 'react-hook-form'
import { z } from 'zod'
import type { CreateFieldRequest, Field, UpdateFieldRequest } from '../api'
import {
  LABEL_MAX_LENGTH,
  MAX_DECIMALS,
  PATTERN_MAX_LENGTH,
  hasOptions,
  isFormulaType,
  isNumeric,
  supportsDefaultValue,
  supportsRequired,
  supportsMaxLength,
  supportsSearchable,
  supportsUnique,
  textLengthLimit,
  type FieldType,
} from './field-types'
import { isValidKey, optionValuesFromLabels } from './identifiers'
import {
  DEFAULT_FORMULA_CURRENCY,
  DEFAULT_FORMULA_RESULT_TYPE,
  FORMULA_EXPRESSION_MAX_LENGTH,
  FORMULA_RESULT_TYPES,
  formulaUsesCurrency,
  formulaUsesDecimals,
  isFormulaResultType,
  type FormulaResultType,
} from '../formula/result-types'

export interface DraftOption {
  label: string
  color: string
}

export interface ConfigValues {
  min: string
  max: string
  decimals: string
  currency: string
  maxLength: string
  pattern: string
  defaultCountry: string
  targetRecordType: string
  defaultValue: string
  defaultOptions: string[]
  expression: string
  resultType: FormulaResultType
}

export interface FieldFormValues {
  label: string
  key: string
  fieldType: FieldType
  isRequired: boolean
  isUnique: boolean
  isSearchable: boolean
  isPersonal: boolean
  isSensitive: boolean
  config: ConfigValues
  options: DraftOption[]
}

export interface FieldFormContext {
  mode: 'create' | 'edit'
  system: boolean
  existingOptionValues: string[]
}

type ConfigPayload = Record<string, unknown>
type TextConfigKey = Exclude<keyof ConfigValues, 'defaultOptions' | 'resultType'>

const NUMBER_PATTERN = /^-?\d+(\.\d+)?$/
const INTEGER_PATTERN = /^\d+$/
const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/
const URL_PATTERN = /^\S+\.\S+$/
const CURRENCY_PATTERN = /^[A-Za-z]{3}$/
const COUNTRY_PATTERN = /^[A-Za-z]{2}$/
export const HEX_COLOR_PATTERN = /^#[0-9a-fA-F]{6}$/

export function emptyConfig(): ConfigValues {
  return {
    min: '',
    max: '',
    decimals: '',
    currency: '',
    maxLength: '',
    pattern: '',
    defaultCountry: '',
    targetRecordType: '',
    defaultValue: '',
    defaultOptions: [],
    expression: '',
    resultType: DEFAULT_FORMULA_RESULT_TYPE,
  }
}

export function emptyFormValues(): FieldFormValues {
  return {
    label: '',
    key: '',
    fieldType: 'text',
    isRequired: false,
    isUnique: false,
    isSearchable: false,
    isPersonal: false,
    isSensitive: false,
    config: emptyConfig(),
    options: [],
  }
}

function pad(value: number): string {
  return String(value).padStart(2, '0')
}

export function isoToLocalInput(iso: string): string {
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return ''
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`
}

function asText(value: unknown): string {
  if (value === undefined || value === null) return ''
  return String(value)
}

function defaultValueText(fieldType: FieldType, value: unknown): string {
  if (value === undefined || value === null || Array.isArray(value)) return ''
  if (fieldType === 'checkbox') return value === true ? 'true' : value === false ? 'false' : ''
  if (fieldType === 'datetime') return isoToLocalInput(String(value))
  return String(value)
}

export function formValuesFromField(field: Field): FieldFormValues {
  const config = field.config as ConfigPayload
  const defaultValue = config.default_value
  return {
    label: field.label,
    key: field.key,
    fieldType: field.field_type,
    isRequired: field.is_required,
    isUnique: field.is_unique,
    isSearchable: field.is_searchable,
    isPersonal: field.is_personal,
    isSensitive: field.is_sensitive,
    config: {
      min: asText(config.min),
      max: asText(config.max),
      decimals: asText(config.decimals),
      currency: asText(config.currency),
      maxLength: asText(config.max_length),
      pattern: asText(config.pattern),
      defaultCountry: asText(config.default_country),
      targetRecordType: asText(config.target_record_type),
      defaultValue: defaultValueText(field.field_type, defaultValue),
      defaultOptions: Array.isArray(defaultValue) ? defaultValue.map(String) : [],
      expression: asText(config.expression),
      resultType: isFormulaResultType(config.result_type) ? config.result_type : DEFAULT_FORMULA_RESULT_TYPE,
    },
    options: [],
  }
}

const text = z.string().trim()

function optionalMatching(pattern: RegExp, message: string) {
  return text.refine((value) => value === '' || pattern.test(value), message)
}

function optionalInteger(min: number, max: number, message: string) {
  return text.refine(
    (value) => value === '' || (INTEGER_PATTERN.test(value) && Number(value) >= min && Number(value) <= max),
    message,
  )
}

function compiles(pattern: string): boolean {
  try {
    new RegExp(pattern)
    return true
  } catch {
    return false
  }
}

function isEmail(value: string): boolean {
  return z.email().safeParse(value).success
}

function typeRules(fieldType: FieldType): Partial<Record<TextConfigKey, typeof text>> {
  switch (fieldType) {
    case 'number':
    case 'currency':
      return {
        min: optionalMatching(NUMBER_PATTERN, 'validation.number'),
        max: optionalMatching(NUMBER_PATTERN, 'validation.number'),
        decimals: optionalInteger(0, MAX_DECIMALS, 'validation.decimals'),
        currency:
          fieldType === 'currency' ? optionalMatching(CURRENCY_PATTERN, 'validation.currency') : text,
        defaultValue: optionalMatching(NUMBER_PATTERN, 'validation.number'),
      }
    case 'text':
      return {
        maxLength: optionalInteger(1, textLengthLimit(fieldType), 'validation.maxLengthText'),
        pattern: text
          .max(PATTERN_MAX_LENGTH, 'validation.pattern')
          .refine((value) => value === '' || compiles(value), 'validation.pattern'),
      }
    case 'long_text':
      return { maxLength: optionalInteger(1, textLengthLimit(fieldType), 'validation.maxLengthLongText') }
    case 'phone':
      return { defaultCountry: optionalMatching(COUNTRY_PATTERN, 'validation.country') }
    case 'record':
      return { targetRecordType: text.min(1, 'validation.targetRecordType') }
    case 'date':
      return { defaultValue: optionalMatching(DATE_PATTERN, 'validation.date') }
    case 'datetime':
      return {
        defaultValue: text.refine((value) => value === '' || !Number.isNaN(Date.parse(value)), 'validation.datetime'),
      }
    case 'email':
      return { defaultValue: text.refine((value) => value === '' || isEmail(value), 'validation.email') }
    case 'url':
      return { defaultValue: optionalMatching(URL_PATTERN, 'validation.url') }
    case 'formula':
      return {
        expression: text.min(1, 'validation.formulaExpression').max(FORMULA_EXPRESSION_MAX_LENGTH, 'validation.formulaExpression'),
        decimals: optionalInteger(0, MAX_DECIMALS, 'validation.decimals'),
        currency: optionalMatching(CURRENCY_PATTERN, 'validation.currency'),
      }
    default:
      return {}
  }
}

function configSchema(fieldType: FieldType, system: boolean) {
  const base = {
    min: text,
    max: text,
    decimals: text,
    currency: text,
    maxLength: text,
    pattern: text,
    defaultCountry: text,
    targetRecordType: text,
    defaultValue: text,
    defaultOptions: z.array(z.string()),
    expression: z.string(),
    resultType: z.enum(FORMULA_RESULT_TYPES),
  }
  return z.object(system ? base : { ...base, ...typeRules(fieldType) })
}

function decimalsOf(value: string): number {
  const [, fraction = ''] = value.split('.')
  return fraction.replace(/0+$/, '').length
}

function optionValuesFor(values: FieldFormValues, context: FieldFormContext): string[] {
  if (context.mode === 'edit') return context.existingOptionValues
  return optionValuesFromLabels(values.options.map((option) => option.label))
}

function refineValues(values: FieldFormValues, context: FieldFormContext, ctx: z.RefinementCtx) {
  if (context.system) return
  const { fieldType, config } = values
  const issue = (path: (string | number)[], message: string) => ctx.addIssue({ code: 'custom', path, message })
  const defaultValue = config.defaultValue.trim()

  if (isNumeric(fieldType)) {
    const min = NUMBER_PATTERN.test(config.min) ? Number(config.min) : undefined
    const max = NUMBER_PATTERN.test(config.max) ? Number(config.max) : undefined
    if (min !== undefined && max !== undefined && min > max) issue(['config', 'max'], 'validation.minMax')
    if (NUMBER_PATTERN.test(defaultValue)) {
      const number = Number(defaultValue)
      if ((min !== undefined && number < min) || (max !== undefined && number > max)) {
        issue(['config', 'defaultValue'], 'validation.defaultOutOfRange')
      }
      const allowed = INTEGER_PATTERN.test(config.decimals)
        ? Number(config.decimals)
        : fieldType === 'currency'
          ? 2
          : MAX_DECIMALS
      if (decimalsOf(defaultValue) > allowed) issue(['config', 'defaultValue'], 'validation.defaultDecimals')
    }
  }

  if (supportsMaxLength(fieldType) && defaultValue !== '') {
    const limit = INTEGER_PATTERN.test(config.maxLength) ? Number(config.maxLength) : textLengthLimit(fieldType)
    if ([...defaultValue].length > limit) issue(['config', 'defaultValue'], 'validation.defaultTooLong')
    const pattern = config.pattern.trim()
    if (fieldType === 'text' && pattern !== '' && compiles(pattern) && !new RegExp(pattern).test(defaultValue)) {
      issue(['config', 'defaultValue'], 'validation.defaultPattern')
    }
  }

  if (hasOptions(fieldType)) {
    const optionValues = optionValuesFor(values, context)
    if (fieldType === 'select' && defaultValue !== '' && !optionValues.includes(defaultValue)) {
      issue(['config', 'defaultValue'], 'validation.defaultOption')
    }
    if (fieldType === 'multi_select' && config.defaultOptions.some((value) => !optionValues.includes(value))) {
      issue(['config', 'defaultOptions'], 'validation.defaultOption')
    }
    if (context.mode === 'create') {
      values.options.forEach((option, index) => {
        const label = option.label.trim()
        if (label === '' || label.length > LABEL_MAX_LENGTH) issue(['options', index, 'label'], 'validation.optionLabel')
        if (option.color.trim() !== '' && !HEX_COLOR_PATTERN.test(option.color.trim())) {
          issue(['options', index, 'color'], 'validation.color')
        }
      })
    }
  }
}

export function fieldFormSchema(fieldType: FieldType, context: FieldFormContext) {
  return z
    .object({
      label: text.min(1, 'validation.required').max(LABEL_MAX_LENGTH, 'validation.label'),
      key:
        context.mode === 'create'
          ? text.min(1, 'validation.required').refine(isValidKey, 'validation.key')
          : z.string(),
      fieldType: z.string().trim().min(1, 'validation.required'),
      isRequired: z.boolean(),
      isUnique: z.boolean(),
      isSearchable: z.boolean(),
      isPersonal: z.boolean(),
      isSensitive: z.boolean(),
      config: configSchema(fieldType, context.system),
      options: z.array(z.object({ label: z.string(), color: z.string() })),
    })
    .superRefine((values, ctx) => refineValues(values, context, ctx))
}

const DEFAULT_CONTEXT: FieldFormContext = { mode: 'create', system: false, existingOptionValues: [] }

export const fieldFormResolver: Resolver<FieldFormValues, FieldFormContext, FieldFormValues> = (
  values,
  context,
  options,
) => zodResolver(fieldFormSchema(values.fieldType, context ?? DEFAULT_CONTEXT))(values, context, options)

function numberOrUndefined(value: string): number | undefined {
  const trimmed = value.trim()
  return trimmed === '' ? undefined : Number(trimmed)
}

function defaultValuePayload(values: FieldFormValues): unknown {
  const raw = values.config.defaultValue.trim()
  switch (values.fieldType) {
    case 'multi_select':
      return values.config.defaultOptions.length > 0 ? values.config.defaultOptions : undefined
    case 'checkbox':
      return raw === 'true' ? true : raw === 'false' ? false : undefined
    case 'number':
    case 'currency':
      return numberOrUndefined(raw)
    case 'datetime':
      return raw === '' ? undefined : new Date(raw).toISOString()
    default:
      return raw === '' ? undefined : raw
  }
}

function buildFormulaConfig(config: ConfigValues): ConfigPayload {
  const result: ConfigPayload = { expression: config.expression.trim(), result_type: config.resultType }
  const decimals = numberOrUndefined(config.decimals)
  if (formulaUsesDecimals(config.resultType) && decimals !== undefined) result.decimals = decimals
  if (formulaUsesCurrency(config.resultType)) {
    result.currency = config.currency.trim().toUpperCase() || DEFAULT_FORMULA_CURRENCY
  }
  return result
}

export function buildConfig(values: FieldFormValues): ConfigPayload {
  const { fieldType, config } = values
  if (isFormulaType(fieldType)) return buildFormulaConfig(config)
  const result: ConfigPayload = {}
  const set = (property: string, value: unknown) => {
    if (value !== undefined && value !== '') result[property] = value
  }
  if (isNumeric(fieldType)) {
    set('min', numberOrUndefined(config.min))
    set('max', numberOrUndefined(config.max))
    set('decimals', numberOrUndefined(config.decimals))
  }
  if (fieldType === 'currency') set('currency', config.currency.trim().toUpperCase())
  if (supportsMaxLength(fieldType)) set('max_length', numberOrUndefined(config.maxLength))
  if (fieldType === 'text') set('pattern', config.pattern.trim())
  if (fieldType === 'phone') set('default_country', config.defaultCountry.trim().toUpperCase())
  if (fieldType === 'record') set('target_record_type', config.targetRecordType.trim())
  if (supportsDefaultValue(fieldType)) set('default_value', defaultValuePayload(values))
  return result
}

function asRequestConfig(config: ConfigPayload): Record<string, never> {
  return config as Record<string, never>
}

function privacyPayload(values: FieldFormValues) {
  return {
    is_personal: values.isPersonal || values.isSensitive,
    is_sensitive: values.isSensitive,
  }
}

function flagsPayload(values: FieldFormValues) {
  return {
    ...(supportsRequired(values.fieldType) ? { is_required: values.isRequired } : {}),
    ...(supportsUnique(values.fieldType) ? { is_unique: values.isUnique } : {}),
    ...(supportsSearchable(values.fieldType) ? { is_searchable: values.isSearchable } : {}),
  }
}

export function createFieldRequest(values: FieldFormValues): CreateFieldRequest {
  const config = buildConfig(values)
  const withOptions = hasOptions(values.fieldType) && values.options.length > 0
  const optionValues = optionValuesFromLabels(values.options.map((option) => option.label))
  return {
    key: values.key.trim(),
    label: values.label.trim(),
    field_type: values.fieldType,
    ...flagsPayload(values),
    ...privacyPayload(values),
    ...(Object.keys(config).length > 0 ? { config: asRequestConfig(config) } : {}),
    ...(withOptions
      ? {
          options: values.options.map((option, index) => ({
            value: optionValues[index],
            label: option.label.trim(),
            ...(option.color.trim() ? { color: option.color.trim().toLowerCase() } : {}),
          })),
        }
      : {}),
  }
}

export function updateFieldRequest(values: FieldFormValues, field: Field): UpdateFieldRequest {
  if (field.is_system) return { label: values.label.trim(), ...privacyPayload(values) }
  return {
    label: values.label.trim(),
    ...(values.fieldType !== field.field_type ? { field_type: values.fieldType } : {}),
    ...flagsPayload(values),
    ...privacyPayload(values),
    config: asRequestConfig(buildConfig(values)),
  }
}
