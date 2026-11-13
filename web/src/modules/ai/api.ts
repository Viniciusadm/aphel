import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { api, isApiError } from '@/api'
import { type FilterNode } from '@/sdk'
export const AI_KEY = 'ai'
export const AI_PATH = '/integrations/ai'
export const USE_PERMISSION = 'ai.use'
export const VIEW_USAGE_PERMISSION = 'ai.view_usage'
export const AI_RECORD_TYPES = ['people.person', 'organizations.organization', 'deals.deal']
export const CALLS_PER_PAGE = 20
export const AI_FEATURES = ['summary', 'next_actions', 'compose', 'query'] as const

export type AiFeature = (typeof AI_FEATURES)[number]
export type AiProviderName = 'anthropic' | 'deepseek' | 'openai_compatible'
export type ComposeContext = 'note' | 'email' | 'message'

export interface AiStatus {
  configured: boolean
  api_key_configured: boolean
  provider: AiProviderName
  base_url: string
  model: string
  daily_limit: number
  used_today: number
  remaining_today: number
  user_hourly_limit: number
  send_personal_data: boolean
  features: Record<AiFeature, boolean>
}

export interface AiSource {
  record_type: string
  record_id: string
  label: string | null
}

interface AiAnswerMeta {
  provider: string
  model: string
}

export interface AiSummary extends AiAnswerMeta {
  text: string
  sources: AiSource[]
}

export interface AiActionSuggestion {
  title: string
  due_in_days: number
  reason: string
}

export interface AiNextActions extends AiAnswerMeta {
  suggestions: AiActionSuggestion[]
  sources: AiSource[]
}

export interface AiComposition extends AiAnswerMeta {
  text: string
  sources: AiSource[]
}

export interface AiQueryResult extends AiAnswerMeta {
  filter: FilterNode
  total: number
  explanation: string
}

export interface AiComposeRequest {
  context: ComposeContext
  instruction: string
  draft?: string
  record_type?: string
  record_id?: string
}

export interface AiCall {
  id: string
  user_id: string | null
  feature: AiFeature
  provider: string
  model: string
  status: 'success' | 'error'
  input_tokens: number | null
  output_tokens: number | null
  duration_ms: number
  error_code: string | null
  created_at: string
}

export interface AiUsageTotals {
  calls: number
  errors: number
  input_tokens: number
  output_tokens: number
}

export interface AiCallList {
  data: AiCall[]
  meta: {
    total: number
    page: number
    per_page: number
    used_today: number
    daily_limit: number
    last_30_days: AiUsageTotals
  }
}

export interface AiCallFilters {
  feature: AiFeature | null
  status: 'success' | 'error' | null
  page: number
}

const BASE = '/v1/ai'

export const aiKeys = {
  all: [AI_KEY] as const,
  status: () => [...aiKeys.all, 'status'] as const,
  calls: (filters: AiCallFilters) => [...aiKeys.all, 'calls', filters] as const,
}

export const aiApi = {
  status: () => api.get<AiStatus>(`${BASE}/status`),
  summary: (recordType: string, recordId: string) =>
    api.post<AiSummary>(`${BASE}/summary`, { record_type: recordType, record_id: recordId }),
  nextActions: (recordType: string, recordId: string) =>
    api.post<AiNextActions>(`${BASE}/next-actions`, { record_type: recordType, record_id: recordId }),
  compose: (request: AiComposeRequest) => api.post<AiComposition>(`${BASE}/compose`, request),
  query: (recordType: string, question: string) =>
    api.post<AiQueryResult>(`${BASE}/query`, { record_type: recordType, question }),
  calls: (filters: AiCallFilters) =>
    api.get<AiCallList>(`${BASE}/calls`, {
      query: {
        feature: filters.feature ?? undefined,
        status: filters.status ?? undefined,
        page: filters.page,
        per_page: CALLS_PER_PAGE,
      },
    }),
}

export function useAiStatus(enabled = true) {
  return useQuery({ queryKey: aiKeys.status(), queryFn: aiApi.status, staleTime: 30_000, enabled })
}

export function useAiCalls(filters: AiCallFilters, enabled = true) {
  return useQuery({
    queryKey: aiKeys.calls(filters),
    queryFn: () => aiApi.calls(filters),
    placeholderData: keepPreviousData,
    enabled,
  })
}

const KNOWN_ERRORS = new Set([
  'ai_not_configured',
  'ai_feature_disabled',
  'ai_limit_reached',
  'ai_user_limit_reached',
  'ai_provider_error',
  'ai_provider_auth_failed',
  'ai_provider_rate_limited',
  'ai_provider_timeout',
  'ai_refused',
  'ai_empty_response',
  'ai_invalid_response',
  'ai_record_not_found',
  'ai_query_invalid',
  'ai_query_no_fields',
  'ai_query_unanswerable',
  'permission_denied',
])

export function providerLabelKey(provider: string): string {
  if (provider === 'openai_compatible') return 'providers.openai_compatible'
  if (provider === 'deepseek') return 'providers.deepseek'
  return 'providers.anthropic'
}

export function aiErrorKey(error: unknown): string {
  if (isApiError(error) && KNOWN_ERRORS.has(error.code)) return `errors.${error.code}`
  return 'errors.generic'
}

export function aiErrorDetail(error: unknown): string | null {
  if (isApiError(error) && (error.code === 'ai_query_unanswerable' || error.code === 'ai_query_invalid')) {
    return error.message
  }
  return null
}
