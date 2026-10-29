import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { DealChecklist, StageGuidance, StageGuidanceRequest } from './types'

export const STAGE_CHECKLIST_INCOMPLETE = 'stage_checklist_incomplete'

export const stageGuidanceKeys = {
  all: ['stage-guidance'] as const,
  stage: (stageId: string) => [...stageGuidanceKeys.all, 'stage', stageId] as const,
  deal: (dealId: string) => [...stageGuidanceKeys.all, 'deal', dealId] as const,
  dealStage: (dealId: string, stageId: string) => [...stageGuidanceKeys.deal(dealId), stageId] as const,
}

export const stageGuidanceApi = {
  get: (stageId: string) => api.get<StageGuidance>(`/v1/pipeline-stages/${encodeURIComponent(stageId)}/guidance`),
  save: (stageId: string, body: StageGuidanceRequest, version: string | null) =>
    api.put<StageGuidance>(`/v1/pipeline-stages/${encodeURIComponent(stageId)}/guidance`, body, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
  dealChecklist: (dealId: string) => api.get<DealChecklist>(`/v1/deals/${encodeURIComponent(dealId)}/checklist`),
  toggle: (dealId: string, itemId: string, done: boolean) =>
    api.put<DealChecklist>(`/v1/deals/${encodeURIComponent(dealId)}/checklist/${encodeURIComponent(itemId)}`, { done }),
}

export function useStageGuidance(stageId: string) {
  return useQuery({
    queryKey: stageGuidanceKeys.stage(stageId),
    queryFn: () => stageGuidanceApi.get(stageId),
  })
}

export function useDealChecklist(dealId: string, stageId: string | undefined) {
  return useQuery({
    queryKey: stageGuidanceKeys.dealStage(dealId, stageId ?? ''),
    queryFn: () => stageGuidanceApi.dealChecklist(dealId),
    enabled: Boolean(dealId),
  })
}
