import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import type { ReferenceOptionsContext, ReferenceOptionsProvider, ReferenceOptionsResult } from '@/kernel/modules/define'
import { activePipelines, pipelinesApi, pipelinesKeys } from './api'

function usePipelineStageOptions({ includeInactive = false, enabled = true }: ReferenceOptionsContext): ReferenceOptionsResult {
  const workflow = useQuery({
    queryKey: pipelinesKeys.list(),
    queryFn: async () => (await pipelinesApi.list()).data,
    staleTime: 60_000,
    enabled,
  })
  return useMemo(() => {
    const list = includeInactive ? (workflow.data ?? []) : activePipelines(workflow.data)
    return {
      options: list.flatMap((pipeline) =>
        pipeline.stages
          .filter((stage) => includeInactive || stage.is_active)
          .map((stage) => ({ value: stage.id, label: stage.name, group: pipeline.name })),
      ),
      isLoading: workflow.isPending,
    }
  }, [workflow.data, workflow.isPending, includeInactive])
}

export const PIPELINE_STAGE_REFERENCE_OPTIONS: ReferenceOptionsProvider = {
  use: usePipelineStageOptions,
  queryKey: pipelinesKeys.all,
}
