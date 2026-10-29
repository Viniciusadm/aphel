import type { DuplicateRecordType } from './api'

export const DUPLICATES_SETTINGS_PATH = '/data/duplicates'

export function mergePagePath(recordType: DuplicateRecordType, primaryId: string, duplicateId: string): string {
  const params = new URLSearchParams({ type: recordType, primary: primaryId, duplicate: duplicateId })
  return `${DUPLICATES_SETTINGS_PATH}/merge?${params.toString()}`
}
