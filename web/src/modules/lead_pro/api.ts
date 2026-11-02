import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import { recordsKeys } from '@/features/records/api'


export const LEAD_PRO_KEY = 'lead_pro'
export const LEAD_PRO_PATH = '/lead-pro'
export const READ_PERMISSION = 'lead_pro.read'
export const TEMPERATURE_FIELD = 'lead_temperature'
export const MAX_TEMPERATURE = 100

const BASE = '/v1/lead-pro'

export interface LeadTouch {
  id: string
  kind: string
  created_at: string
}

export interface LeadTemperature {
  contact_id: string
  temperature: number | null
  hot_threshold: number
  hot: boolean
  touches: LeadTouch[]
}

export interface HotLead {
  contact_id: string
  name: string
  email: string | null
  temperature: number | null
}

export interface HotLeadList {
  data: HotLead[]
  hot_threshold: number
}

const leadProKeys = {
  all: ['lead_pro'] as const,
  board: () => [...leadProKeys.all, 'board'] as const,
  contact: (contactId: string) => [...leadProKeys.all, 'contact', contactId] as const,
}

export const leadProApi = {
  board: () => api.get<HotLeadList>(`${BASE}/board`),
  temperature: (contactId: string) => api.get<LeadTemperature>(`${BASE}/contacts/${contactId}`),
  touch: (contactId: string) => api.post<LeadTemperature>(`${BASE}/contacts/${contactId}/touches`),
}

export function useHotLeads() {
  return useQuery({ queryKey: leadProKeys.board(), queryFn: leadProApi.board })
}

export function useLeadTemperature(contactId: string) {
  return useQuery({ queryKey: leadProKeys.contact(contactId), queryFn: () => leadProApi.temperature(contactId) })
}

export function useRegisterTouch(contactId: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: () => leadProApi.touch(contactId),
    onSuccess: (temperature) => {
      queryClient.setQueryData(leadProKeys.contact(contactId), temperature)
      void queryClient.invalidateQueries({ queryKey: leadProKeys.board() })
      void queryClient.invalidateQueries({ queryKey: [...recordsKeys.all, 'contact'] })
    },
  })
}

export type TemperatureBand = 'cold' | 'warm' | 'hot'

export function temperatureBand(temperature: number, hotThreshold: number): TemperatureBand {
  if (temperature >= hotThreshold) return 'hot'
  if (temperature >= hotThreshold / 2) return 'warm'
  return 'cold'
}
