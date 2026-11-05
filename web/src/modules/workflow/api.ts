import { api, apiRequest } from '@/api'
import type { LostReason, LostReasonsPage, Pipeline, PipelinesPage, PipelineStage, StageKind } from '@/sdk'

export { activePipelines, defaultPipeline, pipelinesKeys, useLostReasons, usePipelines } from '@/sdk'

export const WORKFLOW_RECORDS_QUERY_KEY = ['records'] as const

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
  const base = `/v1/workflow/${encodeURIComponent(id)}`
  return action ? `${base}/${action}` : base
}

export const pipelinesApi = {
  list: () => api.get<PipelinesPage>('/v1/workflow', { query: { per_page: 100 } }),
  create: (body: { name: string; team_ids?: string[] }) => api.post<Pipeline>('/v1/workflow', body),
  update: (id: string, body: { name?: string; team_ids?: string[] }) => api.patch<Pipeline>(pipelinePath(id), body),
  setDefault: (id: string) => api.post<Pipeline>(pipelinePath(id, 'default')),
  activate: (id: string) => api.post<Pipeline>(pipelinePath(id, 'activate')),
  deactivate: (id: string) => api.post<Pipeline>(pipelinePath(id, 'deactivate')),
  reorder: (ids: string[]) => api.put<Pipeline[]>('/v1/workflow/order', { ids }),
  remove: (id: string, body: PipelineRemovalBody) => apiRequest<void>(pipelinePath(id), { method: 'DELETE', body }),
  createStage: (pipelineId: string, body: StageBody & { name: string }) =>
    api.post<PipelineStage>(`/v1/workflow/${encodeURIComponent(pipelineId)}/stages`, body),
  reorderStages: (pipelineId: string, ids: string[]) =>
    api.put<Pipeline>(`/v1/workflow/${encodeURIComponent(pipelineId)}/stages/order`, { ids }),
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
