import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import { ENROLLMENTS_PER_PAGE, SEQUENCES_PER_PAGE } from './constants'
import type {
  EnrollRequest,
  Enrollment,
  EnrollmentStatus,
  Page,
  Sequence,
  SequenceMetrics,
  SequenceRequest,
} from './types'

const id = encodeURIComponent

export interface SequenceListFilter {
  page: number
  search?: string
  active?: boolean
}

export interface EnrollmentListFilter {
  page: number
  status?: EnrollmentStatus
}

export const sequencesKeys = {
  all: ['sequences'] as const,
  lists: () => [...sequencesKeys.all, 'list'] as const,
  list: (filter: SequenceListFilter) => [...sequencesKeys.lists(), filter] as const,
  active: () => [...sequencesKeys.all, 'active'] as const,
  detail: (sequenceId: string) => [...sequencesKeys.all, 'detail', sequenceId] as const,
  metrics: (sequenceId: string) => [...sequencesKeys.all, 'metrics', sequenceId] as const,
  enrollments: () => [...sequencesKeys.all, 'enrollments'] as const,
  sequenceEnrollments: (sequenceId: string, filter: EnrollmentListFilter) =>
    [...sequencesKeys.enrollments(), 'sequence', sequenceId, filter] as const,
  contactEnrollments: (contactId: string) => [...sequencesKeys.enrollments(), 'contact', contactId] as const,
}

function ifMatch(version?: string | null): HeadersInit | undefined {
  return version ? { 'If-Match': version } : undefined
}

export const sequencesApi = {
  list: (filter: SequenceListFilter) =>
    api.get<Page<Sequence>>('/v1/sequences', {
      query: { page: filter.page, per_page: SEQUENCES_PER_PAGE, search: filter.search, active: filter.active },
    }),
  get: (sequenceId: string) => api.get<Sequence>(`/v1/sequences/${id(sequenceId)}`),
  create: (request: SequenceRequest) => api.post<Sequence>('/v1/sequences', request),
  update: (sequenceId: string, request: SequenceRequest, version?: string | null) =>
    api.patch<Sequence>(`/v1/sequences/${id(sequenceId)}`, request, { headers: ifMatch(version) }),
  remove: (sequenceId: string) => api.delete<void>(`/v1/sequences/${id(sequenceId)}`),
  metrics: (sequenceId: string) => api.get<SequenceMetrics>(`/v1/sequences/${id(sequenceId)}/metrics`),
  sequenceEnrollments: (sequenceId: string, filter: EnrollmentListFilter) =>
    api.get<Page<Enrollment>>(`/v1/sequences/${id(sequenceId)}/enrollments`, {
      query: { page: filter.page, per_page: ENROLLMENTS_PER_PAGE, status: filter.status },
    }),
  contactEnrollments: (contactId: string) =>
    api.get<Page<Enrollment>>('/v1/sequence-enrollments', { query: { contact_id: contactId, per_page: 100 } }),
  enroll: (sequenceId: string, request: EnrollRequest) =>
    api.post<Enrollment>(`/v1/sequences/${id(sequenceId)}/enrollments`, request),
  pause: (enrollmentId: string) => api.post<Enrollment>(`/v1/sequence-enrollments/${id(enrollmentId)}/pause`),
  resume: (enrollmentId: string) => api.post<Enrollment>(`/v1/sequence-enrollments/${id(enrollmentId)}/resume`),
  exit: (enrollmentId: string) => api.post<Enrollment>(`/v1/sequence-enrollments/${id(enrollmentId)}/exit`),
}

export function useSequences(filter: SequenceListFilter) {
  return useQuery({
    queryKey: sequencesKeys.list(filter),
    queryFn: () => sequencesApi.list(filter),
    placeholderData: keepPreviousData,
  })
}

export function useActiveSequences(enabled = true) {
  return useQuery({
    queryKey: sequencesKeys.active(),
    queryFn: async () => {
      const page = await api.get<Page<Sequence>>('/v1/sequences', { query: { active: true, per_page: 100 } })
      return page.data
    },
    enabled,
    staleTime: 60_000,
  })
}

export function useSequence(sequenceId: string | undefined) {
  return useQuery({
    queryKey: sequencesKeys.detail(sequenceId ?? ''),
    queryFn: () => sequencesApi.get(sequenceId ?? ''),
    enabled: Boolean(sequenceId),
  })
}

export function useSequenceMetrics(sequenceId: string) {
  return useQuery({
    queryKey: sequencesKeys.metrics(sequenceId),
    queryFn: () => sequencesApi.metrics(sequenceId),
  })
}

export function useSequenceEnrollments(sequenceId: string, filter: EnrollmentListFilter) {
  return useQuery({
    queryKey: sequencesKeys.sequenceEnrollments(sequenceId, filter),
    queryFn: () => sequencesApi.sequenceEnrollments(sequenceId, filter),
    placeholderData: keepPreviousData,
  })
}

export function useContactEnrollments(contactId: string, enabled = true) {
  return useQuery({
    queryKey: sequencesKeys.contactEnrollments(contactId),
    queryFn: async () => (await sequencesApi.contactEnrollments(contactId)).data,
    enabled,
  })
}

export function useSequenceMutation<TVariables, TResult>(mutationFn: (variables: TVariables) => Promise<TResult>) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: sequencesKeys.all }),
  })
}

export type EnrollmentAction = 'pause' | 'resume' | 'exit'

export function useEnrollmentAction() {
  return useSequenceMutation(({ enrollmentId, action }: { enrollmentId: string; action: EnrollmentAction }) =>
    sequencesApi[action](enrollmentId),
  )
}
