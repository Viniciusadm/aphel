import type { ParseKeys } from 'i18next'
import { isApiError } from '@/api'
import { errorMessage } from '@/components/feedback'
import { toast } from '@/components/ui/toast-store'
import i18n from '@/i18n'

type ObjectsKey = ParseKeys<'objects'>

const ERROR_KEYS: Record<string, ObjectsKey> = {
  custom_object_key_taken: 'errors.customObjectKeyTaken',
  too_many_custom_objects: 'errors.tooManyCustomObjects',
  custom_object_has_records: 'errors.customObjectHasRecords',
  too_many_relationships: 'errors.tooManyRelationships',
  version_conflict: 'errors.versionConflict',
  record_has_relations: 'errors.recordHasRelations',
  relation_required: 'errors.relationRequired',
  relation_limit_reached: 'errors.relationLimitReached',
  relation_not_found: 'errors.relationNotFound',
  record_not_accessible: 'errors.recordNotAccessible',
  validation_error: 'errors.validation',
}

export function objectsErrorMessage(error: unknown): string {
  if (isApiError(error)) {
    const key = ERROR_KEYS[error.code]
    if (key) return i18n.t(key, { ns: 'objects' })
  }
  return errorMessage(error)
}

export function notifyObjectsError(error: unknown) {
  toast({
    title: i18n.t('errors.title', { ns: 'objects' }),
    description: objectsErrorMessage(error),
    variant: 'destructive',
  })
}

export function hasErrorCode(error: unknown, code: string): boolean {
  return isApiError(error) && error.code === code
}
