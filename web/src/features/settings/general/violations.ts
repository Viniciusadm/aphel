import { isApiError } from '@/api'
import { fieldViolations } from '@/features/auth/lib/api-errors'

export function serverFieldErrors(error: unknown, fields: Record<string, string>): Array<[string, string]> {
  if (!isApiError(error)) return []
  return fieldViolations(error.details).flatMap((violation) => {
    const field = fields[violation.field]
    return field ? [[field, violation.message] as [string, string]] : []
  })
}
