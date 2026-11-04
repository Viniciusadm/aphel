import { isApiError } from '@/api'

const KNOWN_ERRORS = [
  'product_bundle_not_found',
  'product_bundle_nested',
  'product_not_found',
  'price_list_not_found',
  'version_conflict',
  'validation_error',
] as const

export type BundleErrorKey = `bundles.errors.${(typeof KNOWN_ERRORS)[number]}` | 'bundles.errors.generic'

export function bundleErrorKey(error: unknown): BundleErrorKey {
  if (isApiError(error) && (KNOWN_ERRORS as readonly string[]).includes(error.code)) {
    return `bundles.errors.${error.code as (typeof KNOWN_ERRORS)[number]}`
  }
  return 'bundles.errors.generic'
}
