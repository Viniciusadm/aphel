import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { api, type components } from '@/api'
import { toApiQuery, type AuditFilters } from './lib/filters'

export type AuditLog = components['schemas']['AuditLogResponse']
export type AuditActor = components['schemas']['AuditActorResponse']
export type AuditFieldChange = components['schemas']['AuditFieldChangeResponse']
export type AuditLogsPage = components['schemas']['Page_AuditLogResponse']

export const AUDIT_PER_PAGE = 25

export const auditKeys = {
  all: ['audit'] as const,
  list: (filters: AuditFilters, page: number) => [...auditKeys.all, 'list', filters, page] as const,
}

export const auditApi = {
  listAuditLogs: (filters: AuditFilters, page: number, perPage: number) =>
    api.get<AuditLogsPage>('/v1/audit-logs', { query: { ...toApiQuery(filters), page, per_page: perPage } }),
}

export function useAuditLogs(filters: AuditFilters, page: number) {
  return useQuery({
    queryKey: auditKeys.list(filters, page),
    queryFn: () => auditApi.listAuditLogs(filters, page, AUDIT_PER_PAGE),
    placeholderData: keepPreviousData,
  })
}
