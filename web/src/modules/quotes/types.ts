import type { components } from '@/api'

type Schemas = components['schemas']

export type { Page } from '@/features/records/types'

export type QuoteStatus = Schemas['QuoteStatus']
export type QuoteDiscountType = Schemas['QuoteDiscountType']
export type QuoteItemComponent = Schemas['QuoteItemComponentResponse']
export type QuoteItem = Schemas['QuoteItemResponse']
export type QuoteSummary = Omit<Schemas['QuoteResponse'], 'custom_fields'> & { custom_fields: Record<string, unknown> }
export type QuoteVersion = Schemas['QuoteVersionResponse']
export type QuoteAcceptance = Schemas['QuoteAcceptanceResponse'] & { recorded_by?: string | null }
export type QuoteDetail = Omit<Schemas['QuoteDetailResponse'], 'custom_fields' | 'acceptance'> & {
  custom_fields: Record<string, unknown>
  acceptance?: QuoteAcceptance | null
}
export type QuoteItemRequest = Schemas['QuoteItemRequest']
export type QuoteRequest = Omit<Schemas['QuoteRequest'], 'custom_fields'> & { custom_fields?: Record<string, unknown> | null }
export type EmailAddressInput = Schemas['EmailAddressInput']
export type SendQuoteRequest = Schemas['SendQuoteRequest']
export type QuoteSettings = Omit<Schemas['QuoteSettingsResponse'], 'accept_stages'> & { accept_stages: Record<string, string> }
export type QuoteSettingsRequest = Omit<Schemas['QuoteSettingsRequest'], 'accept_stages'> & {
  accept_stages?: Record<string, string | null> | null
}

export interface AcceptQuoteRequest {
  name?: string
  email?: string
  note?: string
}
