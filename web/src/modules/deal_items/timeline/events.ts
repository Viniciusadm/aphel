export const DEAL_ITEM_EVENT_TYPES = [
  'deal.item_added',
  'deal.item_updated',
  'deal.item_removed',
  'deal.items_reordered',
] as const

export type DealItemEventType = (typeof DEAL_ITEM_EVENT_TYPES)[number]
