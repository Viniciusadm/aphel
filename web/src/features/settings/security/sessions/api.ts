import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type SecuritySession = Schemas['SecuritySessionResponse']
export type SecurityDevice = Schemas['DeviceResponse']
export type RevokedSessions = Schemas['RevokedSessionsResponse']
export type ChangePasswordBody = Schemas['ChangePasswordRequest']
export type PasswordChanged = Schemas['PasswordChangedResponse']

export const MIN_PASSWORD_LENGTH = 8
export const MAX_PASSWORD_LENGTH = 128

export const securitySessionsKeys = {
  all: ['security', 'sessions'] as const,
}

export const securitySessionsApi = {
  list: () => api.get<SecuritySession[]>('/v1/security/sessions'),
  revoke: (id: string) => api.delete<void>(`/v1/security/sessions/${encodeURIComponent(id)}`),
  revokeOthers: () => api.post<RevokedSessions>('/v1/security/sessions/revoke-others'),
  changePassword: (body: ChangePasswordBody) => api.post<PasswordChanged>('/v1/security/password', body),
}

export function useSecuritySessions() {
  return useQuery({ queryKey: securitySessionsKeys.all, queryFn: securitySessionsApi.list })
}

export function useInvalidateSecuritySessions() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: securitySessionsKeys.all })
}
