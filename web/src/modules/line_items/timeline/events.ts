export const DEAL_ITEM_EVENT_TYPES = [
  'line_items.item_added',
  'line_items.item_updated',
  'line_items.item_removed',
  'line_items.items_reordered',
] as const

export type LineItemEventType = (typeof DEAL_ITEM_EVENT_TYPES)[number]
