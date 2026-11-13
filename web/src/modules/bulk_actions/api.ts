import { keepPreviousData, useInfiniteQuery, useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
import { type BulkActionContext } from '@/sdk'
type Schemas = components['schemas']

export type BulkOperation = Schemas['BulkOperationResponse']
export type BulkItem = Schemas['BulkItemResponse']
export type BulkItemsPage = Schemas['Page_BulkItemResponse']
export type BulkOperationsPage = Schemas['Page_BulkOperationResponse']
export type BulkAction = Schemas['BulkActionRequest'] | { action: 'enroll_in_sequence'; sequence_id: string }
export type BulkSelection = Schemas['BulkSelectionRequest']
export type BulkExportFormat = Schemas['ExportFormatDto']
export type ExportJob = Schemas['ExportJobResponse']

export type BulkRecordType = string
export const POLL_INTERVAL_MS = 2000
export const REPORT_PAGE_SIZE = 50
export const HISTORY_PAGE_SIZE = 25
export const MAX_EXPORT_IDS = 100

export function bulkPermission(
  entity: { permissions: { read: string } & Partial<Record<'update' | 'archive' | 'delete' | 'export', string | null>> },
  action: 'update' | 'archive' | 'delete' | 'export',
): string {
  return entity.permissions[action] ?? `${entity.permissions.read.replace(/\.read$/, '')}.${action}`
}

export function isFinished(status: string | undefined): boolean {
  return status === 'completed' || status === 'failed'
}

export function selectionOf(context: BulkActionContext): BulkSelection {
  if (context.allMatching) {
    return {
      query: {
        filter: context.query.filter ?? undefined,
        search: context.query.search.trim() || undefined,
        archived: context.query.archived,
      },
    }
  }
  return { ids: context.selectedIds }
}

export const bulkKeys = {
  all: ['bulk-operations'] as const,
  detail: (id: string) => [...bulkKeys.all, 'detail', id] as const,
  items: (id: string) => [...bulkKeys.all, 'items', id] as const,
  history: (page: number, status: string) => [...bulkKeys.all, 'history', page, status] as const,
}

function operationPath(id: string, suffix = '') {
  return `/v1/bulk-operations/${encodeURIComponent(id)}${suffix}`
}

export const bulkApi = {
  start: (recordType: string, action: BulkAction, selection: BulkSelection) =>
    api.post<BulkOperation>('/v1/bulk-operations', { record_type: recordType, action, selection }),
  export: (recordType: string, format: BulkExportFormat, selection: BulkSelection) =>
    api.post<ExportJob>('/v1/bulk-operations/export', { record_type: recordType, format, selection }),
  get: (id: string) => api.get<BulkOperation>(operationPath(id)),
  items: (id: string, page: number) =>
    api.get<BulkItemsPage>(operationPath(id, '/items'), { query: { page, per_page: REPORT_PAGE_SIZE } }),
  list: (page: number, status: string) =>
    api.get<BulkOperationsPage>('/v1/bulk-operations', {
      query: { page, per_page: HISTORY_PAGE_SIZE, status: status || undefined },
    }),
}

export function useBulkOperation(initial: BulkOperation | null) {
  const id = initial?.id ?? ''
  return useQuery({
    queryKey: bulkKeys.detail(id),
    queryFn: () => bulkApi.get(id),
    enabled: Boolean(initial) && !isFinished(initial?.status),
    initialData: initial ?? undefined,
    refetchInterval: (query) => (isFinished(query.state.data?.status) ? false : POLL_INTERVAL_MS),
  })
}

export function useBulkItems(operation: BulkOperation | undefined) {
  const id = operation?.id ?? ''
  const hasIssues = Boolean(operation && operation.skipped + operation.failed > 0)
  return useInfiniteQuery({
    queryKey: bulkKeys.items(id),
    queryFn: ({ pageParam }) => bulkApi.items(id, pageParam),
    initialPageParam: 1,
    getNextPageParam: (last) =>
      last.meta.page * last.meta.per_page < last.meta.total ? last.meta.page + 1 : undefined,
    enabled: hasIssues && isFinished(operation?.status),
  })
}

export function useBulkHistory(page: number, status: string) {
  return useQuery({
    queryKey: bulkKeys.history(page, status),
    queryFn: () => bulkApi.list(page, status),
    placeholderData: keepPreviousData,
    refetchInterval: (query) =>
      query.state.data?.data.some((operation) => !isFinished(operation.status)) ? POLL_INTERVAL_MS : false,
  })
}
