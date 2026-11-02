import { keepPreviousData, useInfiniteQuery, useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
import type { BulkActionContext } from '@/features/records/table'

type Schemas = components['schemas']

export type BulkOperation = Schemas['BulkOperationResponse']
export type BulkItem = Schemas['BulkItemResponse']
export type BulkItemsPage = Schemas['Page_BulkItemResponse']
export type BulkOperationsPage = Schemas['Page_BulkOperationResponse']
export type BulkAction = Schemas['BulkActionRequest'] | { action: 'enroll_in_sequence'; sequence_id: string }
export type BulkSelection = Schemas['BulkSelectionRequest']
export type BulkExportFormat = Schemas['ExportFormatDto']
export type ExportJob = Schemas['ExportJobResponse']

export const BULK_RECORD_TYPES = ['contacts.contact', 'companies.company', 'deals.deal', 'products.product'] as const
export type BulkRecordType = (typeof BULK_RECORD_TYPES)[number]
export const POLL_INTERVAL_MS = 2000
export const REPORT_PAGE_SIZE = 50
export const HISTORY_PAGE_SIZE = 25
export const MAX_EXPORT_IDS = 100

const PERMISSION_PREFIX: Record<BulkRecordType, string> = {
  'contacts.contact': 'contacts',
  'companies.company': 'companies',
  'deals.deal': 'deals',
  'products.product': 'products',
}

export const BULK_SYSTEM_FIELDS: Record<BulkRecordType, readonly string[]> = {
  'contacts.contact': ['first_name', 'last_name', 'email', 'phone', 'mobile', 'job_title', 'company_id', 'owner_id', 'source', 'notes'],
  'companies.company': ['name', 'domain', 'website', 'industry', 'size_range', 'phone', 'email', 'owner_id', 'description', 'source'],
  'deals.deal': ['title', 'amount', 'currency', 'probability', 'expected_close_date', 'contact_id', 'company_id', 'owner_id', 'source'],
  'products.product': ['price', 'unit', 'recurrence', 'term_months', 'description'],
}

export function isBulkRecordType(recordType: string): recordType is BulkRecordType {
  return (BULK_RECORD_TYPES as readonly string[]).includes(recordType)
}

export function bulkPermission(recordType: BulkRecordType, action: 'update' | 'archive' | 'delete' | 'export'): string {
  return `${PERMISSION_PREFIX[recordType]}.${action}`
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
  history: (page: number) => [...bulkKeys.all, 'history', page] as const,
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
  list: (page: number) =>
    api.get<BulkOperationsPage>('/v1/bulk-operations', { query: { page, per_page: HISTORY_PAGE_SIZE } }),
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

export function useBulkHistory(page: number) {
  return useQuery({
    queryKey: bulkKeys.history(page),
    queryFn: () => bulkApi.list(page),
    placeholderData: keepPreviousData,
    refetchInterval: (query) =>
      query.state.data?.data.some((operation) => !isFinished(operation.status)) ? POLL_INTERVAL_MS : false,
  })
}
