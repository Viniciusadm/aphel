import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import { useIsModuleActive } from '@/kernel/modules/registry'
import { IDEMPOTENCY_HEADER, newIdempotencyKey, recordsKeys } from '@/features/records/api'

import type { CreateViewRequest, SavedView, SavedViewsPage, UpdateViewRequest } from '@/features/records/types'
export const VIEWS_PER_PAGE = 100
function viewsPath(recordType: string, suffix = '') {
  return `/v1/views/${encodeURIComponent(recordType)}${suffix}`
}

export const savedViewsApi = {
  listViews: (recordType: string, page = 1) =>
    api.get<SavedViewsPage>(viewsPath(recordType), { query: { page, per_page: VIEWS_PER_PAGE } }),
  createView: (recordType: string, request: CreateViewRequest, idempotencyKey = newIdempotencyKey()) =>
    api.post<SavedView>(viewsPath(recordType), request, { headers: { [IDEMPOTENCY_HEADER]: idempotencyKey } }),
  updateView: (recordType: string, viewId: string, request: UpdateViewRequest, version?: string | null) =>
    api.patch<SavedView>(viewsPath(recordType, `/${viewId}`), request, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
  deleteView: (recordType: string, viewId: string) => api.delete<void>(viewsPath(recordType, `/${viewId}`)),
  setUserDefaultView: (recordType: string, viewId: string) =>
    api.put<SavedView>(viewsPath(recordType, `/${viewId}/default`)),
  clearUserDefaultView: (recordType: string, viewId: string) =>
    api.delete<void>(viewsPath(recordType, `/${viewId}/default`)),
}
export async function listAllViews(recordType: string): Promise<SavedView[]> {
  const first = await savedViewsApi.listViews(recordType, 1)
  const items = [...first.data]
  const perPage = first.meta.per_page > 0 ? first.meta.per_page : VIEWS_PER_PAGE
  const pages = Math.ceil(first.meta.total / perPage)
  for (let page = 2; page <= pages; page += 1) {
    const next = await savedViewsApi.listViews(recordType, page)
    items.push(...next.data)
  }
  return items.sort((left, right) => left.position - right.position)
}

export function useSavedViews(recordType: string) {
  const active = useIsModuleActive('saved_views')
  const result = useQuery({
    enabled: active,
    queryKey: recordsKeys.views(recordType),
    queryFn: () => listAllViews(recordType),
  })
  return { ...result, data: active ? result.data : undefined, isPending: active && result.isPending }
}
