import { api } from '@/api'
import type {EsignRequest} from '@/sdk'
import type {ProposalDocument} from '@/sdk'

export const PROPOSALS_MODULE = 'proposals'
export const ESIGN_MODULE = 'esign'
export const PRICE_LISTS_MODULE = 'pricing'

const id = encodeURIComponent

export interface QuoteSigner {
  name: string
  email: string
  contact_id?: string
}

export function generateQuotePdf(dealId: string, quoteId: string, templateId: string | null) {
  return api.post<ProposalDocument>(`/v1/proposals/deals/${id(dealId)}/documents`, {
    template_id: templateId ?? undefined,
    quote_id: quoteId,
  })
}

export function sendQuoteForSignature(
  dealId: string,
  quoteId: string,
  fileId: string,
  signers: QuoteSigner[],
  message: string | null,
) {
  return api.post<EsignRequest>('/v1/esign/requests', {
    deal_id: dealId,
    file_id: fileId,
    quote_id: quoteId,
    signers,
    message: message ?? undefined,
  })
}
