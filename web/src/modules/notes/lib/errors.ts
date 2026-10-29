import { isApiError } from '@/api'

export type NoteErrorKey = 'errors.generic' | 'errors.forbidden' | 'errors.notFound' | 'errors.invalid'

export function noteErrorKey(error: unknown): NoteErrorKey {
  if (!isApiError(error)) return 'errors.generic'
  if (error.code === 'permission_denied') return 'errors.forbidden'
  if (error.code === 'note_not_found') return 'errors.notFound'
  if (error.code === 'validation_error') return 'errors.invalid'
  return 'errors.generic'
}
