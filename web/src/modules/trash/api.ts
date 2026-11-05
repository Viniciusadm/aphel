import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/modules/trash/api-types'
import { recordsKeys } from '@/sdk'
type Schemas = components['schemas']

export type TrashItem = Schemas['TrashItemResponse']
export type TrashPage = Schemas['Page_TrashItemResponse']
export type TrashSettings = Schemas['TrashSettingsBody']

export type TrashRecordType = string

export interface TrashTypeInfo {
  record_type: string
  label?: string | null
  custom: boolean
}

export const TRASH_PER_PAGE = 25
export const MIN_RETENTION_DAYS = 1
export const MAX_RETENTION_DAYS = 365

export interface TrashFilters {
  recordType?: TrashRecordType
  search?: string
}

export const trashKeys = {
  all: ['trash'] as const,
  list: (filters: TrashFilters, page: number) => [...trashKeys.all, 'list', filters, page] as const,
  settings: () => [...trashKeys.all, 'settings'] as const,
  types: () => [...trashKeys.all, 'types'] as const,
}

function itemPath(item: Pick<TrashItem, 'record_type' | 'id'>) {
  return `/v1/trash/${encodeURIComponent(item.record_type)}/${encodeURIComponent(item.id)}`
}

export const trashApi = {
  types: () => api.get<TrashTypeInfo[]>('/v1/trash/types'),
  list: (filters: TrashFilters, page: number) =>
    api.get<TrashPage>('/v1/trash', {
      query: {
        record_type: filters.recordType,
        search: filters.search || undefined,
        page,
        per_page: TRASH_PER_PAGE,
      },
    }),
  restore: (item: TrashItem) => api.post<TrashItem>(`${itemPath(item)}/restore`),
  purge: (item: TrashItem) => api.delete<void>(itemPath(item)),
  settings: () => api.get<TrashSettings>('/v1/trash/settings'),
  updateSettings: (settings: TrashSettings) => api.put<TrashSettings>('/v1/trash/settings', settings),
}

export function useTrash(filters: TrashFilters, page: number) {
  return useQuery({
    queryKey: trashKeys.list(filters, page),
    queryFn: () => trashApi.list(filters, page),
    placeholderData: keepPreviousData,
  })
}

export function useTrashMutations() {
  const queryClient = useQueryClient()
  const refresh = (item: TrashItem) => {
    void queryClient.invalidateQueries({ queryKey: trashKeys.all })
    void queryClient.invalidateQueries({ queryKey: [...recordsKeys.all, item.record_type] })
  }
  const restore = useMutation({ mutationFn: trashApi.restore, onSuccess: (_, item) => refresh(item) })
  const purge = useMutation({ mutationFn: trashApi.purge, onSuccess: (_, item) => refresh(item) })
  return { restore, purge }
}

export function useTrashTypes() {
  return useQuery({ queryKey: trashKeys.types(), queryFn: trashApi.types, staleTime: 60_000 })
}

export function useTrashSettings() {
  return useQuery({ queryKey: trashKeys.settings(), queryFn: trashApi.settings })
}

export function useUpdateTrashSettings() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: trashApi.updateSettings,
    onSuccess: (settings) => {
      queryClient.setQueryData(trashKeys.settings(), settings)
      void queryClient.invalidateQueries({ queryKey: trashKeys.all })
    },
  })
}
