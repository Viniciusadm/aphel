import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/modules/duplicates/api-types'
import { recordsKeys } from '@/sdk'
type Schemas = components['schemas']

export type DuplicateCandidate = Schemas['DuplicateCandidateResponse']
export type DuplicateCandidatesPage = Schemas['Page_DuplicateCandidateResponse']
export type DuplicateRules = Schemas['DuplicateRulesResponse']
export type DuplicateMatch = Schemas['DuplicateMatchResponse']
export type MergeRequest = Schemas['MergeRequest']
export type MergeResponse = Schemas['MergeResponse']
export type DuplicateScanStatus = NonNullable<Schemas['Data_Option_ScanStatusResponse']['data']>

export type DuplicateRecordType = string

export interface DuplicateTypeInfo {
  record_type: string
  label?: string | null
  custom: boolean
  rules: { rule: string; enabled: boolean }[]
  merge_fields: string[]
  permissions: { read: string; update: string; delete: string }
}

export const CANDIDATE_STATUSES = ['pending', 'dismissed', 'merged'] as const
export type CandidateStatus = (typeof CANDIDATE_STATUSES)[number]

export const DUPLICATES_PER_PAGE = 25

export function isCandidateStatus(value: string | null): value is CandidateStatus {
  return CANDIDATE_STATUSES.includes(value as CandidateStatus)
}

export const duplicatesKeys = {
  all: ['duplicates'] as const,
  types: () => [...duplicatesKeys.all, 'types'] as const,
  list: (recordType: DuplicateRecordType, status: CandidateStatus, page: number) =>
    [...duplicatesKeys.all, 'list', recordType, status, page] as const,
  rules: (recordType: DuplicateRecordType) => [...duplicatesKeys.all, 'rules', recordType] as const,
  scan: () => [...duplicatesKeys.all, 'scan'] as const,
}

export const duplicatesApi = {
  types: () => api.get<DuplicateTypeInfo[]>('/v1/duplicates/types'),
  list: (recordType: DuplicateRecordType, status: CandidateStatus, page: number) =>
    api.get<DuplicateCandidatesPage>('/v1/duplicates', {
      query: { record_type: recordType, status, page, per_page: DUPLICATES_PER_PAGE },
    }),
  dismiss: (candidateId: string) =>
    api.post<DuplicateCandidate>(`/v1/duplicates/${encodeURIComponent(candidateId)}/dismiss`),
  merge: ({ version, ...request }: MergeRequest & { version?: string | null }) =>
    api.post<MergeResponse>('/v1/duplicates/merge', request, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
  rules: (recordType: DuplicateRecordType) =>
    api.get<DuplicateRules>('/v1/duplicates/rules', { query: { record_type: recordType } }),
  updateRules: (recordType: DuplicateRecordType, rules: Record<string, boolean>) =>
    api.put<DuplicateRules>('/v1/duplicates/rules', { record_type: recordType, rules }),
  scan: () => api.post<void>('/v1/duplicates/scan'),
  scanStatus: () => api.get<DuplicateScanStatus | null>('/v1/duplicates/scan'),
}

export function useDuplicateTypes() {
  return useQuery({ queryKey: duplicatesKeys.types(), queryFn: duplicatesApi.types, staleTime: 60_000 })
}

export function useDuplicateCandidates(recordType: DuplicateRecordType, status: CandidateStatus, page: number) {
  return useQuery({
    queryKey: duplicatesKeys.list(recordType, status, page),
    queryFn: () => duplicatesApi.list(recordType, status, page),
    placeholderData: keepPreviousData,
  })
}

export function useDismissDuplicate() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: duplicatesApi.dismiss,
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: duplicatesKeys.all }),
  })
}

export function useMergeRecords() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: duplicatesApi.merge,
    onSuccess: (result) => {
      void queryClient.invalidateQueries({ queryKey: duplicatesKeys.all })
      void queryClient.invalidateQueries({ queryKey: [...recordsKeys.all, result.record_type] })
    },
  })
}

export function useDuplicateRules(recordType: DuplicateRecordType, enabled = true) {
  return useQuery({
    queryKey: duplicatesKeys.rules(recordType),
    queryFn: () => duplicatesApi.rules(recordType),
    enabled,
  })
}

export function useUpdateDuplicateRules(recordType: DuplicateRecordType) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (rules: Record<string, boolean>) => duplicatesApi.updateRules(recordType, rules),
    onSuccess: (result) => {
      queryClient.setQueryData(duplicatesKeys.rules(recordType), result)
      void queryClient.invalidateQueries({ queryKey: duplicatesKeys.types() })
    },
  })
}

const SCAN_POLL_MS = 2000

export function isScanActive(status: DuplicateScanStatus | null | undefined): boolean {
  return status?.status === 'queued' || status?.status === 'running'
}

export function useDuplicateScanStatus() {
  return useQuery({
    queryKey: duplicatesKeys.scan(),
    queryFn: duplicatesApi.scanStatus,
    refetchInterval: (query) => (isScanActive(query.state.data) ? SCAN_POLL_MS : false),
  })
}

export function useRequestDuplicateScan() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: duplicatesApi.scan,
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: duplicatesKeys.scan() }),
  })
}
