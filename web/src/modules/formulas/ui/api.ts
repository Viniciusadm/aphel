import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { api, type components } from '@/api'
import { fieldsKeys } from '@/features/settings/fields/api'

export type FormulaCatalog = components['schemas']['FormulaCatalogResponse']
export type FormulaCatalogField = components['schemas']['FormulaCatalogFieldResponse']
export type FormulaCatalogRelation = components['schemas']['FormulaCatalogRelationResponse']
export type FormulaFunction = components['schemas']['FormulaFunctionResponse']
export type FormulaPreviewRequest = components['schemas']['FormulaPreviewRequest']
export type FormulaPreview = components['schemas']['FormulaPreviewResponse']
export type FormulaError = components['schemas']['FormulaErrorResponse']
export type FormulaDependencies = components['schemas']['FormulaDependenciesResponse']

export const formulaKeys = {
  all: [...fieldsKeys.all, 'formula'] as const,
  catalog: (recordType: string) => [...formulaKeys.all, 'catalog', recordType] as const,
  preview: (request: FormulaPreviewRequest) => [...formulaKeys.all, 'preview', request] as const,
}

export const formulaApi = {
  catalog: (recordType: string, signal?: AbortSignal) =>
    api.get<FormulaCatalog>('/v1/fields/formula/catalog', { query: { record_type: recordType }, signal }),
  preview: (request: FormulaPreviewRequest, signal?: AbortSignal) =>
    api.post<FormulaPreview>('/v1/fields/formula/preview', request, { signal }),
}

export function useFormulaCatalog(recordType: string) {
  return useQuery({
    queryKey: formulaKeys.catalog(recordType),
    queryFn: ({ signal }) => formulaApi.catalog(recordType, signal),
    staleTime: 60_000,
  })
}

export function useFormulaPreview(request: FormulaPreviewRequest | null) {
  return useQuery({
    queryKey: formulaKeys.preview(request ?? { record_type: '', expression: '', result_type: 'number' }),
    queryFn: ({ signal }) => formulaApi.preview(request!, signal),
    enabled: request !== null,
    placeholderData: keepPreviousData,
    retry: false,
    staleTime: 10_000,
  })
}
