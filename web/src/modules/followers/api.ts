import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { FollowedRecordsPage, RecordFollowers } from './types'

export const FOLLOWED_PER_PAGE = 25

export const followersKeys = {
  all: ['record-followers'] as const,
  record: (recordType: string, recordId: string) => [...followersKeys.all, 'record', recordType, recordId] as const,
  followed: (recordType: string, page: number) => [...followersKeys.all, 'followed', recordType, page] as const,
}

const id = encodeURIComponent

function recordPath(recordType: string, recordId: string, suffix: string) {
  return `/v1/records/${id(recordType)}/${id(recordId)}${suffix}`
}

export const followersApi = {
  list: (recordType: string, recordId: string) =>
    api.get<RecordFollowers>(recordPath(recordType, recordId, '/followers')),
  follow: (recordType: string, recordId: string) =>
    api.post<RecordFollowers>(recordPath(recordType, recordId, '/follow')),
  unfollow: (recordType: string, recordId: string) =>
    api.delete<RecordFollowers>(recordPath(recordType, recordId, '/follow')),
  followed: (recordType: string, page: number) =>
    api.get<FollowedRecordsPage>('/v1/me/followed-records', {
      query: { page, per_page: FOLLOWED_PER_PAGE, record_type: recordType || undefined },
    }),
}

export function useRecordFollowers(recordType: string, recordId: string) {
  return useQuery({
    queryKey: followersKeys.record(recordType, recordId),
    queryFn: () => followersApi.list(recordType, recordId),
    enabled: recordId !== '',
    retry: false,
  })
}

export function useFollowedRecords(recordType: string, page: number) {
  return useQuery({
    queryKey: followersKeys.followed(recordType, page),
    queryFn: () => followersApi.followed(recordType, page),
  })
}

export function useFollowToggle(recordType: string, recordId: string) {
  const queryClient = useQueryClient()
  const key = followersKeys.record(recordType, recordId)
  return useMutation({
    mutationFn: (follow: boolean) =>
      follow ? followersApi.follow(recordType, recordId) : followersApi.unfollow(recordType, recordId),
    onSuccess: (data) => queryClient.setQueryData(key, data),
    onSettled: () => queryClient.invalidateQueries({ queryKey: [...followersKeys.all, 'followed'] }),
  })
}
