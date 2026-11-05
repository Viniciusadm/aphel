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
import type { LayoutPeriod } from '@/sdk'

export type { LayoutPeriod, PeriodPreset } from '@/sdk'
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
