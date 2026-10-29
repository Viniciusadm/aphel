import { History, Paperclip, PencilLine } from 'lucide-react'
import { createElement, type ComponentType } from 'react'
import type { TimelineItem, TimelineRenderer } from '@/features/records/timeline'
import { AttachmentTimelineContent, EventTimelineContent, RecordChangeTimelineContent } from './contents'

export function timelineRenderer(
  icon: TimelineRenderer['icon'],
  component: ComponentType<{ item: TimelineItem }>,
): TimelineRenderer {
  return { icon, render: (item: TimelineItem) => createElement(component, { item }) }
}

export const TIMELINE_RENDERERS: Record<string, TimelineRenderer> = {
  record_change: timelineRenderer(PencilLine, RecordChangeTimelineContent),
  attachment: timelineRenderer(Paperclip, AttachmentTimelineContent),
  event: timelineRenderer(History, EventTimelineContent),
}
