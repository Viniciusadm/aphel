import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type {
  DiscountLimit,
  DiscountLimitRequest,
  PriceList,
  PriceListItem,
  PriceListItemRequest,
  PriceListRequest,
  PriceListsPage,
} from './types'

export const PRICE_LISTS_PER_PAGE = 25
export const PRICE_LISTS_PATH = '/pricing'

const id = encodeURIComponent

export function priceListPath(priceListId: string): string {
  return `${PRICE_LISTS_PATH}/${id(priceListId)}`
}

export interface PriceListFilter {
  page: number
  search?: string
  currency?: string
  active?: boolean
}

export const priceListsKeys = {
  all: ['pricing'] as const,
  lists: () => [...priceListsKeys.all, 'list'] as const,
  list: (filter: PriceListFilter) => [...priceListsKeys.lists(), filter] as const,
  detail: (priceListId: string) => [...priceListsKeys.all, 'detail', priceListId] as const,
  items: (priceListId: string) => [...priceListsKeys.all, 'items', priceListId] as const,
  discountLimits: () => [...priceListsKeys.all, 'discount-limits'] as const,
}

function ifMatch(version?: string | null): HeadersInit | undefined {
  return version ? { 'If-Match': version } : undefined
}

export const priceListsApi = {
  list: (filter: PriceListFilter) =>
    api.get<PriceListsPage>('/v1/pricing', {
      query: {
        page: filter.page,
        per_page: PRICE_LISTS_PER_PAGE,
        search: filter.search?.trim() || undefined,
        currency: filter.currency || undefined,
        active: filter.active,
      },
    }),
  get: (priceListId: string) => api.get<PriceList>(`/v1/pricing/${id(priceListId)}`),
  create: (body: PriceListRequest) => api.post<PriceList>('/v1/pricing', body),
  update: (priceListId: string, body: PriceListRequest, version?: string | null) =>
    api.patch<PriceList>(`/v1/pricing/${id(priceListId)}`, body, { headers: ifMatch(version) }),
  remove: (priceListId: string) => api.delete<void>(`/v1/pricing/${id(priceListId)}`),
  items: (priceListId: string) => api.get<PriceListItem[]>(`/v1/pricing/${id(priceListId)}/items`),
  replaceItems: (priceListId: string, items: PriceListItemRequest[], version?: string | null) =>
    api.put<PriceListItem[]>(`/v1/pricing/${id(priceListId)}/items`, { items }, { headers: ifMatch(version) }),
  discountLimits: () => api.get<DiscountLimit[]>('/v1/pricing/discount-limits'),
  setDiscountLimits: (limits: DiscountLimitRequest[]) => api.put<DiscountLimit[]>('/v1/pricing/discount-limits', { limits }),
}

export function usePriceLists(filter: PriceListFilter) {
  return useQuery({
    queryKey: priceListsKeys.list(filter),
    queryFn: () => priceListsApi.list(filter),
    placeholderData: keepPreviousData,
  })
}

export function useActivePriceLists(enabled = true) {
  return useQuery({
    queryKey: [...priceListsKeys.lists(), 'active'],
    queryFn: async () => (await priceListsApi.list({ page: 1, active: true })).data,
    enabled,
    staleTime: 60_000,
  })
}

export function usePriceList(priceListId: string) {
  return useQuery({
    queryKey: priceListsKeys.detail(priceListId),
    queryFn: () => priceListsApi.get(priceListId),
    enabled: Boolean(priceListId),
  })
}

export function usePriceListItems(priceListId: string) {
  return useQuery({
    queryKey: priceListsKeys.items(priceListId),
    queryFn: () => priceListsApi.items(priceListId),
    enabled: Boolean(priceListId),
  })
}

export function useDiscountLimits(enabled = true) {
  return useQuery({
    queryKey: priceListsKeys.discountLimits(),
    queryFn: priceListsApi.discountLimits,
    enabled,
  })
}

export function useInvalidatePriceLists() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: priceListsKeys.all })
}
