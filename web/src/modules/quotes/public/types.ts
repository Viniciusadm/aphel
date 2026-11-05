import type { components } from '@/modules/quotes/api-types'

type Schemas = components['schemas']

export type PublicQuoteState = Schemas['PublicQuoteState']
export type PublicQuoteItem = Schemas['PublicQuoteItemResponse']
export type PublicQuote = Schemas['PublicQuoteResponse']
