import { recordResources } from '@/features/records/resources'

export type CheckInRecordType = string

export interface RecordRouteMatch {
  recordType: CheckInRecordType
  recordId: string
}

export function matchRecordRoute(pathname: string): RecordRouteMatch | null {
  const match = /\/([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\/?$/i.exec(pathname)
  if (!match) return null
  for (const resource of recordResources()) {
    if (resource.checkIn && resource.detailPath?.(match[1]) === pathname.replace(/\/$/, '')) {
      return { recordType: resource.recordType, recordId: match[1] }
    }
  }
  return null
}

export function checkInRecordTypes(): string[] {
  return recordResources().filter((resource) => resource.checkIn).map((resource) => resource.recordType)
}
