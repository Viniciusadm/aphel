import { isApiError } from '@/api'

const KNOWN_ERRORS = [
  'price_list_not_found',
  'price_list_key_taken',
  'version_conflict',
  'permission_denied',
  'validation_error',
] as const

export type PriceListErrorKey = `errors.${(typeof KNOWN_ERRORS)[number]}` | 'errors.generic'

export function priceListErrorKey(error: unknown): PriceListErrorKey {
  if (isApiError(error) && (KNOWN_ERRORS as readonly string[]).includes(error.code)) {
    return `errors.${error.code as (typeof KNOWN_ERRORS)[number]}`
  }
  return 'errors.generic'
}

export interface FieldErrorDetail {
  field: string
  message: string
}

export function fieldErrors(error: unknown): FieldErrorDetail[] {
  if (!isApiError(error)) return []
  const details = error.details as { fields?: unknown } | null
  if (!details || !Array.isArray(details.fields)) return []
  return details.fields.filter(
    (entry): entry is FieldErrorDetail =>
      typeof entry === 'object' &&
      entry !== null &&
      typeof (entry as FieldErrorDetail).field === 'string' &&
      typeof (entry as FieldErrorDetail).message === 'string',
  )
}
