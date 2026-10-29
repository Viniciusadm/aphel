import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type SecurityEvent = Schemas['SecurityEventResponse']
export type SecurityEventKind = Schemas['SecurityEventKindDto']
export type SecurityEventsPage = Schemas['Page_SecurityEventResponse']
export type SecurityEventScope = 'mine' | 'organization'

export const SECURITY_EVENTS_PER_PAGE = 25

export const SECURITY_EVENT_KINDS: SecurityEventKind[] = [
  'login_succeeded',
  'login_failed',
  'login_locked',
  'two_factor_enabled',
  'two_factor_disabled',
  'two_factor_challenge_failed',
  'recovery_code_used',
  'recovery_codes_regenerated',
  'password_changed',
  'session_revoked',
  'api_token_created',
  'api_token_revoked',
  'permission_changed',
]

export interface SecurityEventFilters {
  scope: SecurityEventScope
  kind: SecurityEventKind | ''
  from: string
  to: string
}

export const securityEventsKeys = {
  all: ['security', 'events'] as const,
  list: (filters: SecurityEventFilters, page: number) => [...securityEventsKeys.all, filters, page] as const,
}

function localDayBoundary(date: string, endOfDay: boolean): string | undefined {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date)
  if (!match) return undefined
  const [, year, month, day] = match.map(Number) as [number, number, number, number]
  const instant = endOfDay
    ? new Date(year, month - 1, day, 23, 59, 59, 999)
    : new Date(year, month - 1, day, 0, 0, 0, 0)
  return Number.isNaN(instant.getTime()) ? undefined : instant.toISOString()
}

export const securityEventsApi = {
  list: (filters: SecurityEventFilters, page: number) =>
    api.get<SecurityEventsPage>(
      filters.scope === 'organization' ? '/v1/security/events/organization' : '/v1/security/events',
      {
        query: {
          kind: filters.kind || undefined,
          from: localDayBoundary(filters.from, false),
          to: localDayBoundary(filters.to, true),
          page,
          per_page: SECURITY_EVENTS_PER_PAGE,
        },
      },
    ),
}

export function useSecurityEvents(filters: SecurityEventFilters, page: number) {
  return useQuery({
    queryKey: securityEventsKeys.list(filters, page),
    queryFn: () => securityEventsApi.list(filters, page),
    placeholderData: keepPreviousData,
  })
}
