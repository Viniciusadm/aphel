import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { API_BASE_URL, api } from '@/api'
import type { OrgExport, OrgExportPage, OrgExportRequest } from './types'

export const EXPORTS_PER_PAGE = 10
const POLL_INTERVAL_MS = 4000

const BASE_PATH = '/v1/organization-exports'

export const orgExportKeys = {
  all: ['organization-exports'] as const,
  page: (page: number) => [...orgExportKeys.all, 'page', page] as const,
}

export function downloadUrl(id: string): string {
  return `${API_BASE_URL}${BASE_PATH}/${encodeURIComponent(id)}/download`
}

export function isPending(status: OrgExport['status']): boolean {
  return status === 'queued' || status === 'running'
}

export const orgExportApi = {
  list: (page: number) =>
    api.get<OrgExportPage>(BASE_PATH, { query: { page, per_page: EXPORTS_PER_PAGE } }),
  request: (body: OrgExportRequest) => api.post<OrgExport>(BASE_PATH, body),
  remove: (id: string) => api.delete<void>(`${BASE_PATH}/${encodeURIComponent(id)}`),
}

export function useOrgExports(page: number) {
  return useQuery({
    queryKey: orgExportKeys.page(page),
    queryFn: () => orgExportApi.list(page),
    placeholderData: keepPreviousData,
    refetchInterval: (query) =>
      query.state.data?.data.some((item) => isPending(item.status)) ? POLL_INTERVAL_MS : false,
  })
}

export function useRequestOrgExport() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: orgExportApi.request,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: orgExportKeys.all }),
  })
}

export function useDeleteOrgExport() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: orgExportApi.remove,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: orgExportKeys.all }),
  })
}
