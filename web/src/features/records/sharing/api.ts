import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { CreateRecordShareRequest, RecordShare, RecordShares, UpdateRecordShareRequest } from './types'

export const sharingKeys = {
  all: ['record-shares'] as const,
  record: (recordType: string, recordId: string) => [...sharingKeys.all, recordType, recordId] as const,
}

const id = encodeURIComponent

function sharesPath(recordType: string, recordId: string, suffix = '') {
  return `/v1/records/${id(recordType)}/${id(recordId)}/shares${suffix}`
}

export const sharingApi = {
  list: (recordType: string, recordId: string) => api.get<RecordShares>(sharesPath(recordType, recordId)),
  create: (recordType: string, recordId: string, body: CreateRecordShareRequest) =>
    api.post<RecordShare>(sharesPath(recordType, recordId), body),
  update: (recordType: string, recordId: string, shareId: string, body: UpdateRecordShareRequest, version?: string) =>
    api.patch<RecordShare>(sharesPath(recordType, recordId, `/${id(shareId)}`), body, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
  remove: (recordType: string, recordId: string, shareId: string) =>
    api.delete<void>(sharesPath(recordType, recordId, `/${id(shareId)}`)),
}

export function useRecordShares(recordType: string, recordId: string, enabled = true) {
  return useQuery({
    queryKey: sharingKeys.record(recordType, recordId),
    queryFn: () => sharingApi.list(recordType, recordId),
    enabled: enabled && recordId !== '',
    retry: false,
  })
}

export function useRecordShareMutations(recordType: string, recordId: string) {
  const queryClient = useQueryClient()
  const refresh = () => queryClient.invalidateQueries({ queryKey: sharingKeys.record(recordType, recordId) })
  return {
    create: useMutation({
      mutationFn: (body: CreateRecordShareRequest) => sharingApi.create(recordType, recordId, body),
      onSettled: refresh,
    }),
    update: useMutation({
      mutationFn: ({ share, access }: { share: RecordShare; access: UpdateRecordShareRequest['access'] }) =>
        sharingApi.update(recordType, recordId, share.id, { access }, share.updated_at),
      onSettled: refresh,
    }),
    remove: useMutation({
      mutationFn: (share: RecordShare) => sharingApi.remove(recordType, recordId, share.id),
      onSettled: refresh,
    }),
  }
}
