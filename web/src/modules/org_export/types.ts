import type { components } from '@/modules/org_export/api-types'

type Schemas = components['schemas']

export type OrgExportStatus = 'queued' | 'running' | 'completed' | 'failed'

const ORG_EXPORT_STATUSES: readonly OrgExportStatus[] = ['queued', 'running', 'completed', 'failed']

export function asOrgExportStatus(value: string): OrgExportStatus {
  return ORG_EXPORT_STATUSES.find((status) => status === value) ?? 'failed'
}

export type OrgExportSummary = Schemas['OrgExportSummaryResponse']
export type OrgExport = Schemas['OrgExportResponse']
export type OrgExportPage = Schemas['Page_OrgExportResponse']
export type OrgExportRequest = Schemas['OrgExportRequest']
