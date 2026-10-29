import { useQuery } from '@tanstack/react-query'
import { useCallback, useRef } from 'react'
import { api } from '@/api'
import type { components } from '@/api/schema'
import type {
  CreateViewRequest,
  FieldSchema,
  RecordItem,
  SavedView,
  SavedViewsPage,
  UpdateViewRequest,
  UserReference,
} from './types'

export const VIEWS_PER_PAGE = 100
export const USERS_PER_PAGE = 100
const MAX_USER_PAGES = 20
export const IDEMPOTENCY_HEADER = 'Idempotency-Key'

export function newIdempotencyKey(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID()
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}-${Math.random().toString(36).slice(2)}`
}

export function useIdempotencyKey() {
  const last = useRef<{ body: string; key: string } | null>(null)
  return useCallback((body: unknown) => {
    const serialized = JSON.stringify(body)
    if (last.current?.body !== serialized) last.current = { body: serialized, key: newIdempotencyKey() }
    return last.current.key
  }, [])
}

export const recordsKeys = {
  all: ['records'] as const,
  schema: (recordType: string) => [...recordsKeys.all, recordType, 'schema'] as const,
  views: (recordType: string) => [...recordsKeys.all, recordType, 'views'] as const,
  list: (recordType: string) => [...recordsKeys.all, recordType, 'list'] as const,
  detail: (recordType: string, id: string) => [...recordsKeys.all, recordType, 'detail', id] as const,
  lookup: (recordType: string) => [...recordsKeys.all, recordType, 'lookup'] as const,
  users: () => [...recordsKeys.all, 'users'] as const,
}

function fieldsPath(recordType: string) {
  return `/v1/fields/${encodeURIComponent(recordType)}/schema`
}

function viewsPath(recordType: string, suffix = '') {
  return `/v1/views/${encodeURIComponent(recordType)}${suffix}`
}

type DirectoryPage = components['schemas']['Page_DirectoryUserResponse']

export const recordsApi = {
  schema: (recordType: string) => api.get<FieldSchema>(fieldsPath(recordType)),
  createRecord: (path: string, payload: unknown, idempotencyKey: string) =>
    api.post<RecordItem>(path, payload, { headers: { [IDEMPOTENCY_HEADER]: idempotencyKey } }),
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
  listUsers: async (): Promise<UserReference[]> => {
    const users: UserReference[] = []
    for (let page = 1; page <= MAX_USER_PAGES; page += 1) {
      const result = await api.get<DirectoryPage>('/v1/users', { query: { page, per_page: USERS_PER_PAGE } })
      users.push(...result.data.map((user) => ({ id: user.id, display_name: user.display_name })))
      if (page * USERS_PER_PAGE >= result.meta.total) break
    }
    return users
  },
}

export async function listAllViews(recordType: string): Promise<SavedView[]> {
  const first = await recordsApi.listViews(recordType, 1)
  const items = [...first.data]
  const perPage = first.meta.per_page > 0 ? first.meta.per_page : VIEWS_PER_PAGE
  const pages = Math.ceil(first.meta.total / perPage)
  for (let page = 2; page <= pages; page += 1) {
    const next = await recordsApi.listViews(recordType, page)
    items.push(...next.data)
  }
  return items.sort((left, right) => left.position - right.position)
}

export function useFieldSchema(recordType: string, enabled = true) {
  return useQuery({
    queryKey: recordsKeys.schema(recordType),
    queryFn: () => recordsApi.schema(recordType),
    enabled: enabled && recordType !== '',
    staleTime: 60_000,
  })
}

export function useSavedViews(recordType: string) {
  return useQuery({
    queryKey: recordsKeys.views(recordType),
    queryFn: () => listAllViews(recordType),
  })
}

export function useOrganizationUsers() {
  return useQuery({
    queryKey: recordsKeys.users(),
    queryFn: () => recordsApi.listUsers().catch(() => [] as UserReference[]),
    staleTime: 5 * 60_000,
  })
}
