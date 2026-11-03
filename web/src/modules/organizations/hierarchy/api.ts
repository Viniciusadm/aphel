import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import { recordsKeys } from '@/features/records/api'

import type { CompanyGroupTotals, CompanyHierarchy } from './types'

const COMPANY_RECORD_TYPE = 'organizations.organization'

export const companyHierarchyKeys = {
  all: ['company-hierarchy'] as const,
  tree: (companyId: string) => [...companyHierarchyKeys.all, 'tree', companyId] as const,
  totals: (companyId: string) => [...companyHierarchyKeys.all, 'totals', companyId] as const,
}

const id = encodeURIComponent

export const companyHierarchyApi = {
  hierarchy: (companyId: string) => api.get<CompanyHierarchy>(`/v1/organizations/${id(companyId)}/hierarchy`),
  totals: (companyId: string) => api.get<CompanyGroupTotals>(`/v1/organizations/${id(companyId)}/group-totals`),
  setParent: (companyId: string, parentId: string | null) =>
    api.patch<unknown>(`/v1/organizations/${id(companyId)}`, { parent_company_id: parentId }),
}

export function useCompanyHierarchy(companyId: string) {
  return useQuery({
    queryKey: companyHierarchyKeys.tree(companyId),
    queryFn: () => companyHierarchyApi.hierarchy(companyId),
    enabled: companyId !== '',
  })
}

export function useCompanyGroupTotals(companyId: string) {
  return useQuery({
    queryKey: companyHierarchyKeys.totals(companyId),
    queryFn: () => companyHierarchyApi.totals(companyId),
    enabled: companyId !== '',
  })
}

export function useSetCompanyParent() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ companyId, parentId }: { companyId: string; parentId: string | null }) =>
      companyHierarchyApi.setParent(companyId, parentId),
    onSuccess: (_, { companyId }) => {
      void queryClient.invalidateQueries({ queryKey: companyHierarchyKeys.all })
      void queryClient.invalidateQueries({ queryKey: recordsKeys.detail(COMPANY_RECORD_TYPE, companyId) })
      void queryClient.invalidateQueries({ queryKey: recordsKeys.list(COMPANY_RECORD_TYPE) })
    },
  })
}
