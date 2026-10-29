import { Activity, type LucideIcon } from 'lucide-react'
import { createElement, type ReactNode } from 'react'
import { moduleEventRenderer, moduleTimelineRenderer } from '@/kernel/modules/contributions'
import { moduleState, type ModuleState } from '@/kernel/modules/registry'
import { GenericTimelineContent } from './GenericTimelineContent'
import type { TimelineItem } from './types'

export interface TimelineRenderer {
  icon: LucideIcon
  render: (item: TimelineItem) => ReactNode
}

export const genericTimelineRenderer: TimelineRenderer = {
  icon: Activity,
  render: (item) => createElement(GenericTimelineContent, { item }),
}

export function resolveTimelineRenderer(kind: string, eventType?: unknown, current: ModuleState = moduleState()): TimelineRenderer {
  const byEvent = typeof eventType === 'string' ? moduleEventRenderer(eventType, current) : undefined
  return byEvent ?? moduleTimelineRenderer(kind, current) ?? genericTimelineRenderer
}
