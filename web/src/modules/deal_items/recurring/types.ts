import type { components } from '@/api'

type Schemas = components['schemas']

export type Recurrence = Schemas['ProductRecurrence']

export const RECURRENCES: Recurrence[] = ['none', 'monthly', 'quarterly', 'yearly']

export const MAX_TERM_MONTHS = 600

export type RecurringItemFields = Pick<Schemas['DealItemResponse'], 'recurrence' | 'term_months' | 'monthly_amount'>

export type RecurringItemRequest = Pick<Schemas['DealItemRequest'], 'recurrence' | 'term_months'>

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
