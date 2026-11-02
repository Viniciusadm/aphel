import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type {
  CreateValidationRuleRequest,
  UpdateValidationRuleRequest,
  ValidationRule,
  ValidationRulePreview,
  ValidationRulePreviewRequest,
} from '@/modules/validation_rules/ui/types'

export const validationRulesKeys = {
  all: ['validation-rules'] as const,
  list: (recordType: string) => [...validationRulesKeys.all, 'list', recordType] as const,
  preview: (request: ValidationRulePreviewRequest) => [...validationRulesKeys.all, 'preview', request] as const,
}

function rulePath(id: string) {
  return `/v1/validation-rules/${encodeURIComponent(id)}`
}

export const validationRulesApi = {
  list: (recordType: string) =>
    api.get<ValidationRule[]>('/v1/validation-rules', { query: { record_type: recordType } }),
  create: (body: CreateValidationRuleRequest) => api.post<ValidationRule>('/v1/validation-rules', body),
  update: (id: string, body: UpdateValidationRuleRequest, version: string) =>
    api.patch<ValidationRule>(rulePath(id), body, { headers: { 'If-Match': version } }),
  remove: (id: string) => api.delete<void>(rulePath(id)),
  reorder: (recordType: string, ids: string[]) =>
    api.put<ValidationRule[]>('/v1/validation-rules/order', { record_type: recordType, ids }),
  preview: (body: ValidationRulePreviewRequest, signal?: AbortSignal) =>
    api.post<ValidationRulePreview>('/v1/validation-rules/preview', body, { signal }),
}

export function useValidationRules(recordType: string) {
  return useQuery({
    queryKey: validationRulesKeys.list(recordType),
    queryFn: () => validationRulesApi.list(recordType),
  })
}

export function useValidationRulePreview(request: ValidationRulePreviewRequest | null) {
  return useQuery({
    queryKey: validationRulesKeys.preview(request ?? { record_type: '', expression: '' }),
    queryFn: ({ signal }) => validationRulesApi.preview(request!, signal),
    enabled: request !== null,
    placeholderData: keepPreviousData,
    retry: false,
    staleTime: 10_000,
  })
}

export function useInvalidateValidationRules() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: validationRulesKeys.all })
}
