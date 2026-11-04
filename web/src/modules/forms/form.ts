import { z } from 'zod'
import { FULL_NAME_TARGET, type LeadForm, type LeadFormBody, type LeadFormFieldKind } from './api'

export const MAX_FIELDS = 30
export const MAX_NAME_LENGTH = 120
export const MAX_LABEL_LENGTH = 120
export const MAX_MESSAGE_LENGTH = 500
const KEY_PATTERN = /^[a-z][a-z0-9_]{0,39}$/
const URL_PATTERN = /^https?:\/\/[^\s/"<>][^\s"<>]*$/
const ORIGIN_PATTERN = /^https?:\/\/[a-z0-9.\-:[\]]+\/?$/i
const AMOUNT_PATTERN = /^\d{1,16}([.,]\d{1,2})?$/

const fieldSchema = z
  .object({
    key: z.string().trim().regex(KEY_PATTERN, 'errors.key'),
    label: z.string().trim().min(1, 'errors.labelRequired').max(MAX_LABEL_LENGTH, 'errors.tooLong'),
    kind: z.enum(['text', 'email', 'phone', 'textarea', 'select', 'checkbox']),
    required: z.boolean(),
    placeholder: z.string().max(120, 'errors.tooLong'),
    options: z.string(),
    target: z.string(),
  })
  .superRefine((field, context) => {
    if (field.kind === 'select' && parseOptions(field.options).length === 0) {
      context.addIssue({ code: 'custom', path: ['options'], message: 'errors.optionsRequired' })
    }
    if ((field.target === 'first_name' || field.target === FULL_NAME_TARGET) && !field.required) {
      context.addIssue({ code: 'custom', path: ['required'], message: 'errors.nameFieldRequired' })
    }
  })

export const leadFormSchema = z
  .object({
    name: z.string().trim().min(1, 'errors.nameRequired').max(MAX_NAME_LENGTH, 'errors.tooLong'),
    is_active: z.boolean(),
    acting_user_id: z.string(),
    fields: z.array(fieldSchema).min(1, 'errors.fieldsRequired').max(MAX_FIELDS, 'errors.tooManyFields'),
    create_deal: z.boolean(),
    pipeline_id: z.string(),
    stage_id: z.string(),
    title_template: z.string().max(200, 'errors.tooLong'),
    amount: z.string().trim(),
    tag_id: z.string(),
    owner_id: z.string(),
    distribution_rule_id: z.string(),
    source: z.string().max(120, 'errors.tooLong'),
    success_message: z.string().max(MAX_MESSAGE_LENGTH, 'errors.tooLong'),
    submit_label: z.string().max(60, 'errors.tooLong'),
    redirect_url: z.string().trim(),
    allowed_origins: z.string(),
    dedupe_by_email: z.boolean(),
  })
  .superRefine((values, context) => {
    const keys = values.fields.map((field) => field.key.trim())
    keys.forEach((key, index) => {
      if (key && keys.indexOf(key) !== index) {
        context.addIssue({ code: 'custom', path: ['fields', index, 'key'], message: 'errors.duplicateKey' })
      }
    })
    const targets = values.fields.map((field) => field.target).filter(Boolean)
    targets.forEach((target, index) => {
      if (targets.indexOf(target) !== index) {
        context.addIssue({ code: 'custom', path: ['fields'], message: 'errors.duplicateTarget' })
      }
    })
    if (!targets.includes('first_name') && !targets.includes(FULL_NAME_TARGET)) {
      context.addIssue({ code: 'custom', path: ['fields'], message: 'errors.nameMappingRequired' })
    }
    if (values.create_deal) {
      if (!values.pipeline_id) context.addIssue({ code: 'custom', path: ['pipeline_id'], message: 'errors.pipelineRequired' })
      if (!values.stage_id) context.addIssue({ code: 'custom', path: ['stage_id'], message: 'errors.stageRequired' })
      if (!values.title_template.trim()) {
        context.addIssue({ code: 'custom', path: ['title_template'], message: 'errors.titleRequired' })
      }
      if (values.amount && !AMOUNT_PATTERN.test(values.amount)) {
        context.addIssue({ code: 'custom', path: ['amount'], message: 'errors.amount' })
      }
    }
    if (values.redirect_url && !URL_PATTERN.test(values.redirect_url)) {
      context.addIssue({ code: 'custom', path: ['redirect_url'], message: 'errors.url' })
    }
    if (parseLines(values.allowed_origins).some((origin) => !ORIGIN_PATTERN.test(origin))) {
      context.addIssue({ code: 'custom', path: ['allowed_origins'], message: 'errors.origin' })
    }
  })

