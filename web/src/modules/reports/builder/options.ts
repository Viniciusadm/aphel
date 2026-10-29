import { useQueries } from '@tanstack/react-query'
import { useMemo } from 'react'
import type { TFunction } from 'i18next'
import { aggregatesApi, aggregatesKeys, type AggregateField, type AggregateMetric, type AggregateSchema } from '@/features/records/aggregates'

export interface GroupOption {
  key: string
  label: string
  temporal: boolean
}

export function useGroupOptions(schema: AggregateSchema | undefined): GroupOption[] {
  const relations = useMemo(
    () => (schema?.relations ?? []).filter((relation) => relation.single && relation.readable),
    [schema],
  )
  const related = useQueries({
    queries: relations.map((relation) => ({
      queryKey: aggregatesKeys.schema(relation.target_record_type),
      queryFn: () => aggregatesApi.schema(relation.target_record_type),
      staleTime: 60_000,
    })),
  })
  const own = (schema?.fields ?? [])
    .filter((field) => field.groupable)
    .map((field) => ({ key: field.key, label: field.label, temporal: field.temporal }))
  const nested = relations.flatMap((relation, index) =>
    (related[index]?.data?.fields ?? [])
      .filter((field) => field.groupable && !field.is_virtual)
      .map((field) => ({
        key: `${relation.key}.${field.key}`,
        label: `${relation.label} · ${field.label}`,
        temporal: field.temporal,
      })),
  )
  return [...own, ...nested]
}

export function metricName(metric: AggregateMetric, fields: AggregateField[], t: TFunction<'reports'>): string {
  const labelOf = (key: string) => fields.find((field) => field.key === key)?.label ?? key
  const fn = t(`functions.${metric.fn}`)
  if (metric.field && metric.to) {
    return t('durationLabel', {
      fn,
      from: labelOf(metric.field),
      to: labelOf(metric.to),
      unit: t(`durationUnits.${metric.unit ?? 'days'}`),
    })
  }
  return metric.field ? `${fn} · ${labelOf(metric.field)}` : fn
}

