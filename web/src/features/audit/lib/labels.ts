import type { ParseKeys } from 'i18next'

type AuditKey = ParseKeys<'audit'>

const recordTypeLabels: Record<string, AuditKey> = {
  contact: 'recordTypes.contact',
  company: 'recordTypes.company',
  deal: 'recordTypes.deal',
  activity: 'recordTypes.activity',
  note: 'recordTypes.note',
  file: 'recordTypes.file',
  member: 'recordTypes.member',
  role: 'recordTypes.role',
  team: 'recordTypes.team',
  organization: 'recordTypes.organization',
}

export const KNOWN_RECORD_TYPES = Object.keys(recordTypeLabels)

export function recordTypeLabel(recordType: string): AuditKey | undefined {
  return recordTypeLabels[recordType]
}
