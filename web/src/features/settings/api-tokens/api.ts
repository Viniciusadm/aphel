import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type ApiToken = Schemas['ApiTokenResponse']
export type CreatedApiToken = Schemas['CreatedApiTokenResponse']
export type ApiTokenScope = Schemas['ApiTokenScopeResponse']
export type ApiTokensPage = Schemas['Page_ApiTokenResponse']
export type CreateApiTokenBody = Schemas['CreateApiTokenRequest']
export type ApiTokenOwner = 'mine' | 'all'

export const API_TOKENS_PER_PAGE = 100
export const MAX_API_TOKEN_NAME_LENGTH = 100

export const apiTokensKeys = {
  all: ['api-tokens'] as const,
  list: (owner: ApiTokenOwner) => [...apiTokensKeys.all, 'list', owner] as const,
  scopes: () => [...apiTokensKeys.all, 'scopes'] as const,
}

export const apiTokensApi = {
  list: (owner: ApiTokenOwner) =>
    api.get<ApiTokensPage>('/v1/api-tokens', { query: { owner, per_page: API_TOKENS_PER_PAGE } }),
  scopes: () => api.get<ApiTokenScope[]>('/v1/api-tokens/scopes'),
  create: (body: CreateApiTokenBody) => api.post<CreatedApiToken>('/v1/api-tokens', body),
  revoke: (id: string) => api.post<ApiToken>(`/v1/api-tokens/${encodeURIComponent(id)}/revoke`),
}

export function useApiTokens(owner: ApiTokenOwner) {
  return useQuery({
    queryKey: apiTokensKeys.list(owner),
    queryFn: () => apiTokensApi.list(owner),
    select: (page) => page.data,
  })
}

export function useApiTokenScopes() {
  return useQuery({ queryKey: apiTokensKeys.scopes(), queryFn: apiTokensApi.scopes })
}

export function useInvalidateApiTokens() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: apiTokensKeys.all })
}

export type ApiTokenStatus = 'active' | 'revoked' | 'expired'

export function tokenStatus(token: ApiToken, now: Date = new Date()): ApiTokenStatus {
  if (token.revoked_at) return 'revoked'
  if (token.expires_at && new Date(token.expires_at) <= now) return 'expired'
  return 'active'
}

export interface ScopeGroup {
  resource: string
  read?: ApiTokenScope
  write?: ApiTokenScope
}

export function groupScopes(scopes: ApiTokenScope[]): ScopeGroup[] {
  const groups = new Map<string, ScopeGroup>()
  for (const scope of scopes) {
    const group = groups.get(scope.resource) ?? { resource: scope.resource }
    if (scope.access === 'read') group.read = scope
    else group.write = scope
    groups.set(scope.resource, group)
  }
  return [...groups.values()]
}

export function expirationDate(days: number | null, now: Date = new Date()): string | null {
  if (days === null) return null
  const date = new Date(now.getTime() + days * 24 * 60 * 60 * 1000)
  return date.toISOString()
}
