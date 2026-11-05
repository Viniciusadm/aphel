import type { components } from '@/modules/line_items/api-types'

type Schemas = components['schemas']

export type Recurrence = Schemas['Recurrence']

export const RECURRENCES: Recurrence[] = ['none', 'monthly', 'quarterly', 'yearly']

export const MAX_TERM_MONTHS = 600

export type RecurringItemFields = Pick<Schemas['LineItemResponse'], 'recurrence' | 'term_months' | 'monthly_amount'>

export type RecurringItemRequest = Pick<Schemas['LineItemRequest'], 'recurrence' | 'term_months'>

