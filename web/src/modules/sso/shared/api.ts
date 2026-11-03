import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'

export interface SsoLoginStatus {
  provider: string
  label: string
  callback_url: string
  configured: boolean
  domains: string[]
  require_sso: boolean
}

export type CalendarConnectionState = 'active' | 'error' | 'revoked'

export interface CalendarConnectionStatus {
  account_email: string | null
  status: CalendarConnectionState | string
  last_synced_at: string | null
  last_error_code: string | null
  last_error_message: string | null
  imported_count: number
  exported_count: number
  connected_at: string
}

export interface CalendarStatus {
  can_connect: boolean
  configured: boolean
  connection: CalendarConnectionStatus | null
}

export interface ProviderStatus {
  login: SsoLoginStatus
  calendar?: CalendarStatus
}

export interface CalendarConnectUrl {
  url: string
}

export const SSO_SHARED_KEY = 'sso'

function basePath(moduleKey: string) {
  return `/v1/${moduleKey}`
}

export function providerKeys(moduleKey: string) {
  return { status: [moduleKey, 'status'] as const }
}

export function useProviderStatus(moduleKey: string) {
  return useQuery({
    queryKey: providerKeys(moduleKey).status,
    queryFn: () => api.get<ProviderStatus | SsoLoginStatus>(`${basePath(moduleKey)}/status`),
    select: (data): ProviderStatus => ('login' in data ? data : { login: data }),
  })
}

export function useCalendarActions(moduleKey: string) {
  const queryClient = useQueryClient()
  const invalidate = () => queryClient.invalidateQueries({ queryKey: providerKeys(moduleKey).status })
  const connect = useMutation({
    mutationFn: () => api.post<CalendarConnectUrl>(`${basePath(moduleKey)}/calendar/connect`),
    onSuccess: (result) => {
      window.location.assign(result.url)
    },
  })
  const disconnect = useMutation({
    mutationFn: () => api.delete<void>(`${basePath(moduleKey)}/calendar/connection`),
    onSettled: invalidate,
  })
  const sync = useMutation({
    mutationFn: () => api.post<void>(`${basePath(moduleKey)}/calendar/sync`),
    onSettled: invalidate,
  })
  return { connect, disconnect, sync, invalidate }
}
