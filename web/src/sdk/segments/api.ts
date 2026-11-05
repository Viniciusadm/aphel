import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
import { type FilterNode } from '@/sdk'
type Schemas = components['schemas']

export type Segment = Schemas['SegmentResponse']
export type SegmentsPage = Schemas['Page_SegmentResponse']
export type SegmentMember = Schemas['SegmentMemberResponse']
export type SegmentMembersPage = Schemas['Page_SegmentMemberResponse']

export const SEGMENT_RECORD_TYPES = ['people.person', 'organizations.organization', 'deals.deal'] as const
export type SegmentRecordType = (typeof SEGMENT_RECORD_TYPES)[number]
export const SEGMENTS_PER_PAGE = 100
export const MEMBERS_PER_PAGE = 25
export const MAX_SEGMENT_NAME_LENGTH = 140
export const MAX_SEGMENT_DESCRIPTION_LENGTH = 500

const READ_PERMISSION: Record<SegmentRecordType, string> = {
  'people.person': 'people.read',
  'organizations.organization': 'organizations.read',
  'deals.deal': 'deals.read',
}

export function segmentReadPermission(recordType: SegmentRecordType): string {
  return READ_PERMISSION[recordType]
}

export function isSegmentRecordType(value: string | null | undefined): value is SegmentRecordType {
  return Boolean(value) && (SEGMENT_RECORD_TYPES as readonly string[]).includes(value as string)
}

export function segmentCondition(segmentId: string): FilterNode {
  return { field: 'segment', operator: 'in_saved_filter', value: segmentId }
}

export interface SegmentBody {
  record_type?: string
  name: string
  description?: string | null
  filter: FilterNode
}

export const segmentsKeys = {
  all: ['segments'] as const,
  list: (recordType: string) => [...segmentsKeys.all, 'list', recordType] as const,
  detail: (id: string) => [...segmentsKeys.all, 'detail', id] as const,
  members: (id: string, page: number) => [...segmentsKeys.all, 'members', id, page] as const,
}

function segmentPath(id: string, suffix = '') {
  return `/v1/segments/${encodeURIComponent(id)}${suffix}`
}

export const segmentsApi = {
  list: (recordType: string) =>
    api.get<SegmentsPage>('/v1/segments', {
      query: { record_type: recordType, counts: true, per_page: SEGMENTS_PER_PAGE },
    }),
  get: (id: string) => api.get<Segment>(segmentPath(id)),
  create: (body: SegmentBody) => api.post<Segment>('/v1/segments', body),
  update: (id: string, body: Omit<SegmentBody, 'record_type'>) => api.patch<Segment>(segmentPath(id), body),
  remove: (id: string) => api.delete<void>(segmentPath(id)),
  members: (id: string, page: number) =>
    api.get<SegmentMembersPage>(segmentPath(id, '/members'), { query: { page, per_page: MEMBERS_PER_PAGE } }),
}

export function useSegments(recordType: string, options: { enabled?: boolean } = {}) {
  return useQuery({
    queryKey: segmentsKeys.list(recordType),
    queryFn: async () => (await segmentsApi.list(recordType)).data,
    enabled: options.enabled ?? true,
  })
}

export function useSegmentOptions(recordType: string) {
  return useQuery({
    queryKey: [...segmentsKeys.all, 'options', recordType] as const,
    queryFn: async () =>
      (await api.get<SegmentsPage>('/v1/segments', { query: { record_type: recordType, per_page: SEGMENTS_PER_PAGE } }))
        .data,
    enabled: isSegmentRecordType(recordType),
    staleTime: 60_000,
  })
}

export function useSegment(id: string) {
  return useQuery({ queryKey: segmentsKeys.detail(id), queryFn: () => segmentsApi.get(id), enabled: Boolean(id) })
}

export function useSegmentMembers(id: string, page: number) {
  return useQuery({
    queryKey: segmentsKeys.members(id, page),
    queryFn: () => segmentsApi.members(id, page),
    enabled: Boolean(id),
    placeholderData: keepPreviousData,
  })
}

export function useInvalidateSegments() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: segmentsKeys.all })
}
