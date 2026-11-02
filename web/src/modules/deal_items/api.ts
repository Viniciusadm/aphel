import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
import { recordsKeys } from '@/features/records/api'


type Schemas = components['schemas']

export type DealItem = Schemas['DealItemResponse']
export type DealItems = Schemas['DealItemsResponse']
export type DealItemRequest = Schemas['DealItemRequest']
export type DiscountType = Schemas['DiscountType']

export const DISCOUNT_TYPES: DiscountType[] = ['none', 'percent', 'amount']

const DEAL_RECORD_TYPE = 'deals.deal'

export const dealItemsKeys = {
  all: ['deal-items'] as const,
  deal: (dealId: string) => [...dealItemsKeys.all, dealId] as const,
}

function itemsPath(dealId: string, suffix = '') {
  return `/v1/deals/${encodeURIComponent(dealId)}/items${suffix}`
}

export const dealItemsApi = {
  list: (dealId: string) => api.get<DealItems>(itemsPath(dealId)),
  create: (dealId: string, body: DealItemRequest) => api.post<DealItem>(itemsPath(dealId), body),
  update: (dealId: string, itemId: string, body: DealItemRequest, version?: string) =>
    api.patch<DealItem>(itemsPath(dealId, `/${encodeURIComponent(itemId)}`), body, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
  remove: (dealId: string, itemId: string) => api.delete<void>(itemsPath(dealId, `/${encodeURIComponent(itemId)}`)),
  reorder: (dealId: string, itemIds: string[]) =>
    api.put<DealItems>(itemsPath(dealId, '/order'), { item_ids: itemIds }),
  applyTotal: (dealId: string) => api.post<DealItems>(itemsPath(dealId, '/apply-total')),
}

export function useDealItems(dealId: string, enabled = true) {
  return useQuery({
    queryKey: dealItemsKeys.deal(dealId),
    queryFn: () => dealItemsApi.list(dealId),
    enabled: enabled && dealId !== '',
    retry: false,
  })
}

export function useDealItemMutations(dealId: string) {
  const queryClient = useQueryClient()
  const refresh = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: dealItemsKeys.deal(dealId) }),
      queryClient.invalidateQueries({ queryKey: recordsKeys.detail(DEAL_RECORD_TYPE, dealId) }),
      queryClient.invalidateQueries({ queryKey: recordsKeys.list(DEAL_RECORD_TYPE) }),
    ])
  }
  const onSummary = async (summary: DealItems) => {
    queryClient.setQueryData(dealItemsKeys.deal(dealId), summary)
    await refresh()
  }
  return {
    create: useMutation({
      mutationFn: (body: DealItemRequest) => dealItemsApi.create(dealId, body),
      onSettled: refresh,
    }),
    update: useMutation({
      mutationFn: ({ item, body }: { item: DealItem; body: DealItemRequest }) =>
        dealItemsApi.update(dealId, item.id, body, item.updated_at),
      onSettled: refresh,
    }),
    remove: useMutation({
      mutationFn: (item: DealItem) => dealItemsApi.remove(dealId, item.id),
      onSettled: refresh,
    }),
    reorder: useMutation({
      mutationFn: (itemIds: string[]) => dealItemsApi.reorder(dealId, itemIds),
      onSuccess: onSummary,
      onError: refresh,
    }),
    applyTotal: useMutation({
      mutationFn: () => dealItemsApi.applyTotal(dealId),
      onSuccess: onSummary,
      onError: refresh,
    }),
  }
}

export function toDecimalInput(text: string): string {
  const value = text.trim().replace(/\s/g, '')
  if (value.includes(',')) return value.replace(/\./g, '').replace(',', '.')
  return value
}
