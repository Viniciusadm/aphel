import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { RecordChecklist, StageGuidance, StageGuidanceRequest } from './types'

export const STAGE_CHECKLIST_INCOMPLETE = 'stage_checklist_incomplete'

export const stageGuidanceKeys = {
  all: ['stage-guidance'] as const,
  stage: (stageId: string) => [...stageGuidanceKeys.all, 'stage', stageId] as const,
  record: (recordType: string, recordId: string) => [...stageGuidanceKeys.all, 'record', recordType, recordId] as const,
  recordStage: (recordType: string, recordId: string, stageId: string) => [...stageGuidanceKeys.record(recordType, recordId), stageId] as const,
}

export const stageGuidanceApi = {
  get: (stageId: string) => api.get<StageGuidance>(`/v1/pipeline-stages/${encodeURIComponent(stageId)}/guidance`),
  save: (stageId: string, body: StageGuidanceRequest, version: string | null) =>
    api.put<StageGuidance>(`/v1/pipeline-stages/${encodeURIComponent(stageId)}/guidance`, body, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
  recordChecklist: (recordType: string, recordId: string) => api.get<RecordChecklist>(`/v1/workflow/records/${encodeURIComponent(recordType)}/${encodeURIComponent(recordId)}/checklist`),
  toggle: (recordType: string, recordId: string, itemId: string, done: boolean) =>
    api.put<RecordChecklist>(`/v1/workflow/records/${encodeURIComponent(recordType)}/${encodeURIComponent(recordId)}/checklist/${encodeURIComponent(itemId)}`, { done }),
}

export function useStageGuidance(stageId: string) {
  return useQuery({
    queryKey: stageGuidanceKeys.stage(stageId),
    queryFn: () => stageGuidanceApi.get(stageId),
  })
}

export function useRecordChecklist(recordType: string, recordId: string, stageId: string | undefined) {
  return useQuery({
    queryKey: stageGuidanceKeys.recordStage(recordType, recordId, stageId ?? ''),
    queryFn: () => stageGuidanceApi.recordChecklist(recordType, recordId),
    enabled: Boolean(recordType && recordId),
  })
}
