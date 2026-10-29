import type { components } from '@/api'

type Schemas = components['schemas']

export type PriceListAppliesTo = Schemas['PriceListAppliesTo']
export type PriceList = Schemas['PriceListResponse']
export type PriceListsPage = Schemas['Page_PriceListResponse']
export type PriceListRequest = Schemas['PriceListRequest']
export type PriceListItem = Schemas['PriceListItemResponse']
export type PriceListItemRequest = Schemas['PriceListItemRequest']
export type DiscountLimit = Schemas['DiscountLimitResponse']
export type DiscountLimitRequest = Schemas['DiscountLimitRequest']
