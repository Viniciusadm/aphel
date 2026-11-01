import { registerRecordResource } from '@/features/records/resources'
import type { components } from '@/api/schema'

export type CustomObject = components['schemas']['CustomObjectResponse']

export const CUSTOM_RECORD_TYPE_PREFIX = 'co_'
export const CUSTOM_RECORD_TITLE_FIELD = 'title'
export const CUSTOM_RECORD_READ_ONLY_KEYS = [
  'id',
  'title',
  'created_by',
  'created_at',
  'updated_at',
  'archived_at',
  'deleted_at',
]

export function isCustomRecordType(recordType: string): boolean {
  return recordType.startsWith(CUSTOM_RECORD_TYPE_PREFIX)
}

export function customRecordType(key: string): string {
  return `${CUSTOM_RECORD_TYPE_PREFIX}${key}`
}

export function customObjectKey(recordType: string): string {
  return isCustomRecordType(recordType) ? recordType.slice(CUSTOM_RECORD_TYPE_PREFIX.length) : recordType
}

export function customObjectPath(key: string): string {
  return `/objects/${encodeURIComponent(key)}`
}

export function customRecordPath(key: string, id: string): string {
  return `${customObjectPath(key)}/${encodeURIComponent(id)}`
}

export function customRecordsApiPath(key: string): string {
  return `/v1/objects/${encodeURIComponent(key)}/records`
}

export function objectPermission(object: Pick<CustomObject, 'record_type'>, action: string): string {
  return `${object.record_type}.${action}`
}

export function registerCustomObjectResources(objects: CustomObject[]) {
  for (const object of objects) {
    registerRecordResource({
      recordType: object.record_type,
      updatePermission: objectPermission(object, 'update'),
      basePath: customRecordsApiPath(object.key),
      titleField: CUSTOM_RECORD_TITLE_FIELD,
      detailPath: (id) => customRecordPath(object.key, id),
    })
  }
}
