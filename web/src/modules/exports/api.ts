import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { isCustomRecordType } from '@/features/custom-records/resources'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type ExportJob = Schemas['ExportJobResponse']
export type ExportRequest = Schemas['ExportRequest']
export type ExportFormat = Schemas['ExportFormatDto']
export type ExportJobsPage = Schemas['Page_ExportJobResponse']

export const EXPORTABLE_RECORD_TYPES = ['contacts.contact', 'companies.company', 'deals.deal', 'activities.activity', 'products.product'] as const
const POLL_INTERVAL_MS = 2000
export const HISTORY_PAGE_SIZE = 25

export type ExportableRecordType = (typeof EXPORTABLE_RECORD_TYPES)[number]

const PERMISSION_PREFIX: Record<ExportableRecordType, string> = {
  'contacts.contact': 'contacts',
  'companies.company': 'companies',
  'deals.deal': 'deals',
  'activities.activity': 'activities',
  'products.product': 'products',
}

export function isExportable(recordType: string): recordType is ExportableRecordType {
  return (EXPORTABLE_RECORD_TYPES as readonly string[]).includes(recordType)
}

export function exportPermission(recordType: string): string | null {
  if (isCustomRecordType(recordType)) return `${recordType}.export`
  return isExportable(recordType) ? `${PERMISSION_PREFIX[recordType]}.export` : null
}

function isRunning(status: string | undefined): boolean {
  return status === 'queued' || status === 'running'
}

export const exportsKeys = {
  all: ['exports'] as const,
  detail: (id: string) => [...exportsKeys.all, 'detail', id] as const,
  history: (page: number) => [...exportsKeys.all, 'history', page] as const,
}

export const exportsApi = {
  request: (request: ExportRequest) => api.post<ExportJob>('/v1/exports', request),
  get: (id: string) => api.get<ExportJob>(`/v1/exports/${encodeURIComponent(id)}`),
  list: (page: number) =>
    api.get<ExportJobsPage>('/v1/exports', { query: { page, per_page: HISTORY_PAGE_SIZE } }),
}

export function useRequestExport() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: exportsApi.request,
    onSuccess: (job) => {
      queryClient.setQueryData(exportsKeys.detail(job.id), job)
      void queryClient.invalidateQueries({ queryKey: [...exportsKeys.all, 'history'] })
    },
  })
}

export function useExportHistory(page: number) {
  return useQuery({
    queryKey: exportsKeys.history(page),
    queryFn: () => exportsApi.list(page),
    placeholderData: keepPreviousData,
    refetchInterval: (query) =>
      query.state.data?.data.some((job) => isRunning(job.status)) ? POLL_INTERVAL_MS : false,
  })
}

export function useExportJob(id: string | null) {
  return useQuery({
    queryKey: exportsKeys.detail(id ?? ''),
    queryFn: () => exportsApi.get(id ?? ''),
    enabled: Boolean(id),
    refetchInterval: (query) => (isRunning(query.state.data?.status) ? POLL_INTERVAL_MS : false),
  })
}
