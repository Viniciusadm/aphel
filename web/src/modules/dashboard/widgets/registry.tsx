import { moduleWidgetRenderer } from '@/kernel/modules/contributions'
import type { WidgetRenderer } from '@/kernel/widgets/types'
import type { RecentActivityData } from '../types'
import { CollectionWidget, type CollectionData } from './CollectionWidget'
import { RecentActivityWidget } from './RecentActivityWidget'

export const DASHBOARD_WIDGETS: Record<string, WidgetRenderer> = {
  pending_center: {
    render: (data) => <CollectionWidget collection="pending" data={data as CollectionData} />,
    isEmpty: (data) => (data as CollectionData).sections.length === 0,
    settings: [
      { kind: 'integer', key: 'stale_days', label: 'widgets.pending_center.staleDays', min: 1, max: 365 },
      { kind: 'boolean', key: 'only_mine', label: 'widgets.pending_center.onlyMine' },
    ],
  },
  report_totals: { render: (data) => <CollectionWidget collection="summary" data={data as CollectionData} /> },
  recent_activity: {
    render: (data) => <RecentActivityWidget data={data as RecentActivityData} />,
    isEmpty: (data) => (data as RecentActivityData).items.length === 0,
    settings: [{ kind: 'integer', key: 'limit', label: 'widgets.recent_activity.limit', min: 5, max: 50 }],
  },
}

export function widgetRenderer(key: string): WidgetRenderer | undefined {
  return moduleWidgetRenderer(key)
}
