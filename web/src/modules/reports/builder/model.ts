import { metricKey } from '@/features/records/aggregates'
import type { FilterNode } from '@/features/records/types'
import type {
  AggregateFunction,
  AggregateGroupBy,
  AggregateMetric,
  AggregateQuery,
  AggregateSort,
  TimeGranularity,
} from '@/features/records/aggregates'
import type { PeriodPreset, Report, ReportSettings, ReportVisibility, ReportVisualization } from './api'

export const MAX_GROUPS = 3
export const MAX_METRICS = 10
export const MAX_REPORT_NAME_LENGTH = 140
export const DEFAULT_LIMIT = 500
export const MAX_LIMIT = 1000

export const REPORT_RECORD_TYPES = [
  { recordType: 'deals.deal', permission: 'deals.read' },
  { recordType: 'people.person', permission: 'people.read' },
  { recordType: 'organizations.organization', permission: 'organizations.read' },
  { recordType: 'activities.activity', permission: 'activities.read' },
] as const

export type ReportRecordType = (typeof REPORT_RECORD_TYPES)[number]['recordType']

export const VISUALIZATIONS: ReportVisualization[] = ['table', 'bar', 'line', 'pie', 'number']

export const PERIOD_PRESETS: PeriodPreset[] = [
  'today',
  'yesterday',
  'last_7_days',
  'last_30_days',
  'week',
  'last_week',
  'month',
  'last_month',
  'quarter',
  'last_quarter',
  'year',
  'last_year',
  'custom',
]

export interface PeriodDraft {
  field: string
  preset: PeriodPreset
  from: string
  to: string
}

export interface ReportDraft {
  name: string
  description: string
  visibility: ReportVisibility
  teamId: string
  visualization: ReportVisualization
  recordType: string
  filter: FilterNode | null
  groupBy: AggregateGroupBy[]
  metrics: AggregateMetric[]
  period: PeriodDraft | null
  compare: boolean
  sort: AggregateSort[]
  limit: number
  chartMetric: string | null
}

export function emptyDraft(recordType: string): ReportDraft {
  return {
    name: '',
    description: '',
    visibility: 'private',
    teamId: '',
    visualization: 'table',
    recordType,
    filter: null,
    groupBy: [],
    metrics: [{ fn: 'count' }],
    period: null,
    compare: false,
    sort: [],
    limit: DEFAULT_LIMIT,
    chartMetric: null,
  }
}

export function draftFromReport(report: Report): ReportDraft {
  const definition = report.definition
  const period = definition.period
  return {
    name: report.name,
    description: report.description ?? '',
    visibility: report.visibility,
    teamId: report.team_id ?? '',
    visualization: report.visualization,
    recordType: report.record_type,
    filter: definition.filter ?? null,
    groupBy: definition.group_by ?? [],
    metrics: definition.metrics && definition.metrics.length > 0 ? definition.metrics : [{ fn: 'count' }],
    period: period
      ? {
          field: period.field,
          preset: (period.preset as PeriodPreset | null | undefined) ?? 'month',
          from: period.from ?? '',
          to: period.to ?? '',
        }
      : null,
    compare: definition.compare_to_previous ?? false,
    sort: definition.sort ?? [],
    limit: definition.limit ?? DEFAULT_LIMIT,
    chartMetric: typeof report.settings.chart_metric === 'string' ? report.settings.chart_metric : null,
  }
}

export function isPeriodComplete(period: PeriodDraft | null): boolean {
  if (!period) return true
  if (!period.field) return false
  if (period.preset !== 'custom') return true
  return Boolean(period.from && period.to && period.from <= period.to)
}

export function draftQuery(draft: ReportDraft): AggregateQuery {
  const period = draft.period && draft.period.field ? draft.period : null
  return {
    record_type: draft.recordType,
    filter: draft.filter,
    group_by: draft.groupBy.filter((group) => group.field),
    metrics: draft.metrics.filter(isMetricComplete).map(cleanMetric),
    period: period
      ? {
          field: period.field,
          preset: period.preset,
          from: period.preset === 'custom' ? period.from || null : null,
          to: period.preset === 'custom' ? period.to || null : null,
        }
      : null,
    compare_to_previous: Boolean(period) && draft.compare,
    sort: draft.sort.filter((sort) => sort.key),
    limit: draft.limit,
  }
}

export function draftSettings(draft: ReportDraft): ReportSettings {
  return draft.chartMetric ? { chart_metric: draft.chartMetric } : {}
}

export function serializeDraft(draft: ReportDraft): string {
  return JSON.stringify({
    ...draftQuery(draft),
    name: draft.name.trim(),
    description: draft.description.trim(),
    visibility: draft.visibility,
    teamId: draft.visibility === 'team' ? draft.teamId : '',
    visualization: draft.visualization,
    settings: draftSettings(draft),
  })
}

export function isDurationMetric(metric: AggregateMetric): boolean {
  return metric.to !== undefined && metric.to !== null
}

export function isMetricComplete(metric: AggregateMetric): boolean {
  if (isDurationMetric(metric)) return Boolean(metric.field && metric.to && metric.field !== metric.to)
  return metric.fn === 'count' || Boolean(metric.field)
}

function cleanMetric(metric: AggregateMetric): AggregateMetric {
  if (!isDurationMetric(metric)) return metric.field ? { fn: metric.fn, field: metric.field } : { fn: metric.fn }
  return { fn: metric.fn, field: metric.field, to: metric.to, unit: metric.unit ?? 'days' }
}

export function metricKeyOf(metric: AggregateMetric): string {
  return metricKey(metric)
}

export function needsField(fn: AggregateFunction): boolean {
  return fn !== 'count'
}

export function withGranularity(group: AggregateGroupBy, temporal: boolean): AggregateGroupBy {
  if (!temporal) return { field: group.field }
  return { field: group.field, granularity: group.granularity ?? ('month' as TimeGranularity) }
}
