import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'

export const ESIGN_KEY = 'esign'
export const SEND_PERMISSION = 'esign.send'
export const VIEW_PERMISSION = 'esign.view'

export type EsignRequestStatus = 'sent' | 'completed' | 'refused' | 'cancelled' | 'expired'
export type EsignSignerStatus = 'pending' | 'signed' | 'refused'

export interface EsignSigner {
  id: string
  name: string
  email: string
  contact_id?: string | null
  status: EsignSignerStatus
  signed_at?: string | null
  refused_at?: string | null
}

export interface EsignRequest {
  id: string
  deal_id: string
  file_id: string
  file_name: string
  provider: string
  environment: string
  status: EsignRequestStatus
  message?: string | null
  signed_file_id?: string | null
  created_by: string
  created_at: string
  updated_at: string
  completed_at?: string | null
  signers: EsignSigner[]
}

export interface EsignStatus {
  provider: string
  environment: string
  configured: boolean
  webhook_url: string
  legal_notice: string
}

export interface EsignSignerInput {
  name: string
  email: string
  contact_id?: string
}

export interface CreateEsignRequest {
  deal_id: string
  file_id: string
  signers: EsignSignerInput[]
  message?: string
}

const BASE = '/v1/esign'

export const esignKeys = {
  all: [ESIGN_KEY] as const,
  status: () => [...esignKeys.all, 'status'] as const,
  requests: (dealId: string) => [...esignKeys.all, 'requests', dealId] as const,
}

export const esignApi = {
  status: () => api.get<EsignStatus>(`${BASE}/status`),
  requests: (dealId: string) => api.get<EsignRequest[]>(`${BASE}/requests`, { query: { deal_id: dealId } }),
  create: (body: CreateEsignRequest) => api.post<EsignRequest>(`${BASE}/requests`, body),
  cancel: (requestId: string) => api.post<EsignRequest>(`${BASE}/requests/${encodeURIComponent(requestId)}/cancel`),
}

export function useEsignStatus(enabled = true) {
  return useQuery({ queryKey: esignKeys.status(), queryFn: esignApi.status, enabled, staleTime: 60_000 })
}

export function useEsignRequests(dealId: string) {
  return useQuery({ queryKey: esignKeys.requests(dealId), queryFn: () => esignApi.requests(dealId) })
}

export function useInvalidateEsign() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: esignKeys.all })
}
