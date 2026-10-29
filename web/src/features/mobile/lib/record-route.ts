export const CHECK_IN_RECORD_TYPES = ['contact', 'company', 'deal'] as const

export type CheckInRecordType = (typeof CHECK_IN_RECORD_TYPES)[number]

export interface RecordRouteMatch {
  recordType: CheckInRecordType
  recordId: string
}

const SEGMENT_RECORD_TYPES: Record<string, CheckInRecordType> = {
  contacts: 'contact',
  companies: 'company',
  deals: 'deal',
}

const RECORD_ROUTE = /^\/(contacts|companies|deals)\/([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\/?$/i

export function matchRecordRoute(pathname: string): RecordRouteMatch | null {
  const match = RECORD_ROUTE.exec(pathname)
  if (!match) return null
  const recordType = SEGMENT_RECORD_TYPES[match[1].toLowerCase()]
  return recordType ? { recordType, recordId: match[2] } : null
}
