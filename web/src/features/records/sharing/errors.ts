import type { TFunction } from 'i18next'
import { isApiError } from '@/api'

export function sharingErrorMessage(error: unknown, t: TFunction<'sharing'>): string {
  if (!isApiError(error)) return t('errors.generic')
  switch (error.code) {
    case 'permission_denied':
    case 'insufficient_scope':
    case 'record_share_not_allowed':
      return t('errors.forbidden')
    case 'record_not_found':
    case 'record_share_not_found':
      return t('errors.notFound')
    case 'record_share_exists':
      return t('errors.exists')
    case 'record_shares_limit_reached':
      return t('errors.limit')
    case 'version_conflict':
      return t('errors.versionConflict')
    case 'validation_error':
      return t('errors.invalid')
    default:
      return t('errors.generic')
  }
}
