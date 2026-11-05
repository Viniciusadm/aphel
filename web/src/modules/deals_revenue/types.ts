import type { components } from '@/modules/deals_revenue/api-types'

type Schemas = components['schemas']

export type DealRevenueFields = Partial<
  Pick<
    Schemas['DealResponse'],
    'currency' | 'status' | 'mrr' | 'arr' | 'tcv' | 'contract_start' | 'contract_end' | 'renewal_of_deal_id'
  >
>

export interface RecurringRevenueTotals {
  won_count: number
  mrr: string | null
  arr: string | null
  tcv: string | null
  partial?: boolean
}

export interface RecurringRevenueData extends RecurringRevenueTotals {
  currency: string
  previous: RecurringRevenueTotals | null
}
