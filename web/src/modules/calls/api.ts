import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import { activitiesKeys } from '@/modules/tasks/api'
import type {
  CallActivity,
  CallOutcome,
  CallRecording,
  CallReport,
  CallReportQuery,
  CallSession,
  CallSessionItemPage,
  CallSessionPage,
  CreateCallOutcomeRequest,
  LogCallRequest,
  SessionCallRequest,
  SessionCallResult,
  StartCallRequest,
  StartCallSessionRequest,
  StartedCall,
  TelephonyProvider,
  UpdateCallOutcomeRequest,
  UpdateCallRequest,
} from './types'

export const SESSION_ITEMS_PER_PAGE = 100

export const callsKeys = {
  all: ['calls'] as const,
  outcomes: (includeInactive: boolean) => [...callsKeys.all, 'outcomes', includeInactive] as const,
  providers: () => [...callsKeys.all, 'providers'] as const,
  report: (query: CallReportQuery) => [...callsKeys.all, 'report', query] as const,
  sessions: () => [...callsKeys.all, 'sessions'] as const,
  session: (id: string) => [...callsKeys.sessions(), 'detail', id] as const,
  sessionItems: (id: string) => [...callsKeys.sessions(), 'items', id] as const,
}

const id = encodeURIComponent

export const callsApi = {
  log: (request: LogCallRequest) => api.post<CallActivity>('/v1/calls', request),
  update: (activityId: string, request: UpdateCallRequest, version?: string | null) =>
    api.patch<CallActivity>(`/v1/calls/${id(activityId)}`, request, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
  recording: (activityId: string) => api.get<CallRecording>(`/v1/calls/${id(activityId)}/recording`),
  providers: () => api.get<TelephonyProvider[]>('/v1/calls/providers'),
  start: (request: StartCallRequest) => api.post<StartedCall>('/v1/calls/start', request),
  report: (query: CallReportQuery) =>
    api.get<CallReport>('/v1/calls/report', {
      query: { from: query.from, to: query.to, owner_id: query.owner_id },
    }),
  outcomes: (includeInactive: boolean) =>
    api.get<CallOutcome[]>('/v1/calls/outcomes', { query: { include_inactive: includeInactive } }),
  createOutcome: (request: CreateCallOutcomeRequest) => api.post<CallOutcome>('/v1/calls/outcomes', request),
  updateOutcome: (outcomeId: string, request: UpdateCallOutcomeRequest) =>
    api.patch<CallOutcome>(`/v1/calls/outcomes/${id(outcomeId)}`, request),
  reorderOutcomes: (ids: string[]) => api.put<CallOutcome[]>('/v1/calls/outcomes/order', { ids }),
  sessions: (page = 1) => api.get<CallSessionPage>('/v1/calls/sessions', { query: { page, per_page: 20 } }),
  startSession: (request: StartCallSessionRequest) => api.post<CallSession>('/v1/calls/sessions', request),
  session: (sessionId: string) => api.get<CallSession>(`/v1/calls/sessions/${id(sessionId)}`),
  sessionItems: (sessionId: string) =>
    api.get<CallSessionItemPage>(`/v1/calls/sessions/${id(sessionId)}/items`, {
      query: { page: 1, per_page: SESSION_ITEMS_PER_PAGE },
    }),
  logInSession: (sessionId: string, request: SessionCallRequest) =>
    api.post<SessionCallResult>(`/v1/calls/sessions/${id(sessionId)}/calls`, request),
  skip: (sessionId: string, position?: number) =>
    api.post<CallSession>(`/v1/calls/sessions/${id(sessionId)}/skip`, { position }),
  moveTo: (sessionId: string, position: number) =>
    api.post<CallSession>(`/v1/calls/sessions/${id(sessionId)}/cursor`, { position }),
  finish: (sessionId: string) => api.post<CallSession>(`/v1/calls/sessions/${id(sessionId)}/finish`),
}

export function useCallOutcomes(includeInactive = false) {
  return useQuery({
    queryKey: callsKeys.outcomes(includeInactive),
    queryFn: () => callsApi.outcomes(includeInactive),
    staleTime: 60_000,
  })
}

export function useTelephonyProviders(enabled = true) {
  return useQuery({
    queryKey: callsKeys.providers(),
    queryFn: () => callsApi.providers(),
    staleTime: 5 * 60_000,
    enabled,
  })
}

export function useInvalidateCalls() {
  const client = useQueryClient()
  return async () => {
    await Promise.all([
      client.invalidateQueries({ queryKey: callsKeys.all }),
      client.invalidateQueries({ queryKey: activitiesKeys.all }),
    ])
  }
}
