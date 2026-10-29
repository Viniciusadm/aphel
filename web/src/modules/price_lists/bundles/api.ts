import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { ProductBundle, ProductBundleRequest } from './types'

const id = encodeURIComponent

export const bundlesKeys = {
  all: ['product-bundles'] as const,
  product: (productId: string) => [...bundlesKeys.all, productId] as const,
  detail: (productId: string, priceListId: string) => [...bundlesKeys.product(productId), priceListId] as const,
}

function bundlePath(productId: string): string {
  return `/v1/products/${id(productId)}/bundle`
}

export const bundlesApi = {
  get: (productId: string, priceListId?: string) =>
    api.get<ProductBundle | null>(bundlePath(productId), {
      query: { price_list_id: priceListId || undefined },
    }),
  save: (productId: string, body: ProductBundleRequest, version?: string | null) =>
    api.put<ProductBundle>(bundlePath(productId), body, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
  remove: (productId: string) => api.delete<void>(bundlePath(productId)),
}

export function useProductBundle(productId: string, priceListId = '') {
  return useQuery({
    queryKey: bundlesKeys.detail(productId, priceListId),
    queryFn: () => bundlesApi.get(productId, priceListId),
    enabled: Boolean(productId),
  })
}

export function useInvalidateBundle(productId: string) {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: bundlesKeys.product(productId) })
}
