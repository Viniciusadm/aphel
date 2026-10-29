import type { QuoteDiscountType, QuoteItem, QuoteItemRequest } from './types'

export interface EditableItem extends QuoteItemRequest {
  key: string
  saved?: QuoteItem
}

export const DISCOUNT_TYPES: QuoteDiscountType[] = ['none', 'percent', 'amount']

export function editableFromItem(item: QuoteItem): EditableItem {
  return {
    key: item.id,
    saved: item,
    product_id: item.product_id,
    name: item.name,
    description: item.description,
    unit: item.unit,
    quantity: item.quantity,
    unit_price: item.unit_price,
    discount_type: item.discount_type,
    discount_value: item.discount_value,
    tax_percent: item.tax_percent,
    recurrence: item.recurrence,
    term_months: item.term_months,
  }
}

export function itemRequest({ key: _key, saved: _saved, ...request }: EditableItem): QuoteItemRequest {
  return request
}
