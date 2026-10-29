import { UserCog, UserMinus, UserPlus, type LucideIcon } from 'lucide-react'
import type { TimelineRenderer } from '@/features/records/timeline/registry'
import { DealMemberEventContent } from './DealMemberEventContent'
import type { DealMemberEventType } from './types'

const ICONS: Record<DealMemberEventType, LucideIcon> = {
  'deal.member_added': UserPlus,
  'deal.member_updated': UserCog,
  'deal.member_removed': UserMinus,
}

export const DEAL_TEAM_TIMELINE_RENDERERS = Object.fromEntries(
  Object.entries(ICONS).map(([eventType, icon]) => [
    eventType,
    { icon, render: (item) => <DealMemberEventContent item={item} /> } satisfies TimelineRenderer,
  ]),
) as Record<DealMemberEventType, TimelineRenderer>
