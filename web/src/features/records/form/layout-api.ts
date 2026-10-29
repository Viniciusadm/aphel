import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { FormLayout, FormLayoutResponse, LayoutContext } from './lib/layout'

export const formLayoutKeys = {
  all: ['form-layouts'] as const,
  recordType: (recordType: string) => [...formLayoutKeys.all, recordType] as const,
  detail: (recordType: string, context: LayoutContext) => [...formLayoutKeys.recordType(recordType), context] as const,
}

function layoutPath(recordType: string, context: LayoutContext) {
  return `/v1/layouts/${encodeURIComponent(recordType)}/${context}`
}

export const formLayoutsApi = {
  get: (recordType: string, context: LayoutContext) => api.get<FormLayoutResponse>(layoutPath(recordType, context)),
  save: (recordType: string, context: LayoutContext, layout: FormLayout) =>
    api.put<FormLayoutResponse>(layoutPath(recordType, context), { layout }),
  reset: (recordType: string, context: LayoutContext) => api.delete<void>(layoutPath(recordType, context)),
}

export function useFormLayout(recordType: string, context: LayoutContext, { enabled = true }: { enabled?: boolean } = {}) {
  return useQuery({
    queryKey: formLayoutKeys.detail(recordType, context),
    queryFn: () => formLayoutsApi.get(recordType, context),
    staleTime: 60_000,
    enabled,
  })
}

export function savedLayout(response: FormLayoutResponse | undefined): FormLayout | null {
  return response?.is_custom && response.layout ? response.layout : null
}
