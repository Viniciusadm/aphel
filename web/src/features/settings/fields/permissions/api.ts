import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api, type components } from '@/api'
import { ifMatchHeaders } from '../../api'
import { fieldsKeys } from '../api'

export type FieldAccess = components['schemas']['FieldAccess']
export type FieldPermissionMatrix = components['schemas']['FieldPermissionMatrixResponse']
export type FieldPermissionField = components['schemas']['FieldPermissionFieldResponse']
export type FieldPermissionRole = components['schemas']['FieldPermissionRoleResponse']
export type FieldPermissionEntry = components['schemas']['FieldPermissionEntryRequest']

export const FIELD_ACCESS_LEVELS = ['hidden', 'read_only', 'editable'] as const satisfies readonly FieldAccess[]

export const fieldPermissionsKeys = {
  all: ['field-permissions'] as const,
  recordType: (recordType: string) => [...fieldPermissionsKeys.all, recordType] as const,
}

function permissionsPath(recordType: string) {
  return `/v1/field-permissions/${encodeURIComponent(recordType)}`
}

export const fieldPermissionsApi = {
  get: (recordType: string) => api.get<FieldPermissionMatrix>(permissionsPath(recordType)),
  save: (recordType: string, entries: FieldPermissionEntry[], version?: string | null) =>
    api.put<FieldPermissionMatrix>(permissionsPath(recordType), { entries }, { headers: ifMatchHeaders(version) }),
}

export function useFieldPermissions(recordType: string) {
  return useQuery({
    queryKey: fieldPermissionsKeys.recordType(recordType),
    queryFn: () => fieldPermissionsApi.get(recordType),
  })
}

export function useSaveFieldPermissions(recordType: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ entries, version }: { entries: FieldPermissionEntry[]; version?: string | null }) =>
      fieldPermissionsApi.save(recordType, entries, version),
    onSuccess: (matrix) => {
      queryClient.setQueryData(fieldPermissionsKeys.recordType(recordType), matrix)
      void queryClient.invalidateQueries({ queryKey: fieldsKeys.recordType(recordType) })
      void queryClient.invalidateQueries({ queryKey: ['records'] })
    },
  })
}
