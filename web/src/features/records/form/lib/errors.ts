import { NETWORK_ERROR_CODE, isApiError, localizeViolationMessage } from '@/api'
import { errorPathToFieldKey, fieldViolations } from '../../fields'
import type { FieldDefinition } from '../../types'
import { serverMessageIssue } from './messages'
import { issueMessage, type RecordFormT } from './validation'

export interface MappedFieldError {
  key: string
  message: string
}

export interface MappedRecordError {
  fields: MappedFieldError[]
  general: string | null
}

const CODE_MESSAGES = {
  [NETWORK_ERROR_CODE]: 'errors.network',
  permission_denied: 'errors.permissionDenied',
  forbidden: 'errors.permissionDenied',
  not_found: 'errors.notFound',
  record_not_found: 'errors.notFound',
} as const

const VERBATIM_MESSAGE_CODES = new Set(['validation_rule_failed'])

function isVerbatim(code: string): boolean {
  return VERBATIM_MESSAGE_CODES.has(code)
}

function serverViolationMessage(message: string, t: RecordFormT): string {
  const issue = serverMessageIssue(message)
  if (issue.code !== 'invalid') return issueMessage(issue, t)
  const localized = localizeViolationMessage(message)
  return localized === message ? issueMessage(issue, t) : localized
}

function codeMessage(code: string, t: RecordFormT): string {
  const key = CODE_MESSAGES[code as keyof typeof CODE_MESSAGES]
  return t(key ?? 'errors.generic')
}

export function mapRecordApiError(
  error: unknown,
  visibleKeys: string[],
  t: RecordFormT,
  knownFields: FieldDefinition[] = [],
): MappedRecordError {
  if (!isApiError(error)) return { fields: [], general: t('errors.generic') }
  const violations = fieldViolations(error.details)
  if (violations.length === 0) return { fields: [], general: codeMessage(error.code, t) }

  const visible = new Set(visibleKeys)
  const verbatim = isVerbatim(error.code)
  const fields: MappedFieldError[] = []
  const unmatched: string[] = []
  for (const violation of violations) {
    const key = errorPathToFieldKey(violation.field)
    const message = verbatim ? violation.message : serverViolationMessage(violation.message, t)
    if (visible.has(key) && !fields.some((item) => item.key === key)) {
      fields.push({ key, message })
      continue
    }
    if (visible.has(key) && !verbatim) continue
    if (verbatim) {
      unmatched.push(message)
      continue
    }
    const label = knownFields.find((field) => field.key === key)?.label ?? key
    unmatched.push(t('errors.fieldMessage', { field: label, message }))
  }
  const general =
    unmatched.length === 0 ? null : verbatim ? unmatched.join(' ') : t('errors.invalidFields', { fields: unmatched.join('; ') })
  return { fields, general }
}

export function fieldErrorMessage(
  error: unknown,
  fieldKey: string,
  t: RecordFormT,
  knownFields: FieldDefinition[] = [],
): string {
  if (isApiError(error)) {
    const violations = fieldViolations(error.details)
    if (isVerbatim(error.code) && violations.length > 0) return violations.map((item) => item.message).join(' ')
    const violation = violations.find((item) => errorPathToFieldKey(item.field) === fieldKey)
    if (violation) return serverViolationMessage(violation.message, t)
    if (violations.length > 0) {
      const others = violations.map((item) => {
        const key = errorPathToFieldKey(item.field)
        const label = knownFields.find((field) => field.key === key)?.label ?? key
        return t('errors.fieldMessage', { field: label, message: serverViolationMessage(item.message, t) })
      })
      return t('errors.invalidFields', { fields: others.join('; ') })
    }
    return codeMessage(error.code, t)
  }
  return t('errors.generic')
}
