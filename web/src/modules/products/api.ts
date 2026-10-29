import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
import { recordsKeys } from '@/features/records/api'
import { PRODUCT_RECORD_TYPE } from './resource'

type Schemas = components['schemas']

export type Product = Schemas['ProductResponse']
export type ProductsPage = Schemas['Page_ProductResponse']
export type ProductRecurrence = Schemas['ProductRecurrence']
export type DealAmountMode = Schemas['DealAmountMode']
export type DealItemSettings = Schemas['DealItemSettingsResponse']
export type UpdateDealItemSettings = Schemas['UpdateDealItemSettingsRequest']

export const PRODUCT_PICKER_PAGE_SIZE = 20
export const DEAL_AMOUNT_MODES: DealAmountMode[] = ['manual', 'items']

export const productsKeys = {
  all: ['products'] as const,
  picker: (search: string) => [...recordsKeys.lookup(PRODUCT_RECORD_TYPE), 'picker', search] as const,
  settings: () => [...productsKeys.all, 'deal-item-settings'] as const,
}

export const productsApi = {
  searchActive: (search: string) =>
    api.get<ProductsPage>('/v1/products', {
      query: {
        search: search.trim() || undefined,
        'filter[is_active][eq]': 'true',
        sort: 'name',
        page: 1,
        per_page: PRODUCT_PICKER_PAGE_SIZE,
      },
    }),
  dealItemSettings: () => api.get<DealItemSettings>('/v1/deal-items/settings'),
  updateDealItemSettings: (body: UpdateDealItemSettings, version?: string | null) =>
    api.patch<DealItemSettings>('/v1/deal-items/settings', body, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
}

export function useProductPicker(search: string, enabled: boolean) {
  return useQuery({
    queryKey: productsKeys.picker(search.trim()),
    queryFn: () => productsApi.searchActive(search),
    enabled,
    placeholderData: keepPreviousData,
  })
}

export function useDealItemSettings() {
  return useQuery({
    queryKey: productsKeys.settings(),
    queryFn: productsApi.dealItemSettings,
  })
}
