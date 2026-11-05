import { ArrowRightLeft, CircleDollarSign, NotebookPen, SquareCheck } from 'lucide-react'
import { type TimelineRenderer, timelineRenderer as renderer } from '@/sdk'
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
