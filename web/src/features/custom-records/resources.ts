import { registerRecordResource } from '@/features/records/resources'
import type { EntityDescriptor } from '@/sdk/entities'
import type { components } from '@/api/schema'

export type CustomObject = components['schemas']['CustomObjectResponse']

export const CUSTOM_RECORD_TYPE_PREFIX = 'custom.'
const objectKeys = new Map<string, string>()

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
  return objectKeys.has(recordType) || recordType.startsWith(CUSTOM_RECORD_TYPE_PREFIX)
}

export function customRecordType(key: string): string {
  return `${CUSTOM_RECORD_TYPE_PREFIX}${key}`
}

export function customObjectKey(recordType: string): string {
  return objectKeys.get(recordType) ?? (recordType.startsWith(CUSTOM_RECORD_TYPE_PREFIX) ? recordType.slice(CUSTOM_RECORD_TYPE_PREFIX.length) : recordType)
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

export function managedEntitiesApiPath(recordType: string): string {
  return `/v1/entities/${encodeURIComponent(recordType)}/records`
}

export function registerCustomObjectResources(objects: CustomObject[], entities: EntityDescriptor[] = []) {
  objectKeys.clear()
  const managed = new Map(
    entities.filter((entity) => entity.storage === 'managed').map((entity) => [entity.record_type, entity]),
  )
  for (const object of objects) {
    objectKeys.set(object.record_type, object.key)
    const entity = managed.get(object.record_type)
    registerRecordResource({
      recordType: object.record_type,
      traits: object.traits,
      createPermission: objectPermission(object, 'create'),
      updatePermission: objectPermission(object, 'update'),
      basePath: entity ? managedEntitiesApiPath(object.record_type) : customRecordsApiPath(object.key),
      titleField: entity?.title_field ?? CUSTOM_RECORD_TITLE_FIELD,
      detailPath: (id) => customRecordPath(object.key, id),
    })
  }
}
