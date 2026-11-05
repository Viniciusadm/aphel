import { Phone } from 'lucide-react'
import { createElement } from 'react'
import {ACTIVITY_TIMELINE_RENDERERS} from '@/sdk'
import { type TimelineRenderer } from '@/sdk'
import { CallTimelineContent } from './CallTimelineContent'

const fallback = ACTIVITY_TIMELINE_RENDERERS.activity

export const callAwareActivityRenderer: TimelineRenderer = {
  icon: fallback?.icon ?? Phone,
  render: (item) =>
    item.metadata.call && typeof item.metadata.call === 'object'
      ? createElement(CallTimelineContent, { item })
      : (fallback?.render(item) ?? null),
}
