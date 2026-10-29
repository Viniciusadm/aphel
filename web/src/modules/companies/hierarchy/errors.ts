import type { TFunction } from 'i18next'
import { isApiError } from '@/api'

export function hierarchyErrorMessage(error: unknown, t: TFunction<'companies', 'hierarchy'>): string {
  if (!isApiError(error)) return t('errors.generic')
  switch (error.code) {
    case 'company_hierarchy_cycle':
      return t('errors.cycle')
    case 'company_hierarchy_too_deep':
      return t('errors.tooDeep')
    case 'permission_denied':
    case 'insufficient_scope':
      return t('errors.forbidden')
    case 'company_not_found':
      return t('errors.notFound')
    case 'validation_error':
      return t('errors.invalid')
    default:
      return t('errors.generic')
  }
}
