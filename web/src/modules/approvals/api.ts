import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import { recordsApi } from '@/features/records/api'
import { APPROVALS_PER_PAGE, PENDING_COUNT_REFETCH_MS, POLICIES_PER_PAGE, QUOTE_RECORD_TYPE } from './constants'
import type {
  ApprovalBox,
  ApprovalPendingCount,
  ApprovalPolicy,
  ApprovalPolicyRequest,
  ApprovalRequest,
  ApprovalStatus,
  ApprovalSubmission,
  ApprovalSubmissionRequest,
  Page,
} from './types'

const id = encodeURIComponent

export interface ApprovalInboxFilter {
  box: ApprovalBox
  status?: ApprovalStatus
  page: number
}

export type ApprovalDecisionAction = 'approve' | 'reject' | 'cancel'

export const approvalsKeys = {
  all: ['approvals'] as const,
  policies: () => [...approvalsKeys.all, 'policies'] as const,
  policy: (policyId: string) => [...approvalsKeys.policies(), 'detail', policyId] as const,
  requests: () => [...approvalsKeys.all, 'requests'] as const,
  inbox: (filter: ApprovalInboxFilter) => [...approvalsKeys.requests(), 'inbox', filter] as const,
  record: (recordType: string, recordId: string) =>
    [...approvalsKeys.requests(), 'record', recordType, recordId] as const,
  request: (requestId: string) => [...approvalsKeys.requests(), 'detail', requestId] as const,
  pendingCount: () => [...approvalsKeys.requests(), 'pending-count'] as const,
  quoteAvailable: () => [...approvalsKeys.all, 'quote-available'] as const,
}

function ifMatch(version?: string | null): HeadersInit | undefined {
  return version ? { 'If-Match': version } : undefined
}

export const approvalsApi = {
  listPolicies: () =>
    api.get<Page<ApprovalPolicy>>('/v1/approval-policies', {
      query: { per_page: POLICIES_PER_PAGE },
    }),
  getPolicy: (policyId: string) => api.get<ApprovalPolicy>(`/v1/approval-policies/${id(policyId)}`),
  createPolicy: (request: ApprovalPolicyRequest) => api.post<ApprovalPolicy>('/v1/approval-policies', request),
  updatePolicy: (policyId: string, request: ApprovalPolicyRequest, version?: string | null) =>
    api.patch<ApprovalPolicy>(`/v1/approval-policies/${id(policyId)}`, request, { headers: ifMatch(version) }),
  removePolicy: (policyId: string) => api.delete<void>(`/v1/approval-policies/${id(policyId)}`),
  reorderPolicies: (ids: string[]) => api.put<ApprovalPolicy[]>('/v1/approval-policies/order', { ids }),
  inbox: (filter: ApprovalInboxFilter) =>
    api.get<Page<ApprovalRequest>>('/v1/approval-requests', {
      query: {
        box: filter.box,
        status: filter.status,
        page: filter.page,
        per_page: APPROVALS_PER_PAGE,
      },
    }),
  forRecord: (recordType: string, recordId: string) =>
    api.get<Page<ApprovalRequest>>('/v1/approval-requests', {
      query: { record_type: recordType, record_id: recordId, per_page: 100 },
    }),
  pendingCount: () => api.get<ApprovalPendingCount>('/v1/approval-requests/pending-count'),
  getRequest: (requestId: string) => api.get<ApprovalRequest>(`/v1/approval-requests/${id(requestId)}`),
  submit: (request: ApprovalSubmissionRequest) => api.post<ApprovalSubmission>('/v1/approval-requests', request),
  decide: (requestId: string, action: ApprovalDecisionAction, comment: string | null) =>
    api.post<ApprovalRequest>(`/v1/approval-requests/${id(requestId)}/${action}`, comment ? { comment } : {}),
}

export function useApprovalPolicies(enabled = true) {
  return useQuery({
    queryKey: approvalsKeys.policies(),
    queryFn: async () => (await approvalsApi.listPolicies()).data,
    enabled,
  })
}

export function useApprovalInbox(filter: ApprovalInboxFilter) {
  return useQuery({
    queryKey: approvalsKeys.inbox(filter),
    queryFn: () => approvalsApi.inbox(filter),
    placeholderData: keepPreviousData,
  })
}

export function useRecordApprovalRequests(recordType: string, recordId: string) {
  return useQuery({
    queryKey: approvalsKeys.record(recordType, recordId),
    queryFn: async () => (await approvalsApi.forRecord(recordType, recordId)).data,
    enabled: Boolean(recordType && recordId),
  })
}

export function useApprovalRequest(requestId: string | undefined) {
  return useQuery({
    queryKey: approvalsKeys.request(requestId ?? ''),
    queryFn: () => approvalsApi.getRequest(requestId ?? ''),
    enabled: Boolean(requestId),
  })
}

export function usePendingApprovalsCount() {
  return useQuery({
    queryKey: approvalsKeys.pendingCount(),
    queryFn: async () => (await approvalsApi.pendingCount()).count,
    refetchInterval: PENDING_COUNT_REFETCH_MS,
    staleTime: PENDING_COUNT_REFETCH_MS / 2,
    retry: false,
  })
}

export function useQuoteRecordTypeAvailable(enabled = true) {
  return useQuery({
    queryKey: approvalsKeys.quoteAvailable(),
    queryFn: async () => {
      try {
        await recordsApi.schema(QUOTE_RECORD_TYPE)
        return true
      } catch {
        return false
      }
    },
    enabled,
    staleTime: 5 * 60_000,
  })
}

export function useInvalidateApprovals() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: approvalsKeys.all })
}

export function useApprovalDecision() {
  const invalidate = useInvalidateApprovals()
  return useMutation({
    mutationFn: ({
      requestId,
      action,
      comment,
    }: {
      requestId: string
      action: ApprovalDecisionAction
      comment: string | null
    }) => approvalsApi.decide(requestId, action, comment),
    onSuccess: () => invalidate(),
  })
}

export function useSubmitApproval() {
  const invalidate = useInvalidateApprovals()
  return useMutation({
    mutationFn: (request: ApprovalSubmissionRequest) => approvalsApi.submit(request),
    onSuccess: () => invalidate(),
  })
}

export function usePolicyMutation<TVariables, TResult>(mutationFn: (variables: TVariables) => Promise<TResult>) {
  const invalidate = useInvalidateApprovals()
  return useMutation({ mutationFn, onSuccess: () => invalidate() })
}
