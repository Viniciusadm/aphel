import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api, type components } from '@/api'

export type Member = components['schemas']['MemberResponse']
export type MemberStatus = components['schemas']['MemberStatusDto']
export type InvitedMember = components['schemas']['Data_InvitationResponse']['data']
export type PasswordResetLink = components['schemas']['PasswordResetLinkResponse']
export type MembersPage = components['schemas']['Page_MemberResponse']
export type Role = components['schemas']['RoleResponse']
export type RolesPage = components['schemas']['Page_RoleResponse']
export type Team = components['schemas']['TeamResponse']
export type TeamsPage = components['schemas']['Page_TeamResponse']
export type PermissionDefinition = components['schemas']['PermissionDefinitionResponse']
export type PermissionGrant = components['schemas']['PermissionGrantRequest']
export type RecordScope = components['schemas']['RecordScopeDto']
export type InviteMemberRequest = components['schemas']['InviteMemberRequest']
export type CreateRoleRequest = components['schemas']['CreateRoleRequest']
export type UpdateRoleRequest = components['schemas']['UpdateRoleRequest']
export type TeamRequest = components['schemas']['TeamRequest']
export type UpdateTeamRequest = Partial<TeamRequest>

export function versionOf(item: object | null | undefined): string | null {
  if (!item || !('updated_at' in item)) return null
  const value = (item as { updated_at?: unknown }).updated_at
  return typeof value === 'string' ? value : null
}

export function ifMatchHeaders(version?: string | null): HeadersInit | undefined {
  return version ? { 'If-Match': version } : undefined
}

export const MEMBERS_PER_PAGE = 25
export const MAX_PER_PAGE = 100
const MAX_PAGES = 50

interface PagedResult<T> {
  data: T[]
  meta: { total: number }
}

export async function fetchAllPages<T>(load: (page: number, perPage: number) => Promise<PagedResult<T>>): Promise<T[]> {
  const items: T[] = []
  for (let page = 1; page <= MAX_PAGES; page += 1) {
    const result = await load(page, MAX_PER_PAGE)
    items.push(...result.data)
    if (result.data.length === 0 || page * MAX_PER_PAGE >= result.meta.total) break
  }
  return items
}

export const settingsKeys = {
  all: ['settings'] as const,
  members: () => [...settingsKeys.all, 'members'] as const,
  membersPage: (page: number) => [...settingsKeys.members(), page] as const,
  allMembers: () => [...settingsKeys.members(), 'all'] as const,
  roles: () => [...settingsKeys.all, 'roles'] as const,
  teams: () => [...settingsKeys.all, 'teams'] as const,
  permissions: () => [...settingsKeys.all, 'permissions'] as const,
}

export const settingsApi = {
  listMembers: (page: number, perPage: number) =>
    api.get<MembersPage>('/v1/members', { query: { page, per_page: perPage } }),
  inviteMember: (request: InviteMemberRequest) => api.post<InvitedMember>('/v1/members/invitations', request),
  resendInvitation: (userId: string) => api.post<InvitedMember>(`/v1/members/${userId}/resend-invitation`),
  resetMemberPassword: (userId: string) => api.post<PasswordResetLink>(`/v1/members/${userId}/password-reset`),
  cancelInvitation: (userId: string) => api.delete<void>(`/v1/members/${userId}/invitation`),
  changeMemberRole: (userId: string, roleId: string, version?: string | null) =>
    api.patch<Member>(`/v1/members/${userId}`, { role_id: roleId }, { headers: ifMatchHeaders(version) }),
  suspendMember: (userId: string) => api.post<Member>(`/v1/members/${userId}/suspend`),
  reactivateMember: (userId: string) => api.post<Member>(`/v1/members/${userId}/reactivate`),
  listRoles: (page = 1, perPage = MAX_PER_PAGE) =>
    api.get<RolesPage>('/v1/roles', { query: { page, per_page: perPage } }),
  createRole: (request: CreateRoleRequest) => api.post<Role>('/v1/roles', request),
  updateRole: (roleId: string, request: UpdateRoleRequest, version?: string | null) =>
    api.patch<Role>(`/v1/roles/${roleId}`, request, { headers: ifMatchHeaders(version) }),
  deleteRole: (roleId: string) => api.delete<void>(`/v1/roles/${roleId}`),
  listPermissions: () => api.get<PermissionDefinition[]>('/v1/permissions'),
  listTeams: (page = 1, perPage = MAX_PER_PAGE) =>
    api.get<TeamsPage>('/v1/teams', { query: { page, per_page: perPage } }),
  createTeam: (request: TeamRequest) => api.post<Team>('/v1/teams', request),
  updateTeam: (teamId: string, request: UpdateTeamRequest, version?: string | null) =>
    api.patch<Team>(`/v1/teams/${teamId}`, request, { headers: ifMatchHeaders(version) }),
  deleteTeam: (teamId: string) => api.delete<void>(`/v1/teams/${teamId}`),
}

export function useMembersPage(page: number) {
  return useQuery({
    queryKey: settingsKeys.membersPage(page),
    queryFn: () => settingsApi.listMembers(page, MEMBERS_PER_PAGE),
    placeholderData: (previous) => previous,
  })
}

export function useAllMembers() {
  return useQuery({
    queryKey: settingsKeys.allMembers(),
    queryFn: () => fetchAllPages(settingsApi.listMembers),
  })
}

export function useRoles() {
  return useQuery({
    queryKey: settingsKeys.roles(),
    queryFn: () => fetchAllPages(settingsApi.listRoles),
  })
}

export function usePermissionCatalog() {
  return useQuery({
    queryKey: settingsKeys.permissions(),
    queryFn: settingsApi.listPermissions,
    staleTime: Infinity,
  })
}

export function useTeams(enabled = true) {
  return useQuery({
    queryKey: settingsKeys.teams(),
    queryFn: () => fetchAllPages(settingsApi.listTeams),
    enabled,
  })
}

export function useSettingsMutation<TVariables, TResult>(mutationFn: (variables: TVariables) => Promise<TResult>) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: settingsKeys.all }),
  })
}
