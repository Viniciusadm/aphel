import { useIsModuleActive } from '@/kernel/modules/registry'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type SearchResult = Schemas['SearchResponse']
export type SearchGroup = Schemas['SearchGroupResponse']
export type SearchHit = Schemas['SearchHitResponse']

export const SEARCH_MIN_LENGTH = 2

export interface SearchParams {
  q: string
  types?: string[]
  limit?: number
}

export const searchKeys = {
  all: ['search'] as const,
  query: (params: SearchParams) => [...searchKeys.all, params] as const,
}

export const searchApi = {
  search: (params: SearchParams, signal?: AbortSignal) =>
    api.get<SearchResult>('/v1/search', {
      query: { q: params.q, types: params.types?.join(','), limit: params.limit },
      signal,
    }),
}

export function useGlobalSearch(params: SearchParams) {
  const q = params.q.trim()
  const active = useIsModuleActive('search')
  return useQuery({
    queryKey: searchKeys.query({ ...params, q }),
    queryFn: ({ signal }) => searchApi.search({ ...params, q }, signal),
    enabled: active && q.length >= SEARCH_MIN_LENGTH,
    placeholderData: keepPreviousData,
    staleTime: 15_000,
  })
}
