import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type AggregateQuery = Schemas['AggregateQueryRequest']
export type AggregateGroupBy = Schemas['AggregateGroupBy']
export type AggregateMetric = Schemas['AggregateMetric']
export type AggregatePeriod = Schemas['AggregatePeriod']
export type AggregateSort = Schemas['AggregateSort']
export type AggregateFunction = Schemas['AggregateFunction']
export type TimeGranularity = Schemas['TimeGranularity']
export type AggregateResult = Schemas['AggregateResultResponse']
export type AggregateRow = Schemas['AggregateRowResponse']
export type AggregateSchema = Schemas['AggregateSchemaResponse']
export type AggregateField = Schemas['AggregateFieldResponse']
export type AggregateRelation = Schemas['AggregateRelationResponse']

export const COUNT_METRIC = 'count'
export const TIME_GRANULARITIES: TimeGranularity[] = ['day', 'week', 'month', 'quarter', 'year']

export function metricKey(metric: AggregateMetric): string {
  if (metric.field && metric.to !== undefined && metric.to !== null) {
    return `${metric.fn}:${metric.field}..${metric.to}:${metric.unit ?? 'days'}`
  }
  return metric.field ? `${metric.fn}:${metric.field}` : metric.fn
}

export function metricIndex(result: AggregateResult, key: string): number {
  return result.metrics.findIndex((metric) => metric.key === key)
}

export function decimalValue(value: string | null | undefined): number | null {
  if (value === null || value === undefined) return null
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

export function totalOf(result: AggregateResult, key: string): number | null {
  const index = metricIndex(result, key)
  return index < 0 ? null : decimalValue(result.totals.values[index])
}

export function previousTotalOf(result: AggregateResult, key: string): number | null {
  const index = metricIndex(result, key)
  return index < 0 ? null : decimalValue(result.totals.previous?.[index])
}

export function variation(current: number | null, previous: number | null): number | null {
  if (current === null || previous === null || previous === 0) return null
  return (current - previous) / Math.abs(previous)
}

export const aggregatesKeys = {
  all: ['aggregates'] as const,
  run: (query: AggregateQuery) => [...aggregatesKeys.all, 'run', query] as const,
  schema: (recordType: string) => [...aggregatesKeys.all, 'schema', recordType] as const,
}

export const aggregatesApi = {
  run: (query: AggregateQuery) => api.post<AggregateResult>('/v1/aggregates', query),
  schema: (recordType: string) =>
    api.get<AggregateSchema>(`/v1/aggregates/${encodeURIComponent(recordType)}/schema`),
}

export function useAggregate(query: AggregateQuery | null, options: { enabled?: boolean } = {}) {
  return useQuery({
    queryKey: aggregatesKeys.run(query ?? { record_type: '' }),
    queryFn: () => aggregatesApi.run(query as AggregateQuery),
    enabled: query !== null && (options.enabled ?? true),
    placeholderData: keepPreviousData,
  })
}

export function useAggregateSchema(recordType: string | null) {
  return useQuery({
    queryKey: aggregatesKeys.schema(recordType ?? ''),
    queryFn: () => aggregatesApi.schema(recordType as string),
    enabled: Boolean(recordType),
    staleTime: 60_000,
  })
}
