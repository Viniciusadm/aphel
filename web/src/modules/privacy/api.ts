import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { API_BASE_URL, api, type components } from '@/api'

type Schemas = components['schemas']

export type SubjectSearch = Schemas['SubjectSearchResponse']
export type SubjectRecord = Schemas['SubjectRecordResponse']
export type SubjectNote = Schemas['SubjectNoteResponse']
export type SubjectAttachment = Schemas['SubjectAttachmentResponse']
export type SubjectActivity = Schemas['SubjectActivityResponse']
export type SubjectImportRow = Schemas['SubjectImportRowResponse']
export type PrivacyRequest = Schemas['PrivacyRequestResponse']
export type PrivacyRequestDetail = Schemas['PrivacyRequestDetailResponse']
export type PrivacyRequestsPage = Schemas['Page_PrivacyRequestResponse']
export type CreatePrivacyRequestBody = Schemas['CreatePrivacyRequestBody']
export type RetentionPolicy = Schemas['RetentionPolicyResponse']
export type RetentionPreview = Schemas['RetentionPreviewResponse']
export type RetentionRun = Schemas['RetentionRunResponse']
export type RetentionRunsPage = Schemas['Page_RetentionRunResponse']
export type CreateRetentionPolicyBody = Schemas['CreateRetentionPolicyBody']
export type UpdateRetentionPolicyBody = Schemas['UpdateRetentionPolicyBody']

export const REQUEST_KINDS = ['export', 'anonymize', 'delete'] as const
export type RequestKind = (typeof REQUEST_KINDS)[number]

export const REQUEST_STATUSES = ['pending', 'processing', 'completed', 'failed', 'cancelled'] as const
export type RequestStatus = (typeof REQUEST_STATUSES)[number]

export const RETENTION_ACTIONS = [
  'archive_inactive',
  'purge_archived',
  'purge_audit_logs',
  'purge_security_events',
  'purge_domain_events',
  'purge_orphan_files',
] as const
export type RetentionAction = (typeof RETENTION_ACTIONS)[number]

export const RETENTION_RECORD_TYPES = ['people.person', 'organizations.organization', 'deals.deal'] as const
export const TARGET_RECORD_TYPES = ['people.person', 'organizations.organization', 'deals.deal', 'tasks.task'] as const
export type TargetRecordType = (typeof TARGET_RECORD_TYPES)[number]

export const REQUESTS_PER_PAGE = 20
export const RUNS_PER_PAGE = 20
export const MIN_THRESHOLD_DAYS = 1
export const MAX_THRESHOLD_DAYS = 36500
export const DEFAULT_DUE_DAYS = 15

export function isRequestKind(value: string): value is RequestKind {
  return REQUEST_KINDS.includes(value as RequestKind)
}

export function isRequestStatus(value: string | null): value is RequestStatus {
  return REQUEST_STATUSES.includes(value as RequestStatus)
}

export function isRetentionAction(value: string): value is RetentionAction {
  return RETENTION_ACTIONS.includes(value as RetentionAction)
}

export function isTargetRecordType(value: string): value is TargetRecordType {
  return TARGET_RECORD_TYPES.includes(value as TargetRecordType)
}

export function needsRecordType(action: RetentionAction): boolean {
  return action === 'archive_inactive' || action === 'purge_archived'
}

export const privacyKeys = {
  all: ['privacy'] as const,
  requests: () => [...privacyKeys.all, 'requests'] as const,
  requestsPage: (status: RequestStatus | undefined, page: number) =>
    [...privacyKeys.requests(), 'list', status ?? 'all', page] as const,
  request: (id: string) => [...privacyKeys.requests(), 'detail', id] as const,
  policies: () => [...privacyKeys.all, 'policies'] as const,
  runs: () => [...privacyKeys.all, 'runs'] as const,
  runsPage: (policyId: string | undefined, page: number) =>
    [...privacyKeys.runs(), policyId ?? 'all', page] as const,
}

function requestPath(id: string) {
  return `/v1/privacy/requests/${encodeURIComponent(id)}`
}

function policyPath(id: string) {
  return `/v1/privacy/retention-policies/${encodeURIComponent(id)}`
}

export function requestDownloadUrl(id: string): string {
  return `${API_BASE_URL}${requestPath(id)}/download`
}

