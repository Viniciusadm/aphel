import type { FieldValues, Path, UseFormSetError } from 'react-hook-form'
import { NETWORK_ERROR_CODE, isApiError } from '@/api'
import type { AuthMessageKey } from './messages'

export const INVALID_TOKEN_CODE = 'invalid_token'
export const RATE_LIMITED_CODE = 'rate_limited'
export const UNAUTHORIZED_CODE = 'unauthorized'
export const EMAIL_TAKEN_CODE = 'email_taken'
export const LOGIN_LOCKED_CODE = 'login_locked'

export const SERVER_ERROR_PATH = 'root.server'

interface FieldViolation {
  field: string
  message: string
}

const serverFieldMessages: Record<string, AuthMessageKey> = {
  password: 'validation.passwordLength',
  email: 'validation.email',
  organization_name: 'validation.nameLength',
  display_name: 'validation.nameLength',
}

export interface CodeErrorMapping<T extends FieldValues> {
  field?: Path<T>
  message: AuthMessageKey
}

export interface ApiErrorMapping<T extends FieldValues> {
  fields?: Partial<Record<string, Path<T>>>
  codes?: Partial<Record<string, CodeErrorMapping<T>>>
}

export function isInvalidTokenError(error: unknown): boolean {
  return isApiError(error) && error.code === INVALID_TOKEN_CODE
}

export function fieldViolations(details: unknown): FieldViolation[] {
  if (typeof details !== 'object' || details === null) return []
  const fields = (details as { fields?: unknown }).fields
  if (!Array.isArray(fields)) return []
  return fields.filter(
    (item): item is FieldViolation =>
      typeof item === 'object' &&
      item !== null &&
      typeof (item as FieldViolation).field === 'string' &&
      typeof (item as FieldViolation).message === 'string',
  )
}

export function retryAfterSeconds(error: unknown): number | null {
  if (!isApiError(error)) return null
  const details = error.details
  if (typeof details === 'object' && details !== null) {
    const value = (details as { retry_after?: unknown }).retry_after
    if (typeof value === 'number' && Number.isFinite(value) && value > 0) return Math.ceil(value)
  }
  const minutes = /(\d+)\s*minute/.exec(error.message)
  return minutes ? Number(minutes[1]) * 60 : null
}

export function isLoginLockedError(error: unknown): boolean {
  return isApiError(error) && error.code === LOGIN_LOCKED_CODE
}

export function generalErrorMessage(error: unknown): AuthMessageKey {
  if (!isApiError(error)) return 'errors.generic'
  if (error.code === NETWORK_ERROR_CODE) return 'errors.network'
  if (error.code === RATE_LIMITED_CODE || error.status === 429) return 'errors.rateLimited'
  return 'errors.generic'
}

export function applyApiError<T extends FieldValues>(
  error: unknown,
  setError: UseFormSetError<T>,
  mapping: ApiErrorMapping<T> = {},
): void {
  const setServerError = (message: AuthMessageKey) => setError(SERVER_ERROR_PATH, { type: 'server', message })

  if (!isApiError(error)) {
    setServerError('errors.generic')
    return
  }

  let focused = false
  for (const violation of fieldViolations(error.details)) {
    const name = mapping.fields?.[violation.field]
    if (!name) continue
    setError(
      name,
      { type: 'server', message: serverFieldMessages[violation.field] ?? 'validation.invalid' },
      { shouldFocus: !focused },
    )
    focused = true
  }
  if (focused) return

  const codeMapping = mapping.codes?.[error.code]
  if (codeMapping?.field) {
    setError(codeMapping.field, { type: 'server', message: codeMapping.message }, { shouldFocus: true })
    return
  }
  if (codeMapping) {
    setServerError(codeMapping.message)
    return
  }

  setServerError(generalErrorMessage(error))
}