export type LeadFormValues = z.infer<typeof leadFormSchema>
export type LeadFormFieldValues = LeadFormValues['fields'][number]

export interface ParsedOption {
  value: string
  label: string
}

export function parseLines(text: string): string[] {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
}

export function parseOptions(text: string): ParsedOption[] {
  return parseLines(text).map((line) => {
    const separator = line.indexOf('|')
    if (separator < 0) return { value: line, label: line }
    const value = line.slice(0, separator).trim()
    const label = line.slice(separator + 1).trim()
    return { value: value || label, label: label || value }
  })
}

export function slugify(text: string): string {
  const slug = text
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 40)
  return /^[a-z]/.test(slug) ? slug : slug ? `f_${slug}`.slice(0, 40) : ''
}

export function emptyField(kind: LeadFormFieldKind, index: number): LeadFormFieldValues {
  return {
    key: `field_${index + 1}`,
    label: '',
    kind,
    required: false,
    placeholder: '',
    options: '',
    target: '',
  }
}

export function defaultFields(labels: { name: string; email: string; phone: string; message: string }): LeadFormFieldValues[] {
  return [
    { key: 'name', label: labels.name, kind: 'text', required: true, placeholder: '', options: '', target: FULL_NAME_TARGET },
    { key: 'email', label: labels.email, kind: 'email', required: true, placeholder: '', options: '', target: 'email' },
    { key: 'phone', label: labels.phone, kind: 'phone', required: false, placeholder: '', options: '', target: 'mobile' },
    { key: 'message', label: labels.message, kind: 'textarea', required: false, placeholder: '', options: '', target: 'notes' },
  ]
}

export function toFormValues(form: LeadForm | null, fields: LeadFormFieldValues[]): LeadFormValues {
  return {
    name: form?.name ?? '',
    is_active: form?.is_active ?? true,
    acting_user_id: form?.acting_user_id ?? '',
    fields: form
      ? (form.fields ?? []).map((field) => ({
          key: field.key,
          label: field.label,
          kind: field.kind,
          required: field.required ?? false,
          placeholder: field.placeholder ?? '',
          options: (field.options ?? [])
            .map((option) => (option.value === option.label ? option.value : `${option.value} | ${option.label}`))
            .join('\n'),
          target: field.target ?? '',
        }))
      : fields,
    create_deal: Boolean(form?.deal),
    pipeline_id: form?.deal?.pipeline_id ?? '',
    stage_id: form?.deal?.stage_id ?? '',
    title_template: form?.deal?.title_template ?? '{{name}} - {{form}}',
    amount: form?.deal?.amount ?? '',
    tag_id: form?.tag_id ?? '',
    owner_id: form?.owner_id ?? '',
    distribution_rule_id: form?.distribution_rule_id ?? '',
    source: form?.source ?? '',
    success_message: form?.success_message ?? '',
    submit_label: form?.submit_label ?? '',
    redirect_url: form?.redirect_url ?? '',
    allowed_origins: (form?.allowed_origins ?? []).join('\n'),
    dedupe_by_email: form?.dedupe_by_email ?? true,
  }
}

function optional(value: string): string | null {
  const trimmed = value.trim()
  return trimmed ? trimmed : null
}

export function toRequestBody(values: LeadFormValues): LeadFormBody {
  return {
    name: values.name.trim(),
    is_active: values.is_active,
    acting_user_id: optional(values.acting_user_id),
    fields: values.fields.map((field) => ({
      key: field.key.trim(),
      label: field.label.trim(),
      kind: field.kind,
      required: field.required,
      placeholder: optional(field.placeholder),
      options: field.kind === 'select' ? parseOptions(field.options) : [],
      target: optional(field.target),
    })),
    deal: values.create_deal
      ? {
          pipeline_id: values.pipeline_id,
          stage_id: values.stage_id,
          title_template: values.title_template.trim(),
          amount: optional(values.amount.replace(',', '.')),
        }
      : null,
    tag_id: optional(values.tag_id),
    owner_id: optional(values.owner_id),
    distribution_rule_id: optional(values.distribution_rule_id),
    source: optional(values.source),
    success_message: optional(values.success_message),
    submit_label: optional(values.submit_label),
    redirect_url: optional(values.redirect_url),
    allowed_origins: parseLines(values.allowed_origins),
    dedupe_by_email: values.dedupe_by_email,
  }
}