export const privacyApi = {
  search: (term: string) => api.post<SubjectSearch>('/v1/privacy/subjects/search', { term }),
  requests: (status: RequestStatus | undefined, page: number) =>
    api.get<PrivacyRequestsPage>('/v1/privacy/requests', {
      query: { status, page, per_page: REQUESTS_PER_PAGE },
    }),
  request: (id: string) => api.get<PrivacyRequestDetail>(requestPath(id)),
  createRequest: (body: CreatePrivacyRequestBody) => api.post<PrivacyRequest>('/v1/privacy/requests', body),
  executeRequest: (id: string) => api.post<PrivacyRequest>(`${requestPath(id)}/execute`),
  cancelRequest: (id: string) => api.post<PrivacyRequest>(`${requestPath(id)}/cancel`),
  policies: () => api.get<RetentionPolicy[]>('/v1/privacy/retention-policies'),
  createPolicy: (body: CreateRetentionPolicyBody) =>
    api.post<RetentionPolicy>('/v1/privacy/retention-policies', body),
  updatePolicy: (id: string, body: UpdateRetentionPolicyBody) => api.patch<RetentionPolicy>(policyPath(id), body),
  deletePolicy: (id: string) => api.delete<void>(policyPath(id)),
  previewPolicy: (id: string) => api.post<RetentionPreview>(`${policyPath(id)}/preview`),
  enablePolicy: (id: string) => api.post<RetentionPolicy>(`${policyPath(id)}/enable`),
  disablePolicy: (id: string) => api.post<RetentionPolicy>(`${policyPath(id)}/disable`),
  runPolicy: (id: string) => api.post<RetentionRun>(`${policyPath(id)}/run`),
  runs: (policyId: string | undefined, page: number) =>
    api.get<RetentionRunsPage>('/v1/privacy/retention-runs', {
      query: { policy_id: policyId, page, per_page: RUNS_PER_PAGE },
    }),
}

export function useSubjectSearch() {
  return useMutation({ mutationFn: privacyApi.search })
}

export function usePrivacyRequests(status: RequestStatus | undefined, page: number) {
  return useQuery({
    queryKey: privacyKeys.requestsPage(status, page),
    queryFn: () => privacyApi.requests(status, page),
    placeholderData: keepPreviousData,
    refetchInterval: (query) =>
      query.state.data?.data.some((request) => request.status === 'processing') ? 5000 : false,
  })
}

export function usePrivacyRequest(id: string | null) {
  return useQuery({
    queryKey: privacyKeys.request(id ?? ''),
    queryFn: () => privacyApi.request(id ?? ''),
    enabled: id !== null,
    refetchInterval: (query) => (query.state.data?.request.status === 'processing' ? 5000 : false),
  })
}

export function useRequestMutations() {
  const queryClient = useQueryClient()
  const refresh = () => queryClient.invalidateQueries({ queryKey: privacyKeys.requests() })
  return {
    create: useMutation({ mutationFn: privacyApi.createRequest, onSuccess: refresh }),
    execute: useMutation({ mutationFn: privacyApi.executeRequest, onSuccess: refresh }),
    cancel: useMutation({ mutationFn: privacyApi.cancelRequest, onSuccess: refresh }),
  }
}

export function useRetentionPolicies() {
  return useQuery({ queryKey: privacyKeys.policies(), queryFn: privacyApi.policies })
}

export function useRetentionRuns(policyId: string | undefined, page: number) {
  return useQuery({
    queryKey: privacyKeys.runsPage(policyId, page),
    queryFn: () => privacyApi.runs(policyId, page),
    placeholderData: keepPreviousData,
    refetchInterval: (query) =>
      query.state.data?.data.some((run) => run.status === 'queued' || run.status === 'running') ? 5000 : false,
  })
}

export function usePolicyMutations() {
  const queryClient = useQueryClient()
  const refresh = () => queryClient.invalidateQueries({ queryKey: privacyKeys.policies() })
  const refreshAll = () => queryClient.invalidateQueries({ queryKey: privacyKeys.all })
  return {
    create: useMutation({ mutationFn: privacyApi.createPolicy, onSuccess: refresh }),
    update: useMutation({
      mutationFn: ({ id, body }: { id: string; body: UpdateRetentionPolicyBody }) => privacyApi.updatePolicy(id, body),
      onSuccess: refresh,
    }),
    remove: useMutation({ mutationFn: privacyApi.deletePolicy, onSuccess: refresh }),
    preview: useMutation({ mutationFn: privacyApi.previewPolicy, onSuccess: refresh }),
    enable: useMutation({ mutationFn: privacyApi.enablePolicy, onSuccess: refresh }),
    disable: useMutation({ mutationFn: privacyApi.disablePolicy, onSuccess: refresh }),
    run: useMutation({ mutationFn: privacyApi.runPolicy, onSuccess: refreshAll }),
  }
}
