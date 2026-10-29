import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import { recordsKeys } from '@/features/records/api'
import type {
  AddDealMemberRequest,
  CreateDealMemberRoleRequest,
  DealMember,
  DealMemberRole,
  DealTeam,
  UpdateDealMemberRequest,
  UpdateDealMemberRoleRequest,
} from './types'

const DEAL_RECORD_TYPE = 'deal'

export const dealTeamKeys = {
  all: ['deal-team'] as const,
  deal: (dealId: string) => [...dealTeamKeys.all, 'deal', dealId] as const,
  roles: (includeInactive: boolean) => [...dealTeamKeys.all, 'roles', includeInactive] as const,
}

const id = encodeURIComponent

function membersPath(dealId: string, suffix = '') {
  return `/v1/deals/${id(dealId)}/members${suffix}`
}

export const dealTeamApi = {
  team: (dealId: string) => api.get<DealTeam>(membersPath(dealId)),
  add: (dealId: string, body: AddDealMemberRequest) => api.post<DealMember>(membersPath(dealId), body),
  update: (dealId: string, memberId: string, body: UpdateDealMemberRequest, version?: string) =>
    api.patch<DealMember>(membersPath(dealId, `/${id(memberId)}`), body, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
  remove: (dealId: string, memberId: string) => api.delete<void>(membersPath(dealId, `/${id(memberId)}`)),
  roles: (includeInactive: boolean) =>
    api.get<DealMemberRole[]>('/v1/deal-member-roles', { query: { include_inactive: includeInactive } }),
  createRole: (body: CreateDealMemberRoleRequest) => api.post<DealMemberRole>('/v1/deal-member-roles', body),
  updateRole: (roleId: string, body: UpdateDealMemberRoleRequest, version?: string) =>
    api.patch<DealMemberRole>(`/v1/deal-member-roles/${id(roleId)}`, body, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
  deleteRole: (roleId: string) => api.delete<void>(`/v1/deal-member-roles/${id(roleId)}`),
  reorderRoles: (ids: string[]) => api.put<DealMemberRole[]>('/v1/deal-member-roles/order', { ids }),
}

export function useDealTeam(dealId: string, enabled = true) {
  return useQuery({
    queryKey: dealTeamKeys.deal(dealId),
    queryFn: () => dealTeamApi.team(dealId),
    enabled: enabled && dealId !== '',
    retry: false,
  })
}

export function useDealMemberRoles(includeInactive = false) {
  return useQuery({
    queryKey: dealTeamKeys.roles(includeInactive),
    queryFn: () => dealTeamApi.roles(includeInactive),
    staleTime: 60_000,
  })
}

export function useInvalidateDealMemberRoles() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: [...dealTeamKeys.all, 'roles'] })
}

export function useDealTeamMutations(dealId: string) {
  const queryClient = useQueryClient()
  const refresh = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: dealTeamKeys.deal(dealId) }),
      queryClient.invalidateQueries({ queryKey: recordsKeys.detail(DEAL_RECORD_TYPE, dealId) }),
    ])
  }
  return {
    add: useMutation({
      mutationFn: (body: AddDealMemberRequest) => dealTeamApi.add(dealId, body),
      onSettled: refresh,
    }),
    update: useMutation({
      mutationFn: ({ member, body }: { member: DealMember; body: UpdateDealMemberRequest }) =>
        dealTeamApi.update(dealId, member.id, body, member.updated_at),
      onSettled: refresh,
    }),
    remove: useMutation({
      mutationFn: (member: DealMember) => dealTeamApi.remove(dealId, member.id),
      onSettled: refresh,
    }),
  }
}
