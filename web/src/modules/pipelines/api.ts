import { useQuery } from '@tanstack/react-query'
import { api, apiRequest } from '@/api'
import type { LostReason, LostReasonsPage, Pipeline, PipelinesPage, PipelineStage, StageKind } from './types'

export const pipelinesKeys = {
  all: ['pipelines'] as const,
  list: () => [...pipelinesKeys.all, 'list'] as const,
  lostReasons: (includeInactive: boolean) => [...pipelinesKeys.all, 'lost-reasons', includeInactive] as const,
}

export const DEAL_BOARDS_QUERY_KEY = ['deals', 'board'] as const

export interface StageBody {
  name?: string
  probability?: number
  kind?: StageKind
  rotting_days?: number | null
  entry_required_fields?: string[]
  exit_required_fields?: string[]
}

export interface PipelineRemovalBody {
  target_pipeline_id?: string
  stage_mapping?: Record<string, string>
}

function pipelinePath(id: string, action?: string) {
  const base = `/v1/pipelines/${encodeURIComponent(id)}`
  return action ? `${base}/${action}` : base
}

export const pipelinesApi = {
  list: () => api.get<PipelinesPage>('/v1/pipelines', { query: { per_page: 100 } }),
  create: (body: { name: string; team_ids?: string[] }) => api.post<Pipeline>('/v1/pipelines', body),
  update: (id: string, body: { name?: string; team_ids?: string[] }) => api.patch<Pipeline>(pipelinePath(id), body),
  setDefault: (id: string) => api.post<Pipeline>(pipelinePath(id, 'default')),
  activate: (id: string) => api.post<Pipeline>(pipelinePath(id, 'activate')),
  deactivate: (id: string) => api.post<Pipeline>(pipelinePath(id, 'deactivate')),
  reorder: (ids: string[]) => api.put<Pipeline[]>('/v1/pipelines/order', { ids }),
  remove: (id: string, body: PipelineRemovalBody) => apiRequest<void>(pipelinePath(id), { method: 'DELETE', body }),
  createStage: (pipelineId: string, body: StageBody & { name: string }) =>
    api.post<PipelineStage>(`/v1/pipelines/${encodeURIComponent(pipelineId)}/stages`, body),
  reorderStages: (pipelineId: string, ids: string[]) =>
    api.put<Pipeline>(`/v1/pipelines/${encodeURIComponent(pipelineId)}/stages/order`, { ids }),
  updateStage: (id: string, body: StageBody) =>
    api.patch<PipelineStage>(`/v1/pipeline-stages/${encodeURIComponent(id)}`, body),
  activateStage: (id: string) => api.post<PipelineStage>(`/v1/pipeline-stages/${encodeURIComponent(id)}/activate`),
  deactivateStage: (id: string, targetStageId?: string) =>
    api.post<PipelineStage>(`/v1/pipeline-stages/${encodeURIComponent(id)}/deactivate`, {
      target_stage_id: targetStageId,
    }),
  deleteStage: (id: string, targetStageId?: string) =>
    api.delete<void>(`/v1/pipeline-stages/${encodeURIComponent(id)}`, {
      query: { target_stage_id: targetStageId },
    }),
}

export const lostReasonsApi = {
  list: (includeInactive: boolean) =>
    api.get<LostReasonsPage>('/v1/lost-reasons', { query: { include_inactive: includeInactive, per_page: 100 } }),
  create: (name: string) => api.post<LostReason>('/v1/lost-reasons', { name }),
  update: (id: string, body: { name?: string; is_active?: boolean }) =>
    api.patch<LostReason>(`/v1/lost-reasons/${encodeURIComponent(id)}`, body),
  reorder: (ids: string[]) => api.put<LostReason[]>('/v1/lost-reasons/order', { ids }),
}

export function usePipelines() {
  return useQuery({
    queryKey: pipelinesKeys.list(),
    queryFn: async () => (await pipelinesApi.list()).data,
    staleTime: 60_000,
  })
}

export function useLostReasons(includeInactive = false) {
  return useQuery({
    queryKey: pipelinesKeys.lostReasons(includeInactive),
    queryFn: async () => (await lostReasonsApi.list(includeInactive)).data,
    staleTime: 60_000,
  })
}

export function defaultPipeline(pipelines: Pipeline[] | undefined): Pipeline | undefined {
  const active = pipelines?.filter((pipeline) => pipeline.is_active)
  return active?.find((pipeline) => pipeline.is_default) ?? active?.[0] ?? pipelines?.[0]
}

export function activePipelines(pipelines: Pipeline[] | undefined): Pipeline[] {
  return pipelines?.filter((pipeline) => pipeline.is_active) ?? []
}
