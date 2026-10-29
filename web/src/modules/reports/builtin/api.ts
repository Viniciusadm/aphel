import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
import { periodQuery } from '@/modules/dashboard/period'
import type { LayoutPeriod } from '@/modules/dashboard/types'

type Schemas = components['schemas']

export type ConversionReport = Schemas['ConversionReportResponse']
export type ConversionSnapshot = Schemas['ConversionSnapshotResponse']
export type ConversionSummary = Schemas['ConversionSummaryResponse']
export type StageFunnel = Schemas['StageFunnelResponse']
export type ReportStage = Schemas['ReportStageResponse']
export type LostReasonTotal = Schemas['LostReasonTotalResponse']
export type PerformanceReport = Schemas['PerformanceReportResponse']
export type PerformanceRow = Schemas['PerformanceRowResponse']
export type PerformanceMetrics = Schemas['PerformanceMetricsResponse']
export type PerformanceDimension = Schemas['PerformanceDimension']

export const PERFORMANCE_DIMENSIONS: PerformanceDimension[] = ['owner', 'team', 'source', 'pipeline']

export interface ConversionFilters {
  period: LayoutPeriod
  pipelineId: string | null
  compare: boolean
}

export interface PerformanceFilters extends ConversionFilters {
  dimension: PerformanceDimension
}

function baseQuery(filters: ConversionFilters) {
  return {
    ...periodQuery(filters.period),
    pipeline_id: filters.pipelineId ?? undefined,
    compare_to_previous: filters.compare ? true : undefined,
  }
}

export const builtinReportsKeys = {
  all: ['reports-builtin'] as const,
  conversion: (filters: ConversionFilters) => [...builtinReportsKeys.all, 'conversion', filters] as const,
  performance: (filters: PerformanceFilters) => [...builtinReportsKeys.all, 'performance', filters] as const,
}

export const builtinReportsApi = {
  conversion: (filters: ConversionFilters) =>
    api.get<ConversionReport>('/v1/reports/builtin/conversion', { query: baseQuery(filters) }),
  performance: (filters: PerformanceFilters) =>
    api.get<PerformanceReport>('/v1/reports/builtin/performance', {
      query: { ...baseQuery(filters), dimension: filters.dimension },
    }),
}

export function useConversionReport(filters: ConversionFilters) {
  return useQuery({
    queryKey: builtinReportsKeys.conversion(filters),
    queryFn: () => builtinReportsApi.conversion(filters),
    placeholderData: keepPreviousData,
    staleTime: 30 * 1000,
  })
}

export function usePerformanceReport(filters: PerformanceFilters) {
  return useQuery({
    queryKey: builtinReportsKeys.performance(filters),
    queryFn: () => builtinReportsApi.performance(filters),
    placeholderData: keepPreviousData,
    staleTime: 30 * 1000,
  })
}
