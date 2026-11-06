import { keepPreviousData, useInfiniteQuery, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
import { recordsKeys } from '@/sdk'
import { ACTIVITY_RECORD_TYPE, ACTIVITY_TYPE_RECORD_TYPE } from './record-types'

export type Activity = components['schemas']['ActivityResponse']
export type ActivityPage = components['schemas']['Page_ActivityResponse']
export type ActivityType = components['schemas']['ActivityTypeResponse']
export type ActivityTypePage = components['schemas']['Page_ActivityTypeResponse']
export type ActivityStatus = components['schemas']['ActivityStatus']
export type ActivityPriority = components['schemas']['ActivityPriority']
export type CreateActivityRequest = components['schemas']['CreateActivityRequest']
export type UpdateActivityRequest = components['schemas']['UpdateActivityRequest']
export type CreateActivityTypeRequest = components['schemas']['CreateActivityTypeRequest']
export type UpdateActivityTypeRequest = components['schemas']['UpdateActivityTypeRequest']
export type ActivityRecurrence = components['schemas']['ActivityRecurrenceResponse']
export type RecurrenceRuleRequest = components['schemas']['RecurrenceRuleRequest']
export type RecurrenceFrequency = components['schemas']['RecurrenceFrequency']
export type RecurrenceWeekday = components['schemas']['RecurrenceWeekday']
export type RecurrenceApplyTo = components['schemas']['RecurrenceApplyTo']
export type InboxResponse = components['schemas']['InboxResponse']
export type InboxGroup = components['schemas']['InboxGroupResponse']
export type InboxGroupKey = components['schemas']['InboxGroupKey']
export type InboxItem = components['schemas']['InboxItemResponse']
export type LinkedDealValue = components['schemas']['LinkedDealValueResponse']
export type InboxOwner = 'me' | 'team' | 'all'
export type InboxSort = 'due' | 'priority' | 'value'

export interface InboxQuery {
  owner: InboxOwner
  teamId?: string
  sort: InboxSort
}

export const ACTIVITY_PRIORITIES: ActivityPriority[] = ['low', 'normal', 'high', 'urgent']
export const RECORD_TASKS_PAGE_SIZE = 100
export const MY_TASKS_PAGE_SIZE = 100
export const ACTIVITY_TYPES_PAGE_SIZE = 100

export interface RecordTarget {
  recordType: string
  recordId: string
}

export const activitiesKeys = {
  all: ['tasks'] as const,
  types: (includeInactive: boolean) => [...activitiesKeys.all, 'types', includeInactive] as const,
  forRecord: (target: RecordTarget) =>
    [...activitiesKeys.all, 'record', target.recordType, target.recordId] as const,
  mine: () => [...activitiesKeys.all, 'mine'] as const,
  inbox: (query: InboxQuery) => [...activitiesKeys.all, 'inbox', query] as const,
}

const TASKS_PATH = '/v1/tasks'
const ACTIVITY_TYPES_PATH = '/v1/activity-types'

function activityPath(id: string, suffix = '') {
  return `${TASKS_PATH}/${encodeURIComponent(id)}${suffix}`
}

function activityTypePath(id: string, suffix = '') {
  return `${ACTIVITY_TYPES_PATH}/${encodeURIComponent(id)}${suffix}`
}

export const activitiesApi = {
  listForRecord: (target: RecordTarget) =>
    api.get<ActivityPage>(TASKS_PATH, {
      query: {
        record_type: target.recordType,
        record_id: target.recordId,
        sort: 'status,due_at',
        per_page: RECORD_TASKS_PAGE_SIZE,
      },
    }),
  listMine: (page = 1) =>
    api.get<ActivityPage>(TASKS_PATH, { query: { my_open: true, page, per_page: MY_TASKS_PAGE_SIZE } }),
  create: (request: CreateActivityRequest) => api.post<Activity>(TASKS_PATH, request),
  update: (id: string, request: UpdateActivityRequest, version?: string | null) =>
    api.patch<Activity>(activityPath(id), request, { headers: version ? { 'If-Match': version } : undefined }),
  inbox: (query: InboxQuery, signal?: AbortSignal) =>
    api.get<InboxResponse>(`${TASKS_PATH}/inbox`, {
      query: { owner: query.owner, team_id: query.teamId || undefined, sort: query.sort },
      signal,
    }),
  complete: (id: string) => api.post<Activity>(activityPath(id, '/complete')),
  reopen: (id: string) => api.post<Activity>(activityPath(id, '/reopen')),
  remove: (id: string) => api.delete<void>(activityPath(id)),
  listTypes: (includeInactive: boolean) =>
    api.get<ActivityTypePage>(ACTIVITY_TYPES_PATH, {
      query: { include_inactive: includeInactive || undefined, per_page: ACTIVITY_TYPES_PAGE_SIZE },
    }),
  createType: (request: CreateActivityTypeRequest) => api.post<ActivityType>(ACTIVITY_TYPES_PATH, request),
  updateType: (id: string, request: UpdateActivityTypeRequest) =>
    api.patch<ActivityType>(activityTypePath(id), request),
  setTypeActive: (id: string, active: boolean) =>
    api.post<ActivityType>(activityTypePath(id, active ? '/activate' : '/deactivate')),
  reorderTypes: (ids: string[]) => api.put<ActivityType[]>(`${ACTIVITY_TYPES_PATH}/order`, { ids }),
}

export function useActivityTypes(includeInactive = false) {
  return useQuery({
    queryKey: activitiesKeys.types(includeInactive),
    queryFn: () => activitiesApi.listTypes(includeInactive),
    select: (page) => page.data,
    staleTime: 60_000,
  })
}

export function useRecordActivities(target: RecordTarget, { enabled = true }: { enabled?: boolean } = {}) {
  return useQuery({
    queryKey: activitiesKeys.forRecord(target),
    queryFn: () => activitiesApi.listForRecord(target),
    enabled,
  })
}

export function useMyOpenActivities() {
  return useInfiniteQuery({
    queryKey: activitiesKeys.mine(),
    queryFn: ({ pageParam }) => activitiesApi.listMine(pageParam),
    initialPageParam: 1,
    getNextPageParam: (last, pages) => {
      const loaded = pages.reduce((count, page) => count + page.data.length, 0)
      return last.data.length > 0 && loaded < last.meta.total ? pages.length + 1 : undefined
    },
  })
}

export function useInvalidateActivities(target?: RecordTarget) {
  const queryClient = useQueryClient()
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: activitiesKeys.all }),
      queryClient.invalidateQueries({ queryKey: ['calendar'] }),
      queryClient.invalidateQueries({ queryKey: [...recordsKeys.all, ACTIVITY_RECORD_TYPE] }),
      target
        ? queryClient.invalidateQueries({ queryKey: recordsKeys.detail(target.recordType, target.recordId) })
        : undefined,
    ])
}

