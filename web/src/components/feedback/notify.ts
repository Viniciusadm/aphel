import type { ParseKeys } from 'i18next'
import { NETWORK_ERROR_CODE, isApiError } from '@/api'
import { toast } from '@/components/ui/toast-store'
import i18n from '@/i18n'
import { formatRuleError } from './rule-errors'

export type FeedbackMessageKey = ParseKeys<'records-detail'>

export function errorMessageKey(error: unknown): FeedbackMessageKey {
  if (!isApiError(error)) return 'notify.errors.generic'
  if (error.code === NETWORK_ERROR_CODE) return 'notify.errors.network'
  switch (error.status) {
    case 400:
    case 422:
      return 'notify.errors.validation'
    case 401:
      return 'notify.errors.unauthorized'
    case 403:
      return 'notify.errors.forbidden'
    case 404:
      return 'notify.errors.notFound'
    case 409:
      return 'notify.errors.conflict'
    case 413:
      return 'notify.errors.tooLarge'
    case 429:
      return 'notify.errors.rateLimited'
    default:
      return error.status >= 500 ? 'notify.errors.server' : 'notify.errors.generic'
  }
}

function violationMessages(details: unknown): string[] {
  if (typeof details !== 'object' || details === null) return []
  const fields = (details as Record<string, unknown>).fields
  if (!Array.isArray(fields)) return []
  return fields
    .map((item: unknown) =>
      typeof item === 'object' && item !== null ? (item as Record<string, unknown>).message : undefined,
    )
    .filter((message): message is string => typeof message === 'string' && message.trim() !== '')
}

export function ruleErrorMessage(error: unknown): string | null {
  if (!isApiError(error)) return null
  const messages = violationMessages(error.details)
  if (messages.length === 0) return null
  return formatRuleError(error.code, messages)
}

const translateLoose = i18n.t.bind(i18n) as unknown as (key: string, options: object) => string

function lookup(namespace: string, key: string): string | null {
  const text = translateLoose(key, { ns: namespace, defaultValue: '' })
  return text === '' ? null : text
}

function codeMessage(error: unknown): string | null {
  if (!isApiError(error)) return null
  const hosted = lookup('records-detail', `notify.codes.${error.code}`)
  if (hosted) return hosted
  const parts = error.code.split('_')
  for (let length = parts.length - 1; length > 0; length--) {
    const text = lookup(parts.slice(0, length).join('_'), `errors.${error.code}`)
    if (text) return text
  }
  return null
}

export function errorMessage(error: unknown): string {
  if (typeof error === 'string' && error.trim() !== '') return error
  return ruleErrorMessage(error) ?? codeMessage(error) ?? i18n.t(errorMessageKey(error), { ns: 'records-detail' })
}

export interface NotifyOptions {
  description?: string
  duration?: number
}

export function notifySuccess(message: string, options: NotifyOptions = {}): number {
  return toast({ title: message, description: options.description, duration: options.duration, variant: 'success' })
}

export interface NotifyErrorOptions {
  title?: string
  duration?: number
}

export function notifyError(error: unknown, options: NotifyErrorOptions = {}): number {
  return toast({
    title: options.title ?? i18n.t('notify.errorTitle', { ns: 'records-detail' }),
    description: errorMessage(error),
    duration: options.duration,
    variant: 'destructive',
  })
}
