import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import { isModuleActive, useModuleState } from '@/kernel/modules/registry'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type Goal = Schemas['GoalResponse']
export type GoalsPage = Schemas['Page_GoalResponse']
export type GoalMetric = Schemas['GoalMetric']
export type GoalPeriod = Schemas['GoalPeriod']
export type GoalOwnerType = Schemas['GoalOwnerType']
export type GoalStatus = Schemas['GoalStatus']
export type GoalProgress = Schemas['GoalProgressResponse']
export type GoalOwnerOptions = Schemas['GoalOwnerOptionsResponse']
export type GoalOwnerOption = Schemas['GoalOwnerOptionResponse']

export const GOAL_METRICS: GoalMetric[] = ['won_amount', 'won_count', 'activities_done', 'new_contacts']

export const GOAL_METRIC_MODULE: Record<GoalMetric, 'deals' | 'tasks' | 'people'> = {
  won_amount: 'deals',
  won_count: 'deals',
  activities_done: 'tasks',
  new_contacts: 'people',
}

export function useMetricModuleActive(): (metric: GoalMetric) => boolean {
  const state = useModuleState()
  return (metric) => isModuleActive(GOAL_METRIC_MODULE[metric], state)
}
export const GOAL_PERIODS: GoalPeriod[] = ['month', 'quarter', 'year']
export const GOAL_OWNER_TYPES: GoalOwnerType[] = ['user', 'team', 'organization']
export const GOALS_PER_PAGE = 24

export type CreditMode = Schemas['GoalCreditMode']

export const CREDIT_MODES: CreditMode[] = ['owner', 'split']

export function goalCreditMode(goal: Goal | null | undefined): CreditMode {
  return goal?.credit_mode === 'split' ? 'split' : 'owner'
}

export interface GoalListParams {
  current?: boolean
  owner_type?: GoalOwnerType
  user_id?: string
  team_id?: string
  metric?: GoalMetric
  period?: GoalPeriod
  page?: number
  per_page?: number
}

export interface GoalBody {
  owner_type: GoalOwnerType
  user_id?: string | null
  team_id?: string | null
  metric: GoalMetric
  period: GoalPeriod
  starts_on: string
  target: string
  credit_mode?: CreditMode
}

export const goalsKeys = {
  all: ['goals'] as const,
  list: (params: GoalListParams) => [...goalsKeys.all, 'list', params] as const,
  owners: () => [...goalsKeys.all, 'owners'] as const,
}

function goalPath(id: string) {
  return `/v1/goals/${encodeURIComponent(id)}`
}

export const goalsApi = {
  list: (params: GoalListParams) => api.get<GoalsPage>('/v1/goals', { query: { ...params } }),
  owners: () => api.get<GoalOwnerOptions>('/v1/goals/owners'),
  create: (body: GoalBody) => api.post<Goal>('/v1/goals', body),
  update: (id: string, body: Partial<GoalBody>, version?: string) =>
    api.patch<Goal>(goalPath(id), body, { headers: version ? { 'If-Match': version } : undefined }),
  remove: (id: string) => api.delete<void>(goalPath(id)),
}

export function useGoals(params: GoalListParams) {
  return useQuery({
    queryKey: goalsKeys.list(params),
    queryFn: () => goalsApi.list(params),
    placeholderData: keepPreviousData,
  })
}

export function useGoalOwners() {
  return useQuery({ queryKey: goalsKeys.owners(), queryFn: goalsApi.owners, staleTime: 60_000 })
}

export function useInvalidateGoals() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: goalsKeys.all })
}

export function isCountMetric(metric: GoalMetric): boolean {
  return metric !== 'won_amount'
}
