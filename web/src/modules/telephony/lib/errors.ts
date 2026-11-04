import { isApiError } from '@/api'

export type CallErrorKey =
  | 'errors.generic'
  | 'errors.forbidden'
  | 'errors.notFound'
  | 'errors.invalid'
  | 'errors.keyTaken'
  | 'errors.lastActiveOutcome'
  | 'errors.versionConflict'
  | 'errors.sessionCompleted'
  | 'errors.sessionEmpty'
  | 'errors.viewNotFound'
  | 'errors.providerNotFound'
  | 'errors.noCallType'

export function callErrorKey(error: unknown): CallErrorKey {
  if (!isApiError(error)) return 'errors.generic'
  switch (error.code) {
    case 'permission_denied':
    case 'insufficient_scope':
      return 'errors.forbidden'
    case 'call_not_found':
    case 'call_outcome_not_found':
    case 'call_session_not_found':
    case 'call_session_item_not_found':
    case 'call_recording_not_found':
    case 'activity_not_found':
      return 'errors.notFound'
    case 'call_outcome_key_taken':
      return 'errors.keyTaken'
    case 'last_active_call_outcome':
      return 'errors.lastActiveOutcome'
    case 'version_conflict':
      return 'errors.versionConflict'
    case 'call_session_completed':
      return 'errors.sessionCompleted'
    case 'call_session_empty':
      return 'errors.sessionEmpty'
    case 'call_session_view_not_found':
      return 'errors.viewNotFound'
    case 'telephony_provider_not_found':
      return 'errors.providerNotFound'
    case 'call_activity_type_missing':
      return 'errors.noCallType'
    case 'validation_error':
      return 'errors.invalid'
    default:
      return 'errors.generic'
  }
}
