import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
import { recordsKeys } from '@/sdk'
import { PRODUCT_RECORD_TYPE } from './resource'

type Schemas = components['schemas']

export type Product = Schemas['ProductResponse']
export type ProductsPage = Schemas['Page_ProductResponse']
export type Recurrence = Schemas['Recurrence']
export const PRODUCT_PICKER_PAGE_SIZE = 20

export const productsKeys = {
  all: ['catalog_items'] as const,
  picker: (search: string) => [...recordsKeys.lookup(PRODUCT_RECORD_TYPE), 'picker', search] as const,
}

export const productsApi = {
  searchActive: (search: string) =>
    api.get<ProductsPage>('/v1/catalog-items', {
      query: {
        search: search.trim() || undefined,
        'filter[is_active][eq]': 'true',
        sort: 'name',
        page: 1,
        per_page: PRODUCT_PICKER_PAGE_SIZE,
      },
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
