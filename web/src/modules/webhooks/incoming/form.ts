import { z } from 'zod'
import type { ActionConfig, ActionType, CreateIncomingWebhookBody, IncomingWebhook } from './api'
import { webhookActions } from './api'

export const MAX_NAME_LENGTH = 100
export const MAX_DESCRIPTION_LENGTH = 500
export const MAX_ACTIONS = 10
export const DEFAULT_TOLERANCE_SECONDS = 300
export const MIN_TOLERANCE_SECONDS = 30
export const MAX_TOLERANCE_SECONDS = 86_400

const mappingSchema = z.object({
  key: z.string().min(1, 'form.errors.fieldRequired'),
  expression: z.string(),
})

const actionSchema = z.object({
  type: z.enum(['create_record', 'upsert_record']),
  record_type: z.string().min(1, 'form.errors.entityRequired'),
  match_field: z.string(),
  assignment_rule_id: z.string(),
  fields: z.array(mappingSchema),
}).superRefine((action, context) => {
  const keys = action.fields.map((field) => field.key).filter(Boolean)
  if (action.type === 'upsert_record' && (!action.match_field || !keys.includes(action.match_field))) {
    context.addIssue({ code: 'custom', path: ['match_field'], message: 'form.errors.matchFieldRequired' })
  }
  if (new Set(keys).size !== keys.length) context.addIssue({ code: 'custom', path: ['fields'], message: 'form.errors.duplicateField' })
})

export const webhookFormSchema = z.object({
  name: z.string().trim().min(1, 'form.errors.nameRequired').max(MAX_NAME_LENGTH, 'form.errors.nameTooLong'),
  description: z.string().max(MAX_DESCRIPTION_LENGTH, 'form.errors.descriptionTooLong'),
  is_active: z.boolean(),
  generate_signing_secret: z.boolean(),
  signature_tolerance_seconds: z
    .number({ message: 'form.errors.tolerance' })
    .int('form.errors.tolerance')
    .min(MIN_TOLERANCE_SECONDS, 'form.errors.tolerance')
    .max(MAX_TOLERANCE_SECONDS, 'form.errors.tolerance'),
  actions: z.array(actionSchema).max(MAX_ACTIONS, 'form.errors.tooManyActions'),
})

export type WebhookFormValues = z.infer<typeof webhookFormSchema>
export type ActionFormValues = WebhookFormValues['actions'][number]
export type MappingFormValues = ActionFormValues['fields'][number]

export function emptyAction(type: ActionType, recordType = ''): ActionFormValues {
  return { type, record_type: recordType, match_field: '', assignment_rule_id: '', fields: [] }
}

function expressionText(value: unknown): string {
  if (typeof value === 'string') return value
  if (value === null || value === undefined) return ''
  return JSON.stringify(value)
}

export function toFormValues(webhook: IncomingWebhook | null): WebhookFormValues {
  if (!webhook) {
    return {
      name: '',
      description: '',
      is_active: true,
      generate_signing_secret: false,
      signature_tolerance_seconds: DEFAULT_TOLERANCE_SECONDS,
      actions: [],
    }
  }
  return {
    name: webhook.name,
    description: webhook.description ?? '',
    is_active: webhook.is_active,
    generate_signing_secret: false,
    signature_tolerance_seconds: webhook.signature_tolerance_seconds,
    actions: webhookActions(webhook).map((action) => ({
      type: action.type,
      record_type: action.record_type,
      match_field: action.match_field ?? '',
      assignment_rule_id: action.assignment_rule_id ?? '',
      fields: Object.entries(action.fields ?? {}).map(([key, value]) => ({ key, expression: expressionText(value) })),
    })),
  }
}

function toActionConfig(action: ActionFormValues): ActionConfig {
  const fields = Object.fromEntries(
    action.fields.filter((field) => field.key).map((field) => [field.key, field.expression]),
  )
  return { type: action.type, record_type: action.record_type, match_field: action.type === 'upsert_record' ? action.match_field : null, assignment_rule_id: action.assignment_rule_id || null, fields }
}

export function toRequestBody(values: WebhookFormValues): CreateIncomingWebhookBody {
  const description = values.description.trim()
  return {
    name: values.name.trim(),
    description: description || null,
    is_active: values.is_active,
    signature_tolerance_seconds: values.signature_tolerance_seconds,
    generate_signing_secret: values.generate_signing_secret,
    actions: values.actions.map(toActionConfig) as unknown as CreateIncomingWebhookBody['actions'],
  }
}

export function samplePayload(actions: ActionFormValues[]): Record<string, string> {
  const sample: Record<string, string> = {}
  const pattern = /\$\.([A-Za-z0-9_-]+)/g
  for (const action of actions) {
    for (const field of action.fields) {
      for (const match of field.expression.matchAll(pattern)) {
        const key = match[1]
        if (key && !(key in sample)) sample[key] = exampleValue(key)
      }
    }
  }
  return sample
}

function exampleValue(key: string): string {
  const normalized = key.toLowerCase()
  if (normalized.includes('mail')) return 'ana@exemplo.com.br'
  if (normalized.includes('phone') || normalized.includes('telefone') || normalized.includes('celular'))
    return '(11) 98888-7777'
  if (normalized.includes('name') || normalized.includes('nome')) return 'Ana Souza'
  return 'valor'
}

export function curlExample(url: string, payload: Record<string, string>, signed: boolean): string {
  const entries = Object.entries(payload)
  const data = entries.length > 0 ? entries : [['nome', 'Ana Souza']]
  if (signed) {
    const body = JSON.stringify(Object.fromEntries(data))
    return [
      `BODY='${body}'`,
      'TS=$(date +%s)',
      `SIG=$(printf '%s.%s' "$TS" "$BODY" | openssl dgst -sha256 -hmac "$APHEL_WEBHOOK_SECRET" -hex | sed 's/^.* //')`,
      [
        `curl -X POST '${url}'`,
        "  -H 'Content-Type: application/json'",
        '  -H "X-Aphel-Signature: t=$TS,v1=$SIG"',
        '  -d "$BODY"',
      ].join(' \\\n'),
    ].join('\n')
  }
  const lines = [`curl -X POST '${url}'`, "  -H 'Content-Type: application/x-www-form-urlencoded'"]
  for (const [key, value] of data) {
    lines.push(`  --data-urlencode '${key}=${value}'`)
  }
  return lines.join(' \\\n')
}
