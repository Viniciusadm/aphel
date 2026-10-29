import type { components } from '@/api'

type Schemas = components['schemas']

export type PublicQuoteState = Schemas['PublicQuoteState']
export type PublicQuoteItem = Schemas['PublicQuoteItemResponse']
export type PublicQuote = Schemas['PublicQuoteResponse']
