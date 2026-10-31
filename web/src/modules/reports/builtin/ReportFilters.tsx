import { useTranslation } from 'react-i18next'
import { usePipelines } from '@/modules/pipelines/api'
import { PeriodSelector } from '@/modules/dashboard/PeriodSelector'
import { CompareToggle } from '@/modules/dashboard/SubjectSelector'
import { PERFORMANCE_DIMENSIONS, type PerformanceDimension } from './api'
import type { ReportFiltersState } from './filters'
import { NativeSelect } from '@/components/ui/native-select'
import { Field } from '@/components/ui/field'

export interface ReportFiltersProps {
  state: ReportFiltersState
  allPipelines?: boolean
  withDimension?: boolean
}

export function ReportFilters({ state, allPipelines = false, withDimension = false }: ReportFiltersProps) {
  const { t } = useTranslation('reports', { keyPrefix: 'builtin' })
  const pipelines = usePipelines()
  const { filters } = state

  return (
    <div className="flex min-w-0 flex-wrap items-end gap-3 rounded-lg border bg-card p-3">
      {withDimension && (
        <Field label={t('filters.dimension')} className="w-full min-w-0 sm:w-auto">
          {(control) => <NativeSelect
            className="w-full sm:w-52"
            {...control}
            value={filters.dimension}
            onChange={(event) => state.setDimension(event.target.value as PerformanceDimension)}
          >
            {PERFORMANCE_DIMENSIONS.map((dimension) => (
              <option key={dimension} value={dimension}>
                {t(`dimensions.${dimension}`)}
              </option>
            ))}
          </NativeSelect>}
        </Field>
      )}
      <Field label={t('filters.pipeline')} className="w-full min-w-0 sm:w-auto">
        {(control) => <NativeSelect
            className="w-full sm:w-52"
          {...control}
          value={filters.pipelineId ?? ''}
          disabled={pipelines.isPending}
          onChange={(event) => state.setPipeline(event.target.value || null)}
        >
          <option value="">{allPipelines ? t('filters.allPipelines') : t('filters.defaultPipeline')}</option>
          {(pipelines.data ?? []).map((pipeline) => (
            <option key={pipeline.id} value={pipeline.id}>
              {pipeline.name}
            </option>
          ))}
        </NativeSelect>}
      </Field>
      <PeriodSelector value={filters.period} onChange={state.setPeriod} />
      <CompareToggle checked={filters.compare} onChange={state.setCompare} />
    </div>
  )
}
