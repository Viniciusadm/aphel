import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api, type components } from '@/api'
import { ifMatchHeaders } from '../api'

export type Field = components['schemas']['FieldResponse']
export type FieldOption = components['schemas']['FieldOptionResponse']
export type FieldsPage = components['schemas']['Page_FieldResponse']
export type CreateFieldRequest = components['schemas']['CreateFieldRequest']
export type UpdateFieldRequest = components['schemas']['UpdateFieldRequest']
export type FieldOptionRequest = components['schemas']['FieldOptionRequest']
export type UpdateOptionRequest = components['schemas']['UpdateOptionRequest']

export const FIELDS_PER_PAGE = 100

export const fieldsKeys = {
  all: ['fields'] as const,
  recordType: (recordType: string) => [...fieldsKeys.all, recordType] as const,
  admin: (recordType: string) => [...fieldsKeys.recordType(recordType), 'admin'] as const,
}

function fieldsPath(recordType: string, suffix = '') {
  return `/v1/fields/${encodeURIComponent(recordType)}${suffix}`
}

export const fieldsApi = {
  list: (recordType: string, page: number) =>
    api.get<FieldsPage>(fieldsPath(recordType), {
      query: { include_inactive: true, per_page: FIELDS_PER_PAGE, page },
    }),
  create: (recordType: string, request: CreateFieldRequest) => api.post<Field>(fieldsPath(recordType), request),
  update: (recordType: string, fieldId: string, request: UpdateFieldRequest, version?: string | null) =>
    api.patch<Field>(fieldsPath(recordType, `/${fieldId}`), request, { headers: ifMatchHeaders(version) }),
  deactivate: (recordType: string, fieldId: string) =>
    api.post<Field>(fieldsPath(recordType, `/${fieldId}/deactivate`)),
  activate: (recordType: string, fieldId: string) => api.post<Field>(fieldsPath(recordType, `/${fieldId}/activate`)),
  reorder: (recordType: string, ids: string[]) => api.put<Field[]>(fieldsPath(recordType, '/order'), { ids }),
  addOption: (recordType: string, fieldId: string, request: FieldOptionRequest) =>
    api.post<Field>(fieldsPath(recordType, `/${fieldId}/options`), request),
  updateOption: (
    recordType: string,
    fieldId: string,
    optionId: string,
    request: UpdateOptionRequest,
    version?: string | null,
  ) =>
    api.patch<Field>(fieldsPath(recordType, `/${fieldId}/options/${optionId}`), request, {
      headers: ifMatchHeaders(version),
    }),
  reorderOptions: (recordType: string, fieldId: string, ids: string[]) =>
    api.put<Field>(fieldsPath(recordType, `/${fieldId}/options/order`), { ids }),
}

export function sortByPosition<T extends { position: number }>(items: T[]): T[] {
  return [...items].sort((left, right) => left.position - right.position)
}

export async function listAllFields(recordType: string): Promise<Field[]> {
  const first = await fieldsApi.list(recordType, 1)
  const items = [...first.data]
  const perPage = first.meta.per_page > 0 ? first.meta.per_page : FIELDS_PER_PAGE
  const pages = Math.ceil(first.meta.total / perPage)
  for (let page = 2; page <= pages; page += 1) {
    const next = await fieldsApi.list(recordType, page)
    items.push(...next.data)
  }
  return sortByPosition(items)
}

export function useFields(recordType: string) {
  return useQuery({
    queryKey: fieldsKeys.admin(recordType),
    queryFn: () => listAllFields(recordType),
  })
}

export function useFieldsCache(recordType: string) {
  const queryClient = useQueryClient()
  const key = fieldsKeys.admin(recordType)
  return {
    replace: (field: Field) =>
      queryClient.setQueryData<Field[]>(key, (current) =>
        current ? sortByPosition(current.map((item) => (item.id === field.id ? field : item))) : current,
      ),
    append: (field: Field) =>
      queryClient.setQueryData<Field[]>(key, (current) =>
        current ? sortByPosition([...current.filter((item) => item.id !== field.id), field]) : [field],
      ),
    setAll: (fields: Field[]) => queryClient.setQueryData<Field[]>(key, sortByPosition(fields)),
  }
}

export function useFieldMutation<TVariables>(recordType: string, mutationFn: (variables: TVariables) => Promise<Field>) {
  const cache = useFieldsCache(recordType)
  return useMutation({
    mutationFn,
    onSuccess: (field) => cache.replace(field),
  })
}

export function useCreateField(recordType: string) {
  const cache = useFieldsCache(recordType)
  return useMutation({
    mutationFn: (request: CreateFieldRequest) => fieldsApi.create(recordType, request),
    onSuccess: (field) => cache.append(field),
  })
}

export function applyOrder(fields: Field[], ids: string[]): Field[] {
  const byId = new Map(fields.map((field) => [field.id, field]))
  return ids.flatMap((id, position) => {
    const field = byId.get(id)
    return field ? [{ ...field, position }] : []
  })
}

export function useReorderFields(recordType: string) {
  const queryClient = useQueryClient()
  const key = fieldsKeys.admin(recordType)
  return useMutation({
    mutationFn: (ids: string[]) => fieldsApi.reorder(recordType, ids),
    onMutate: async (ids) => {
      await queryClient.cancelQueries({ queryKey: key })
      const previous = queryClient.getQueryData<Field[]>(key)
      if (previous) queryClient.setQueryData<Field[]>(key, applyOrder(previous, ids))
      return { previous }
    },
    onError: (_error, _ids, context) => {
      if (context?.previous) queryClient.setQueryData<Field[]>(key, context.previous)
    },
    onSuccess: (fields) => queryClient.setQueryData<Field[]>(key, sortByPosition(fields)),
  })
}
