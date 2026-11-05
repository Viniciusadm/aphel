import { useQuery } from '@tanstack/react-query'
import { api, type components } from '@/api'

type Schemas = components['schemas']

export type Pipeline = Schemas['PipelineResponse']
export type PipelinesPage = Schemas['Page_PipelineResponse']
export type PipelineStage = Schemas['PipelineStageResponse']
export type StagesPage = Schemas['Page_PipelineStageResponse']
export type StageKind = Schemas['StageKind']
export type LostReason = Schemas['LostReasonResponse']
export type LostReasonsPage = Schemas['Page_LostReasonResponse']

export const pipelinesKeys = {
  all: ['workflow'] as const,
  list: () => [...pipelinesKeys.all, 'list'] as const,
  lostReasons: (includeInactive: boolean) => [...pipelinesKeys.all, 'lost-reasons', includeInactive] as const,
}

export function usePipelines() {
  return useQuery({
    queryKey: pipelinesKeys.list(),
    queryFn: async () => (await api.get<PipelinesPage>('/v1/workflow', { query: { per_page: 100 } })).data,
    staleTime: 60_000,
  })
}

export function useLostReasons(includeInactive = false) {
  return useQuery({
    queryKey: pipelinesKeys.lostReasons(includeInactive),
    queryFn: async () =>
      (
        await api.get<LostReasonsPage>('/v1/lost-reasons', {
          query: { include_inactive: includeInactive, per_page: 100 },
        })
      ).data,
    staleTime: 60_000,
  })
}

export function defaultPipeline(workflow: Pipeline[] | undefined): Pipeline | undefined {
  const active = workflow?.filter((pipeline) => pipeline.is_active)
  return active?.find((pipeline) => pipeline.is_default) ?? active?.[0] ?? workflow?.[0]
}

export function activePipelines(workflow: Pipeline[] | undefined): Pipeline[] {
  return workflow?.filter((pipeline) => pipeline.is_active) ?? []
}

export function requiredStageFields(from: PipelineStage | undefined, to: PipelineStage): string[] {
  const keys: string[] = []
  for (const key of [...(from?.exit_required_fields ?? []), ...(to.entry_required_fields ?? [])]) {
    if (!keys.includes(key)) keys.push(key)
  }
  return keys
}
