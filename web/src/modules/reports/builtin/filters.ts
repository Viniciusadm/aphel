import { useCallback, useMemo } from 'react'
import { useSearchParams } from 'react-router'
import { DEFAULT_PERIOD, PERIOD_PRESETS, isValidCustomRange } from '@/modules/dashboard/period'
import type { LayoutPeriod, PeriodPreset } from '@/modules/dashboard/types'
import { PERFORMANCE_DIMENSIONS, type PerformanceDimension, type PerformanceFilters } from './api'

function parsePeriod(params: URLSearchParams): LayoutPeriod {
  const preset = params.get('period') as PeriodPreset | null
  if (!preset || !PERIOD_PRESETS.includes(preset)) return DEFAULT_PERIOD
  if (preset !== 'custom') return { preset, from: null, to: null }
  const from = params.get('from')
  const to = params.get('to')
  return isValidCustomRange(from, to) ? { preset, from, to } : DEFAULT_PERIOD
}

function parseDimension(value: string | null): PerformanceDimension {
  return PERFORMANCE_DIMENSIONS.find((dimension) => dimension === value) ?? 'owner'
}

export interface ReportFiltersState {
  filters: PerformanceFilters
  setPeriod: (period: LayoutPeriod) => void
  setPipeline: (pipelineId: string | null) => void
  setCompare: (compare: boolean) => void
  setDimension: (dimension: PerformanceDimension) => void
}

export function useReportFilters(): ReportFiltersState {
  const [params, setParams] = useSearchParams()

  const filters = useMemo<PerformanceFilters>(
    () => ({
      period: parsePeriod(params),
      pipelineId: params.get('pipeline_id') || null,
      compare: params.get('compare') === 'true',
      dimension: parseDimension(params.get('dimension')),
    }),
    [params],
  )

  const change = useCallback(
    (apply: (next: URLSearchParams) => void) =>
      setParams(
        (current) => {
          const next = new URLSearchParams(current)
          apply(next)
          return next
        },
        { replace: true },
      ),
    [setParams],
  )

  const setPeriod = useCallback(
    (period: LayoutPeriod) =>
      change((next) => {
        next.set('period', period.preset)
        if (period.preset === 'custom' && period.from && period.to) {
          next.set('from', period.from)
          next.set('to', period.to)
        } else {
          next.delete('from')
          next.delete('to')
        }
      }),
    [change],
  )

  const setPipeline = useCallback(
    (pipelineId: string | null) =>
      change((next) => {
        if (pipelineId) next.set('pipeline_id', pipelineId)
        else next.delete('pipeline_id')
      }),
    [change],
  )

  const setCompare = useCallback(
    (compare: boolean) =>
      change((next) => {
        if (compare) next.set('compare', 'true')
        else next.delete('compare')
      }),
    [change],
  )

  const setDimension = useCallback(
    (dimension: PerformanceDimension) => change((next) => next.set('dimension', dimension)),
    [change],
  )

  return { filters, setPeriod, setPipeline, setCompare, setDimension }
}
