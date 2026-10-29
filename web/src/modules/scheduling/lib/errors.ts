import { isApiError } from '@/api'

export interface ServerFieldViolation {
  field: string
  message: string
}

export function serverFieldViolations(error: unknown): ServerFieldViolation[] {
  if (!isApiError(error) || error.code !== 'validation_error') return []
  const fields = (error.details as { fields?: unknown } | null)?.fields
  if (!Array.isArray(fields)) return []
  return fields.filter(
    (item): item is ServerFieldViolation =>
      typeof item === 'object' && item !== null && typeof (item as ServerFieldViolation).field === 'string',
  )
}

export function matchServerField<T extends string>(field: string, paths: Record<string, T>): T | null {
  const prefix = Object.keys(paths).find(
    (path) => field === path || field.startsWith(`${path}.`) || field.startsWith(`${path}[`),
  )
  return prefix ? paths[prefix] : null
}
