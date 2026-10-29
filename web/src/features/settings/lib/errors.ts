import type { ParseKeys } from 'i18next'
import type { FieldValues, Path, UseFormSetError } from 'react-hook-form'
import { NETWORK_ERROR_CODE, isApiError } from '@/api'

export type SettingsMessageKey = ParseKeys<'settings'>

export const SERVER_ERROR_PATH = 'root.server'

const codeMessages: Record<string, SettingsMessageKey> = {
  [NETWORK_ERROR_CODE]: 'errors.network',
  permission_denied: 'errors.permissionDenied',
  member_not_found: 'errors.notFound',
  role_not_found: 'errors.notFound',
  team_not_found: 'errors.notFound',
  not_found: 'errors.notFound',
  already_member: 'errors.alreadyMember',
  last_admin: 'errors.lastAdmin',
  role_in_use: 'errors.roleInUse',
  role_key_taken: 'errors.roleKeyTaken',
  system_role_locked: 'errors.systemRoleLocked',
  team_cycle: 'errors.teamCycle',
  not_invited: 'errors.notInvited',
  invalid_member_status: 'errors.invalidMemberStatus',
  version_conflict: 'errors.versionConflict',
}

const fieldMessages: Record<string, SettingsMessageKey> = {
  email: 'validation.email',
  role_id: 'validation.role',
  name: 'validation.name',
  key: 'validation.roleKey',
}

interface FieldViolation {
  field: string
  message: string
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

export function errorMessageKey(error: unknown): SettingsMessageKey {
  if (!isApiError(error)) return 'errors.generic'
  const violations = fieldViolations(error.details)
  if (violations.some((violation) => violation.field.startsWith('permissions'))) return 'errors.invalidPermissions'
  if (violations.some((violation) => violation.field.startsWith('members'))) return 'errors.invalidMembers'
  return codeMessages[error.code] ?? 'errors.generic'
}

export interface ApiErrorMapping<T extends FieldValues> {
  fields?: Partial<Record<string, Path<T>>>
  codes?: Partial<Record<string, Path<T>>>
}

export function applyApiError<T extends FieldValues>(
  error: unknown,
  setError: UseFormSetError<T>,
  mapping: ApiErrorMapping<T> = {},
): void {
  if (isApiError(error)) {
    let mapped = false
    for (const violation of fieldViolations(error.details)) {
      const name = mapping.fields?.[violation.field]
      if (!name) continue
      setError(
        name,
        { type: 'server', message: fieldMessages[violation.field] ?? 'errors.generic' },
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

export function asMessageKey(message: string | undefined): SettingsMessageKey | undefined {
  return message ? (message as SettingsMessageKey) : undefined
}
