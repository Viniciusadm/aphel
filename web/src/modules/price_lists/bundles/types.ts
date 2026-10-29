import type { components } from '@/api'

type Schemas = components['schemas']

export type BundlePricing = Schemas['BundlePricing']
export type BundleComponent = Schemas['BundleComponentResponse']
export type ProductBundle = Schemas['ProductBundleResponse']
export type ProductBundleRequest = Schemas['ProductBundleRequest']
