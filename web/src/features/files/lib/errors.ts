import type { ParseKeys } from 'i18next'
import { NETWORK_ERROR_CODE, isApiError } from '@/api'

export type FilesMessageKey = ParseKeys<'files'>

const codeMessages: Record<string, FilesMessageKey> = {
  [NETWORK_ERROR_CODE]: 'errors.network',
  file_too_large: 'errors.fileTooLarge',
  file_type_not_allowed: 'errors.fileTypeNotAllowed',
  empty_file: 'errors.emptyFile',
  permission_denied: 'errors.permissionDenied',
  attachment_not_found: 'errors.notFound',
  file_not_found: 'errors.notFound',
  already_attached: 'errors.alreadyAttached',
  unsupported_record_type: 'errors.unsupportedRecordType',
}

export function filesErrorKey(error: unknown): FilesMessageKey {
  if (!isApiError(error)) return 'errors.generic'
  if (error.status === 413) return 'errors.fileTooLarge'
  return codeMessages[error.code] ?? 'errors.generic'
}
