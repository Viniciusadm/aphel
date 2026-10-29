import { moduleWidgetRenderer } from '@/kernel/modules/contributions'
import { lazyComponent } from '@/lib/lazy-component'
import type { WidgetRenderer } from '@/kernel/widgets/types'
import type {
  DealsByStageData,
  PendingCenterData,
  RecentActivityData,
  ReportTotalsData,
  SalesIndicatorsData,
} from '../types'
import { PendingCenterWidget } from './PendingCenterWidget'
import { RecentActivityWidget } from './RecentActivityWidget'
import { ReportTotalsWidget } from './ReportTotalsWidget'
import { SalesIndicatorsWidget } from './SalesIndicatorsWidget'

const DealsByStageWidget = lazyComponent(async () => (await import('./DealsByStageWidget')).DealsByStageWidget)

function isPendingCenterEmpty(data: PendingCenterData): boolean {
  return data.overdue.count + data.due_today.count + data.due_tomorrow.count + (data.stale_deals?.count ?? 0) === 0
}

export const DASHBOARD_WIDGETS: Record<string, WidgetRenderer> = {
  sales_indicators: {
    render: (data) => <SalesIndicatorsWidget data={data as SalesIndicatorsData} />,
  },
  pending_center: {
    render: (data) => <PendingCenterWidget data={data as PendingCenterData} />,
    isEmpty: (data) => isPendingCenterEmpty(data as PendingCenterData),
    settings: [
      { kind: 'integer', key: 'stale_days', label: 'widgets.pending_center.staleDays', min: 1, max: 365 },
      { kind: 'boolean', key: 'only_mine', label: 'widgets.pending_center.onlyMine' },
    ],
  },
  recent_activity: {
    render: (data) => <RecentActivityWidget data={data as RecentActivityData} />,
    isEmpty: (data) => (data as RecentActivityData).items.length === 0,
    settings: [{ kind: 'integer', key: 'limit', label: 'widgets.recent_activity.limit', min: 5, max: 50 }],
  },
  report_totals: {
    render: (data) => <ReportTotalsWidget data={data as ReportTotalsData} />,
  },
  deals_by_stage: {
    render: (data) => <DealsByStageWidget data={data as DealsByStageData} />,
    isEmpty: (data) => {
      const stages = (data as DealsByStageData).stages
      return (data as DealsByStageData).pipeline !== null && stages.every((stage) => stage.count === 0)
    },
  },
}

export function widgetRenderer(key: string): WidgetRenderer | undefined {
  return moduleWidgetRenderer(key)
}
