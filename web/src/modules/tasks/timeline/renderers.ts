import { ArrowRightLeft, CircleDollarSign, NotebookPen, SquareCheck } from 'lucide-react'
import type { TimelineRenderer } from '@/features/records/timeline'
import { timelineRenderer as renderer } from '@/modules/timeline'
import {
  ActivityTimelineContent,
  DealChangeTimelineContent,
  DealStageTimelineContent,
  NoteTimelineContent,
} from './contents'

export const ACTIVITY_TIMELINE_RENDERERS: Record<string, TimelineRenderer> = {
  activity: renderer(SquareCheck, ActivityTimelineContent),
  note: renderer(NotebookPen, NoteTimelineContent),
  deal_stage: renderer(ArrowRightLeft, DealStageTimelineContent),
  deal_change: renderer(CircleDollarSign, DealChangeTimelineContent),
}
