import { isApiError } from '@/api'

export type ActivityErrorKey =
  | 'errors.generic'
  | 'errors.forbidden'
  | 'errors.notFound'
  | 'errors.alreadyCompleted'
  | 'errors.alreadyOpen'
  | 'errors.invalid'
  | 'errors.keyTaken'
  | 'errors.lastActiveType'
  | 'errors.versionConflict'

export function activityErrorKey(error: unknown): ActivityErrorKey {
  if (!isApiError(error)) return 'errors.generic'
  switch (error.code) {
    case 'activity_already_completed':
      return 'errors.alreadyCompleted'
    case 'activity_already_open':
      return 'errors.alreadyOpen'
    case 'activity_type_key_taken':
      return 'errors.keyTaken'
    case 'last_active_activity_type':
      return 'errors.lastActiveType'
    case 'permission_denied':
      return 'errors.forbidden'
    case 'activity_not_found':
    case 'activity_type_not_found':
      return 'errors.notFound'
    case 'version_conflict':
      return 'errors.versionConflict'
    case 'validation_error':
      return 'errors.invalid'
    default:
      return 'errors.generic'
  }
}
