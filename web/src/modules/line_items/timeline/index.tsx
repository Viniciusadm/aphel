import { ArrowUpDown, PackageMinus, PackagePlus, PackageSearch, type LucideIcon } from 'lucide-react'
import type { TimelineRenderer } from '@/features/records/timeline/registry'
import { LineItemEventContent } from './LineItemEventContent'
import type { LineItemEventType } from './events'

const ICONS: Record<LineItemEventType, LucideIcon> = {
  'line_items.item_added': PackagePlus,
  'line_items.item_updated': PackageSearch,
  'line_items.item_removed': PackageMinus,
  'line_items.items_reordered': ArrowUpDown,
}

export const DEAL_ITEM_TIMELINE_RENDERERS = Object.fromEntries(
  Object.entries(ICONS).map(([eventType, icon]) => [
    eventType,
    { icon, render: (item) => <LineItemEventContent item={item} /> } satisfies TimelineRenderer,
  ]),
) as Record<LineItemEventType, TimelineRenderer>
