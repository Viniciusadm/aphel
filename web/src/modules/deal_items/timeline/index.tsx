import { ArrowUpDown, PackageMinus, PackagePlus, PackageSearch, type LucideIcon } from 'lucide-react'
import type { TimelineRenderer } from '@/features/records/timeline/registry'
import { DealItemEventContent } from './DealItemEventContent'
import type { DealItemEventType } from './events'

const ICONS: Record<DealItemEventType, LucideIcon> = {
  'deal.item_added': PackagePlus,
  'deal.item_updated': PackageSearch,
  'deal.item_removed': PackageMinus,
  'deal.items_reordered': ArrowUpDown,
}

export const DEAL_ITEM_TIMELINE_RENDERERS = Object.fromEntries(
  Object.entries(ICONS).map(([eventType, icon]) => [
    eventType,
    { icon, render: (item) => <DealItemEventContent item={item} /> } satisfies TimelineRenderer,
  ]),
) as Record<DealItemEventType, TimelineRenderer>
