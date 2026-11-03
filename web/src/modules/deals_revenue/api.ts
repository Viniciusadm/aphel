import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'

export interface LinkedDeal {
  id: string
  title: string
}

interface DealPage {
  data: LinkedDeal[]
}

export const recurringKeys = {
  all: ['recurring'] as const,
  renewalOf: (dealId: string) => [...recurringKeys.all, 'renewal-of', dealId] as const,
  deal: (dealId: string) => [...recurringKeys.all, 'deal', dealId] as const,
}

export const recurringApi = {
  renewalOf: (dealId: string) =>
    api.get<DealPage>('/v1/deals', {
      query: { 'filter[renewal_of_deal_id][eq]': dealId, per_page: 1, archived: 'include' },
    }),
  deal: (dealId: string) => api.get<LinkedDeal>(`/v1/deals/${encodeURIComponent(dealId)}`),
}

export function useRenewalOf(dealId: string) {
  return useQuery({
    queryKey: recurringKeys.renewalOf(dealId),
    queryFn: async () => (await recurringApi.renewalOf(dealId)).data[0] ?? null,
    enabled: dealId !== '',
    retry: false,
  })
}

export function useLinkedDeal(dealId: string | null | undefined) {
  return useQuery({
    queryKey: recurringKeys.deal(dealId ?? ''),
    queryFn: () => recurringApi.deal(dealId ?? ''),
    enabled: Boolean(dealId),
    retry: false,
  })
}
