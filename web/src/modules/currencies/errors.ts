import { isApiError } from '@/api'

const KNOWN_ERRORS = [
  'currency_not_enabled',
  'exchange_rate_exists',
  'exchange_rate_not_found',
  'exchange_rate_provider_not_configured',
  'exchange_rate_provider_unavailable',
  'version_conflict',
] as const

export type CurrencyErrorKey = `errors.${(typeof KNOWN_ERRORS)[number]}` | 'errors.generic'

export function currencyErrorKey(error: unknown): CurrencyErrorKey {
  if (isApiError(error) && (KNOWN_ERRORS as readonly string[]).includes(error.code)) {
    return `errors.${error.code as (typeof KNOWN_ERRORS)[number]}`
  }
  return 'errors.generic'
}
