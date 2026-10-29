import type { TFunction } from 'i18next'
import { isApiError } from '@/api'

export function lifecycleErrorMessage(error: unknown, t: TFunction<'leads'>): string {
  if (!isApiError(error)) return t('errors.generic')
  switch (error.code) {
    case 'permission_denied':
    case 'insufficient_scope':
    case 'field_read_only':
      return t('errors.forbidden')
    case 'contact_not_found':
    case 'record_not_found':
    case 'disqualification_reason_not_found':
      return t('errors.notFound')
    case 'pipeline_not_found':
      return t('errors.pipelineNotFound')
    case 'contact_not_lead':
      return t('errors.notLead')
    case 'disqualification_reason_key_taken':
      return t('errors.keyTaken')
    case 'version_conflict':
      return t('errors.versionConflict')
    case 'validation_error':
      return validationMessage(error.details, t)
    default:
      return error.message || t('errors.generic')
  }
}

function validationMessage(details: unknown, t: TFunction<'leads'>): string {
  if (typeof details === 'object' && details !== null) {
    const fields = (details as Record<string, unknown>).fields
    if (Array.isArray(fields) && fields.length > 0) {
      const first = fields[0] as Record<string, unknown>
      const field = typeof first.field === 'string' ? first.field : ''
      const message = typeof first.message === 'string' ? first.message : ''
      if (field && message) return t('errors.invalidField', { field, message })
    }
  }
  return t('errors.invalid')
}
