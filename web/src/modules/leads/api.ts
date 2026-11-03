import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import { recordsKeys } from '@/features/records/api'

import type {
  ConvertLeadRequest,
  CreateDisqualificationReasonRequest,
  DisqualificationReason,
  DisqualifyLeadRequest,
  LeadActionResult,
  LifecycleHistoryEntry,
  LifecycleRecordType,
  LifecycleReport,
  LifecycleReportQuery,
  LifecycleSettings,
  LifecycleSettingsRequest,
  UpdateDisqualificationReasonRequest,
} from './types'

const id = encodeURIComponent
const CONTACT: LifecycleRecordType = 'people.person'
const REASONS_PATH = '/v1/lead-disqualification-reasons'

export const lifecycleKeys = {
  all: ['lifecycle'] as const,
  settings: () => [...lifecycleKeys.all, 'settings'] as const,
  reasons: (includeInactive: boolean) => [...lifecycleKeys.all, 'reasons', includeInactive] as const,
  history: (recordType: string, recordId: string) => [...lifecycleKeys.all, 'history', recordType, recordId] as const,
  report: (query: LifecycleReportQuery) => [...lifecycleKeys.all, 'report', query] as const,
}

function versionHeader(version: string | null | undefined) {
  return version ? { 'If-Match': version } : undefined
}

export const lifecycleApi = {
  settings: () => api.get<LifecycleSettings>('/v1/lifecycle/settings'),
  saveSettings: (body: LifecycleSettingsRequest, version?: string | null) =>
    api.put<LifecycleSettings>('/v1/lifecycle/settings', body, { headers: versionHeader(version) }),
  reasons: (includeInactive: boolean) =>
    api.get<DisqualificationReason[]>(REASONS_PATH, { query: { include_inactive: includeInactive } }),
  createReason: (body: CreateDisqualificationReasonRequest) => api.post<DisqualificationReason>(REASONS_PATH, body),
  updateReason: (reasonId: string, body: UpdateDisqualificationReasonRequest, version?: string) =>
    api.patch<DisqualificationReason>(`${REASONS_PATH}/${id(reasonId)}`, body, { headers: versionHeader(version) }),
  deleteReason: (reasonId: string) => api.delete<void>(`${REASONS_PATH}/${id(reasonId)}`),
  reorderReasons: (ids: string[]) => api.put<DisqualificationReason[]>(`${REASONS_PATH}/order`, { ids }),
  history: (recordType: LifecycleRecordType, recordId: string) =>
    api.get<LifecycleHistoryEntry[]>(`/v1/lifecycle/history/${id(recordType)}/${id(recordId)}`),
  report: (query: LifecycleReportQuery) =>
    api.get<LifecycleReport>('/v1/lifecycle/report', {
      query: { record_type: query.record_type, from: query.from || undefined, to: query.to || undefined },
    }),
  qualify: (contactId: string) => api.post<LeadActionResult>(`/v1/leads/${id(contactId)}/qualify`, {}),
  disqualify: (contactId: string, body: DisqualifyLeadRequest) =>
    api.post<LeadActionResult>(`/v1/leads/${id(contactId)}/disqualify`, body),
  convert: (contactId: string, body: ConvertLeadRequest) =>
    api.post<LeadActionResult>(`/v1/leads/${id(contactId)}/convert`, body),
}

export function useLifecycleSettings(enabled = true) {
  return useQuery({
    queryKey: lifecycleKeys.settings(),
    queryFn: lifecycleApi.settings,
    enabled,
  })
}

export function useDisqualificationReasons(includeInactive = false) {
  return useQuery({
    queryKey: lifecycleKeys.reasons(includeInactive),
    queryFn: () => lifecycleApi.reasons(includeInactive),
    staleTime: 60_000,
  })
}

export function useInvalidateDisqualificationReasons() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: [...lifecycleKeys.all, 'reasons'] })
}

export function useLifecycleHistory(recordType: LifecycleRecordType, recordId: string) {
  return useQuery({
    queryKey: lifecycleKeys.history(recordType, recordId),
    queryFn: () => lifecycleApi.history(recordType, recordId),
    enabled: recordId !== '',
  })
}

export function useLifecycleReport(query: LifecycleReportQuery) {
  return useQuery({
    queryKey: lifecycleKeys.report(query),
    queryFn: () => lifecycleApi.report(query),
  })
}

export function useLeadActions() {
  const queryClient = useQueryClient()
  const refresh = async (result: LeadActionResult) => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: [...recordsKeys.all, CONTACT] }),
      queryClient.invalidateQueries({ queryKey: [...recordsKeys.all, 'deal'] }),
      queryClient.invalidateQueries({ queryKey: lifecycleKeys.history(CONTACT, result.contact_id) }),
    ])
  }
  return {
    qualify: useMutation({ mutationFn: (contactId: string) => lifecycleApi.qualify(contactId), onSuccess: refresh }),
    disqualify: useMutation({
      mutationFn: ({ contactId, body }: { contactId: string; body: DisqualifyLeadRequest }) =>
        lifecycleApi.disqualify(contactId, body),
      onSuccess: refresh,
    }),
    convert: useMutation({
      mutationFn: ({ contactId, body }: { contactId: string; body: ConvertLeadRequest }) =>
        lifecycleApi.convert(contactId, body),
      onSuccess: refresh,
    }),
  }
}
