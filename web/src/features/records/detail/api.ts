import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import { notifyError } from '@/components/feedback/notify'
import { recordsKeys } from '../api'
import { toRecordPayload, type RecordPayload } from '../fields'
import { recordPath, requireRecordResource, type RecordResource } from '../resources'
import type { FieldDefinition, RecordItem } from '../types'

export const recordDetailApi = {
  get: (resource: RecordResource, id: string) => api.get<RecordItem>(recordPath(resource, id)),
  update: (resource: RecordResource, id: string, payload: RecordPayload) =>
    api.patch<RecordItem>(recordPath(resource, id), payload),
  remove: (resource: RecordResource, id: string) => api.delete<void>(recordPath(resource, id)),
}

export function useRecord(recordType: string, id: string) {
  const resource = requireRecordResource(recordType)
  return useQuery({
    queryKey: recordsKeys.detail(recordType, id),
    queryFn: () => recordDetailApi.get(resource, id),
  })
}

export function useUpdateRecord(recordType: string, id: string) {
  const resource = requireRecordResource(recordType)
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (payload: RecordPayload) => recordDetailApi.update(resource, id, payload),
    onSuccess: (record) => {
      queryClient.setQueryData(recordsKeys.detail(recordType, id), record)
      void queryClient.invalidateQueries({ queryKey: recordsKeys.list(recordType) })
    },
  })
}

export function useSaveRecordField(recordType: string, id: string) {
  const update = useUpdateRecord(recordType, id)
  const { mutateAsync } = update
  const saveField = async (field: FieldDefinition, value: unknown) => {
    try {
      await mutateAsync(toRecordPayload([field], { [field.key]: value }))
    } catch (error) {
      notifyError(error)
      throw error
    }
  }
  return { saveField, isSaving: update.isPending }
}
