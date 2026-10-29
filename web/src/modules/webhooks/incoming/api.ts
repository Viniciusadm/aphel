import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type IncomingWebhook = Schemas['IncomingWebhookResponse']
export type IssuedIncomingWebhook = Schemas['IssuedIncomingWebhookResponse']
export type IncomingWebhooksPage = Schemas['Page_IncomingWebhookResponse']
export type CreateIncomingWebhookBody = Schemas['CreateIncomingWebhookRequest']
export type UpdateIncomingWebhookBody = Schemas['UpdateIncomingWebhookRequest']
export type ActionType = Schemas['IncomingWebhookActionType']
export type ActionTarget = Schemas['IncomingActionTargetResponse']
export type MappableField = Schemas['IncomingMappableFieldResponse']
export type RequestStatus = Schemas['IncomingWebhookRequestStatus']
export type RequestLog = Schemas['IncomingWebhookRequestResponse']
export type RequestLogDetail = Schemas['IncomingWebhookRequestDetailResponse']
export type RequestLogsPage = Schemas['Page_IncomingWebhookRequestResponse']
export type ActionOutcome = Schemas['IncomingActionOutcomeResponse']

export interface ActionConfig {
  type: ActionType
  match_by?: ContactMatch | null
  match_field?: string | null
  pipeline_id?: string | null
  stage_id?: string | null
  activity_type_id?: string | null
  distribution_rule_id?: string | null
  fields: Record<string, unknown>
}

export type ContactMatch = 'email' | 'phone' | 'field'
export type MatchableField = MappableField & { is_unique?: boolean }

export const ACTION_TYPES: ActionType[] = ['upsert_contact', 'create_deal', 'create_activity']
export const REQUEST_STATUSES: RequestStatus[] = ['succeeded', 'partial', 'failed', 'rejected']
export const WEBHOOKS_PER_PAGE = 100
export const REQUESTS_PER_PAGE = 20
export const HOOKS_PATH = '/api/v1/hooks/'

export const incomingWebhooksKeys = {
  all: ['incoming-webhooks'] as const,
  list: () => [...incomingWebhooksKeys.all, 'list'] as const,
  targets: () => [...incomingWebhooksKeys.all, 'targets'] as const,
  requests: (id: string, status: RequestStatus | null, page: number) =>
    [...incomingWebhooksKeys.all, 'requests', id, status, page] as const,
  request: (id: string, requestId: string) => [...incomingWebhooksKeys.all, 'request', id, requestId] as const,
}

function webhookPath(id: string, action?: string) {
  const base = `/v1/incoming-webhooks/${encodeURIComponent(id)}`
  return action ? `${base}/${action}` : base
}

export const incomingWebhooksApi = {
  list: () => api.get<IncomingWebhooksPage>('/v1/incoming-webhooks', { query: { per_page: WEBHOOKS_PER_PAGE } }),
  targets: () => api.get<ActionTarget[]>('/v1/incoming-webhooks/targets'),
  create: (body: CreateIncomingWebhookBody) => api.post<IssuedIncomingWebhook>('/v1/incoming-webhooks', body),
  update: (id: string, body: UpdateIncomingWebhookBody, updatedAt: string) =>
    api.patch<IncomingWebhook>(webhookPath(id), body, { headers: { 'If-Match': updatedAt } }),
  remove: (id: string) => api.delete<void>(webhookPath(id)),
  regenerateToken: (id: string) => api.post<IssuedIncomingWebhook>(webhookPath(id, 'regenerate-token')),
  rotateSecret: (id: string) => api.post<IssuedIncomingWebhook>(webhookPath(id, 'signing-secret')),
  removeSecret: (id: string) => api.delete<void>(webhookPath(id, 'signing-secret')),
  requests: (id: string, status: RequestStatus | null, page: number) =>
    api.get<RequestLogsPage>(webhookPath(id, 'requests'), {
      query: { status: status ?? undefined, page, per_page: REQUESTS_PER_PAGE },
    }),
  request: (id: string, requestId: string) =>
    api.get<RequestLogDetail>(`${webhookPath(id, 'requests')}/${encodeURIComponent(requestId)}`),
}

export function useIncomingWebhooks() {
  return useQuery({
    queryKey: incomingWebhooksKeys.list(),
    queryFn: incomingWebhooksApi.list,
    select: (page) => page.data,
  })
}

export function useActionTargets() {
  return useQuery({
    queryKey: incomingWebhooksKeys.targets(),
    queryFn: incomingWebhooksApi.targets,
    staleTime: 60_000,
  })
}

export function useRequestLogs(id: string, status: RequestStatus | null, page: number) {
  return useQuery({
    queryKey: incomingWebhooksKeys.requests(id, status, page),
    queryFn: () => incomingWebhooksApi.requests(id, status, page),
    placeholderData: keepPreviousData,
  })
}

export function useRequestLog(id: string, requestId: string | null) {
  return useQuery({
    queryKey: incomingWebhooksKeys.request(id, requestId ?? ''),
    queryFn: () => incomingWebhooksApi.request(id, requestId ?? ''),
    enabled: requestId !== null,
  })
}

export function useInvalidateIncomingWebhooks() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: incomingWebhooksKeys.all })
}

export function hookUrl(path: string, origin: string = window.location.origin): string {
  return `${origin}${path}`
}

export function webhookActions(webhook: IncomingWebhook): ActionConfig[] {
  return webhook.actions as unknown as ActionConfig[]
}
