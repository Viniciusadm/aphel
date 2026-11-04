import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api, type components } from '@/api'

type Schemas = components['schemas']

export type DistributionRule = Schemas['LeadDistributionRuleResponse']
export type DistributionRuleMember = Schemas['LeadDistributionMemberResponse']
export type DistributionRoute = Schemas['LeadDistributionRouteResponse']
export type DistributionRulesPage = Schemas['Page_LeadDistributionRuleResponse']
export type DistributionRuleRequest = Schemas['LeadDistributionRuleRequest']
export type DistributionStrategy = Schemas['LeadDistributionStrategy']
export type DistributionFallback = Schemas['LeadDistributionFallback']
export type DistributionSimulation = Schemas['LeadDistributionSimulationResponse']
export type DistributionSimulateRequest = Schemas['LeadDistributionSimulateRequest']

export type AssignmentSource = 'automation' | 'incoming_webhook' | 'import' | 'booking' | 'lead_form'

export interface DistributionAssignment {
  id: string
  rule_id: string
  rule_key: string
  rule_name: string
  user_id: string | null
  user_name: string | null
  record_type: string | null
  record_id: string | null
  source: AssignmentSource
  created_at: string
}

export interface DistributionAssignmentsPage {
  data: DistributionAssignment[]
  meta: { total: number; page: number; per_page: number }
}

export const ASSIGNMENTS_PER_PAGE = 25

export const STRATEGIES: DistributionStrategy[] = ['round_robin', 'weighted', 'least_loaded']
export const RULES_PER_PAGE = 100

function ifMatch(version?: string | null): HeadersInit | undefined {
  return version ? { 'If-Match': version } : undefined
}

export const leadDistributionKeys = {
  all: ['assignment'] as const,
  rules: () => [...leadDistributionKeys.all, 'rules'] as const,
  teams: () => [...leadDistributionKeys.all, 'teams'] as const,
  assignments: (ruleId: string | null, page: number) =>
    [...leadDistributionKeys.all, 'assignments', ruleId, page] as const,
}

export const leadDistributionApi = {
  list: () =>
    api.get<DistributionRulesPage>('/v1/assignment/rules', { query: { per_page: RULES_PER_PAGE } }),
  create: (request: DistributionRuleRequest) =>
    api.post<DistributionRule>('/v1/assignment/rules', request),
  update: (ruleId: string, request: DistributionRuleRequest, version?: string | null) =>
    api.patch<DistributionRule>(`/v1/assignment/rules/${ruleId}`, request, { headers: ifMatch(version) }),
  remove: (ruleId: string) => api.delete<void>(`/v1/assignment/rules/${ruleId}`),
  simulate: (ruleId: string, request: DistributionSimulateRequest) =>
    api.post<DistributionSimulation>(`/v1/assignment/rules/${ruleId}/simulate`, request),
  assignments: (ruleId: string | null, page: number) =>
    api.get<DistributionAssignmentsPage>('/v1/assignment/assignments', {
      query: { page, per_page: ASSIGNMENTS_PER_PAGE, ...(ruleId ? { rule_id: ruleId } : {}) },
    }),
  teams: async () => {
    const page = await api.get<{ data: { id: string; name: string }[] }>('/v1/teams', { query: { per_page: 100 } })
    return page.data.map((team) => ({ id: team.id, name: team.name }))
  },
}

export function useDistributionRules() {
  return useQuery({
    queryKey: leadDistributionKeys.rules(),
    queryFn: leadDistributionApi.list,
    select: (page) => page.data,
  })
}

export function useDistributionTeams() {
  return useQuery({
    queryKey: leadDistributionKeys.teams(),
    queryFn: () => leadDistributionApi.teams().catch(() => []),
    staleTime: 5 * 60_000,
  })
}

export function useDistributionMutation<TVariables, TResult>(mutationFn: (variables: TVariables) => Promise<TResult>) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: leadDistributionKeys.rules() }),
  })
}

export function useSimulation() {
  return useMutation({
    mutationFn: ({ ruleId, request }: { ruleId: string; request: DistributionSimulateRequest }) =>
      leadDistributionApi.simulate(ruleId, request),
  })
}

export function useDistributionAssignments(ruleId: string | null, page: number) {
  return useQuery({
    queryKey: leadDistributionKeys.assignments(ruleId, page),
    queryFn: () => leadDistributionApi.assignments(ruleId, page),
    placeholderData: keepPreviousData,
  })
}

export interface AssignmentTarget { record_type:string; label:string; labels:Record<string,string> }
export function useAssignmentTargets() {return useQuery({queryKey:['assignment','targets'],queryFn:()=>api.get<AssignmentTarget[]>('/v1/assignment/targets')})}
