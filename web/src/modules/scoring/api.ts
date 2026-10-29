import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type {
  CreateScoringModelRequest,
  RecordScore,
  ScoringModel,
  UpdateScoringModelRequest,
} from './types'

export const scoringKeys = {
  all: ['scoring'] as const,
  models: () => [...scoringKeys.all, 'models'] as const,
  record: (recordType: string, recordId: string) => [...scoringKeys.all, 'record', recordType, recordId] as const,
}

function modelPath(id: string) {
  return `/v1/scoring/models/${encodeURIComponent(id)}`
}

export const scoringApi = {
  list: () => api.get<ScoringModel[]>('/v1/scoring/models'),
  create: (body: CreateScoringModelRequest) => api.post<ScoringModel>('/v1/scoring/models', body),
  update: (id: string, body: UpdateScoringModelRequest, version: string) =>
    api.patch<ScoringModel>(modelPath(id), body, { headers: { 'If-Match': version } }),
  remove: (id: string) => api.delete<void>(modelPath(id)),
  recalculate: (id: string) => api.post<void>(`${modelPath(id)}/recalculate`),
  recordScores: (recordType: string, recordId: string) =>
    api.get<RecordScore[]>(
      `/v1/scoring/records/${encodeURIComponent(recordType)}/${encodeURIComponent(recordId)}`,
    ),
}

export function useScoringModels() {
  return useQuery({ queryKey: scoringKeys.models(), queryFn: scoringApi.list })
}

export function useRecordScores(recordType: string, recordId: string) {
  return useQuery({
    queryKey: scoringKeys.record(recordType, recordId),
    queryFn: () => scoringApi.recordScores(recordType, recordId),
    enabled: Boolean(recordId),
  })
}

export function useInvalidateScoring() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: scoringKeys.all })
}
