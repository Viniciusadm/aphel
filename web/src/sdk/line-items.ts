import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
import { recordsKeys } from '@/features/records/api'


type Schemas = components['schemas']

export type LineItem = Schemas['LineItemResponse']
export type LineItems = Schemas['LineItemsResponse']
export type LineItemRequest = Schemas['LineItemRequest']
export type DiscountType = Schemas['DiscountType']

export const DISCOUNT_TYPES: DiscountType[] = ['none', 'percent', 'amount']



export const lineItemsKeys = {
  all: ['line-items'] as const,
  record: (recordType: string, recordId: string) => [...lineItemsKeys.all, recordType, recordId] as const,
}

function itemsPath(recordType: string, recordId: string, suffix = '') {
  return `/v1/line-items/${encodeURIComponent(recordType)}/${encodeURIComponent(recordId)}/items${suffix}`
}

export const lineItemsApi = {
  list: (recordType: string, recordId: string) => api.get<LineItems>(itemsPath(recordType, recordId)),
  create: (recordType: string, recordId: string, body: LineItemRequest) => api.post<LineItem>(itemsPath(recordType, recordId), body),
  update: (recordType: string, recordId: string, itemId: string, body: LineItemRequest, version?: string) =>
    api.patch<LineItem>(itemsPath(recordType, recordId, `/${encodeURIComponent(itemId)}`), body, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
  remove: (recordType: string, recordId: string, itemId: string) => api.delete<void>(itemsPath(recordType, recordId, `/${encodeURIComponent(itemId)}`)),
  reorder: (recordType: string, recordId: string, itemIds: string[]) =>
    api.put<LineItems>(itemsPath(recordType, recordId, '/order'), { item_ids: itemIds }),
  applyTotal: (recordType: string, recordId: string) => api.post<LineItems>(itemsPath(recordType, recordId, '/apply-total')),
}

export function useLineItems(recordType: string, recordId: string, enabled = true) {
  return useQuery({
    queryKey: lineItemsKeys.record(recordType, recordId),
    queryFn: () => lineItemsApi.list(recordType, recordId),
    enabled: enabled && recordId !== '',
    retry: false,
  })
}

export function useLineItemMutations(recordType: string, recordId: string) {
  const queryClient = useQueryClient()
  const refresh = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: lineItemsKeys.record(recordType, recordId) }),
      queryClient.invalidateQueries({ queryKey: recordsKeys.detail(recordType, recordId) }),
      queryClient.invalidateQueries({ queryKey: recordsKeys.list(recordType) }),
    ])
  }
  const onSummary = async (summary: LineItems) => {
    queryClient.setQueryData(lineItemsKeys.record(recordType, recordId), summary)
    await refresh()
  }
  return {
    create: useMutation({
      mutationFn: (body: LineItemRequest) => lineItemsApi.create(recordType, recordId, body),
      onSettled: refresh,
    }),
    update: useMutation({
      mutationFn: ({ item, body }: { item: LineItem; body: LineItemRequest }) =>
        lineItemsApi.update(recordType, recordId, item.id, body, item.updated_at),
      onSettled: refresh,
    }),
    remove: useMutation({
      mutationFn: (item: LineItem) => lineItemsApi.remove(recordType, recordId, item.id),
      onSettled: refresh,
    }),
    reorder: useMutation({
      mutationFn: (itemIds: string[]) => lineItemsApi.reorder(recordType, recordId, itemIds),
      onSuccess: onSummary,
      onError: refresh,
    }),
    applyTotal: useMutation({
      mutationFn: () => lineItemsApi.applyTotal(recordType, recordId),
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