export function useCreateActivity(target?: RecordTarget) {
  const invalidate = useInvalidateActivities(target)
  return useMutation({
    mutationFn: activitiesApi.create,
    onSuccess: () => invalidate(),
  })
}

export function useToggleActivity(target?: RecordTarget) {
  const invalidate = useInvalidateActivities(target)
  return useMutation({
    mutationFn: ({ id, done }: { id: string; done: boolean }) =>
      done ? activitiesApi.complete(id) : activitiesApi.reopen(id),
    onSettled: () => invalidate(),
  })
}

export function useDeleteActivity(target?: RecordTarget) {
  const invalidate = useInvalidateActivities(target)
  return useMutation({ mutationFn: activitiesApi.remove, onSuccess: () => invalidate() })
}

export function useInvalidateActivityTypes() {
  const queryClient = useQueryClient()
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: [...activitiesKeys.all, 'types'] }),
      queryClient.invalidateQueries({ queryKey: [...recordsKeys.all, ACTIVITY_TYPE_RECORD_TYPE] }),
    ])
}

export function useActivityInbox(query: InboxQuery) {
  return useQuery({
    queryKey: activitiesKeys.inbox(query),
    queryFn: ({ signal }) => activitiesApi.inbox(query, signal),
    placeholderData: keepPreviousData,
  })
}

export interface UpdateActivityVariables {
  activity: Pick<Activity, 'id' | 'updated_at'>
  request: UpdateActivityRequest
}

export function useUpdateActivity(target?: RecordTarget) {
  const invalidate = useInvalidateActivities(target)
  return useMutation({
    mutationFn: ({ activity, request }: UpdateActivityVariables) =>
      activitiesApi.update(activity.id, request, activity.updated_at),
    onSettled: () => invalidate(),
  })
}
