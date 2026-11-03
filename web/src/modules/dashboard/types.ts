import type { components } from '@/api'
import type { WidgetRecordRef } from '@/kernel/widgets/types'

type Schemas = components['schemas']

export type WidgetSettings = Record<string, unknown>

export type WidgetDescriptor = Omit<Schemas['WidgetDescriptorResponse'], 'default_settings'> & {
  default_settings: WidgetSettings
}
export interface WidgetCatalogPage {
  data: WidgetDescriptor[]
  meta: Schemas['PageMeta']
}
export type WidgetSize = Schemas['DashboardWidgetSize']
export type PeriodPreset = Schemas['DashboardPeriodPreset']
export type LayoutPeriod = Schemas['DashboardLayoutPeriod']
export type ResolvedPeriod = Schemas['DashboardPeriodResponse']
export type DashboardSubject = Schemas['DashboardSubject']
export type SubjectType = Schemas['DashboardSubjectType']
export type DashboardSubjects = Schemas['DashboardSubjectsResponse']

export type LayoutWidget = Omit<Schemas['DashboardLayoutWidget'], 'settings'> & { settings?: WidgetSettings }

export interface DashboardLayout {
  widgets: LayoutWidget[]
  period: LayoutPeriod
  subject?: DashboardSubject | null
  compare_to_previous: boolean
  is_default: boolean
}

export interface SaveLayoutRequest {
  widgets: LayoutWidget[]
  period: LayoutPeriod
  subject?: DashboardSubject | null
  compare_to_previous?: boolean
}

export interface WidgetScope {
  period: LayoutPeriod | null
  subject: DashboardSubject | null
  compare: boolean
}

export type WidgetDataResponse = Omit<Schemas['WidgetDataResponse'], 'data' | 'settings'> & {
  data: unknown
  settings: WidgetSettings
}

export interface UserRef {
  id: string
  display_name: string
}

export type RecordRef = WidgetRecordRef

export interface ClosedDealsTotals {
  won_count: number
  won_amount?: string
  lost_count: number
  conversion_rate: number | null
}

export interface SalesIndicatorsData extends ClosedDealsTotals {
  currency: string
  open_count: number
  pipeline_amount: string
  forecast_amount: string
  previous: ClosedDealsTotals | null
}

export interface PendingActivityItem {
  id: string
  title: string
  priority: string | null
  due_at: string | null
  owner: UserRef | null
  record: RecordRef | null
}

export interface StaleLineItem {
  id: string
  title: string
  amount: string | null
  currency: string
  stage_name: string
  owner: UserRef | null
  last_touched_at: string
}

export interface Bucket<T> {
  count: number
  items: T[]
}

export interface PendingCenterData {
  overdue: Bucket<PendingActivityItem>
  due_today: Bucket<PendingActivityItem>
  due_tomorrow: Bucket<PendingActivityItem>
  stale_deals: (Bucket<StaleLineItem> & { stale_days: number }) | null
}

export interface RecentEventItem {
  id: string
  event_type: string
  occurred_at: string
  actor: UserRef | null
  record: RecordRef | null
  payload: Record<string, unknown> | null
}

export interface RecentActivityData {
  items: RecentEventItem[]
}

export interface ReportTotalsPrevious {
  won_deals: number | null
  lost_deals: number | null
  completed_activities: number | null
}

export interface ReportTotalsData {
  contacts_total: number | null
  open_deals: number | null
  won_deals: number | null
  lost_deals: number | null
  pending_tasks: number | null
  completed_activities: number | null
  previous?: ReportTotalsPrevious | null
}

export interface StageBar {
  id: string
  name: string
  kind: 'open' | 'won' | 'lost'
  probability: number
  count: number
  amount: string
}

export interface DealsByStageData {
  pipeline: { id: string; name: string } | null
  currency: string
  stages: StageBar[]
}
