import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { RenewalSettings, RenewalSettingsRequest } from './types'

export const renewalsKeys = {
  all: ['renewals'] as const,
  pipeline: (pipelineId: string) => [...renewalsKeys.all, 'pipeline', pipelineId] as const,
}

function renewalPath(pipelineId: string) {
  return `/v1/pipelines/${encodeURIComponent(pipelineId)}/renewal`
}

export const renewalsApi = {
  get: (pipelineId: string) => api.get<RenewalSettings>(renewalPath(pipelineId)),
  save: (pipelineId: string, body: RenewalSettingsRequest, version?: string | null) =>
    api.put<RenewalSettings>(renewalPath(pipelineId), body, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
}

export function useRenewalSettings(pipelineId: string) {
  return useQuery({
    queryKey: renewalsKeys.pipeline(pipelineId),
    queryFn: () => renewalsApi.get(pipelineId),
    enabled: pipelineId !== '',
  })
}

export function useSaveRenewalSettings(pipelineId: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ body, version }: { body: RenewalSettingsRequest; version?: string | null }) =>
      renewalsApi.save(pipelineId, body, version),
    onSuccess: (settings) => {
      queryClient.setQueryData(renewalsKeys.pipeline(pipelineId), settings)
    },
  })
}
