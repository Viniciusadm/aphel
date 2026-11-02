import type { RecordItem } from './types'

export interface RecordResource {
  recordType: string
  basePath: string
  titleField: string
  createPermission?: string
  updatePermission?: string
  collectionQueryKeys?: readonly (readonly unknown[])[]
  summaryFields?: string[]
  subtitleField?: string
  detailPath?: (id: string) => string
}

const resources = new Map<string, RecordResource>()

export function registerRecordResource(resource: RecordResource): RecordResource {
  resources.set(resource.recordType, resource)
  return resource
}

export function getRecordResource(recordType: string): RecordResource | undefined {
  return resources.get(recordType)
}

export function requireRecordResource(recordType: string): RecordResource {
  const resource = resources.get(recordType)
  if (!resource) throw new Error(`record resource not registered: ${recordType}`)
  return resource
}

export function recordPath(resource: RecordResource, id?: string): string {
  return id ? `${resource.basePath}/${encodeURIComponent(id)}` : resource.basePath
}

export function recordTitle(resource: RecordResource, record: RecordItem): string {
  const value = record[resource.titleField] ?? record.custom_fields?.[resource.titleField]
  return typeof value === 'string' && value.trim() ? value : record.id
}
