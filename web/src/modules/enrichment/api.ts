import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'

export const ENRICHMENT_KEY = 'enrichment'
export const USE_PERMISSION = 'enrichment.use'

export interface EnrichmentFieldSource {
  field_key: string
  data_key: string
  source: string
  cnpj: string
  value_snapshot?: string | null
  fetched_at: string
  applied_by?: string | null
  applied_at: string
}

export interface EnrichmentCompanyOverview {
  company_id: string
  cnpj_field?: string | null
  cnpj?: string | null
  source_mode: string
  automatic: boolean
  sources: EnrichmentFieldSource[]
}

export interface EnrichmentProposalItem {
  data_key: string
  label: string
  field_key?: string | null
  field_label?: string | null
  current: unknown
  proposed: string
  changed: boolean
}

export interface EnrichmentLookup {
  id: string
  company_id: string
  cnpj: string
  source: string
  fetched_at: string
  items: EnrichmentProposalItem[]
}

export interface EnrichmentApplied {
  company_id: string
  source: string
  cnpj: string
  fields: string[]
}

const BASE = '/v1/enrichment'

export const enrichmentKeys = {
  all: [ENRICHMENT_KEY] as const,
  company: (companyId: string) => [...enrichmentKeys.all, 'company', companyId] as const,
}

export const enrichmentApi = {
  company: (companyId: string) =>
    api.get<EnrichmentCompanyOverview>(`${BASE}/companies/${encodeURIComponent(companyId)}`),
  lookup: (companyId: string, cnpj?: string) =>
    api.post<EnrichmentLookup>(`${BASE}/lookups`, { company_id: companyId, cnpj: cnpj || undefined }),
  apply: (lookupId: string, items: string[]) =>
    api.post<EnrichmentApplied>(`${BASE}/lookups/${encodeURIComponent(lookupId)}/apply`, { items }),
}

export function useEnrichmentCompany(companyId: string) {
  return useQuery({ queryKey: enrichmentKeys.company(companyId), queryFn: () => enrichmentApi.company(companyId) })
}

export function useInvalidateEnrichment() {
  const queryClient = useQueryClient()
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: enrichmentKeys.all }),
      queryClient.invalidateQueries({ queryKey: ['records'] }),
    ])
}
