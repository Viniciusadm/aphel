import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { API_BASE_URL, api } from '@/api'
import type {
  Page,
  AcceptQuoteRequest,
  QuoteDetail,
  QuoteRequest,
  QuoteSettings,
  QuoteSettingsRequest,
  QuoteSummary,
  QuoteVersion,
  SendQuoteRequest,
} from './types'

export const QUOTES_PATH = '/quotes'
export const QUOTES_PER_PAGE = 25

const id = encodeURIComponent

export function quotePath(quoteId: string): string {
  return `${QUOTES_PATH}/${id(quoteId)}`
}

export function quotePdfUrl(quoteId: string): string {
  return `${API_BASE_URL}/v1/quotes/${id(quoteId)}/pdf`
}

function ifMatch(version?: string | null): HeadersInit | undefined {
  return version ? { 'If-Match': version } : undefined
}

export const quotesKeys = {
  all: ['quotes'] as const,
  lists: () => [...quotesKeys.all, 'list'] as const,
  list: (page: number, search: string, status: string) => [...quotesKeys.lists(), page, search, status] as const,
  deal: (dealId: string) => [...quotesKeys.all, 'deal', dealId] as const,
  detail: (quoteId: string) => [...quotesKeys.all, 'detail', quoteId] as const,
  versions: (quoteId: string) => [...quotesKeys.all, 'versions', quoteId] as const,
  settings: () => [...quotesKeys.all, 'settings'] as const,
}

export const quotesApi = {
  list: (page: number, search: string, status: string) =>
    api.get<Page<QuoteSummary>>('/v1/quotes', {
      query: {
        page,
        per_page: QUOTES_PER_PAGE,
        sort: '-number',
        search: search || undefined,
        'filter[status][eq]': status || undefined,
      },
    }),
  forDeal: (dealId: string) => api.get<QuoteSummary[]>(`/v1/deals/${id(dealId)}/quotes`),
  get: (quoteId: string) => api.get<QuoteDetail>(`/v1/quotes/${id(quoteId)}`),
  create: (request: QuoteRequest) => api.post<QuoteDetail>('/v1/quotes', request),
  update: (quoteId: string, request: QuoteRequest, version?: string | null) =>
    api.patch<QuoteDetail>(`/v1/quotes/${id(quoteId)}`, request, { headers: ifMatch(version) }),
  remove: (quoteId: string) => api.delete<void>(`/v1/quotes/${id(quoteId)}`),
  versions: (quoteId: string) => api.get<QuoteVersion[]>(`/v1/quotes/${id(quoteId)}/versions`),
  createVersion: (quoteId: string) => api.post<QuoteDetail>(`/v1/quotes/${id(quoteId)}/versions`),
  submit: (quoteId: string, comment: string | null) =>
    api.post<QuoteDetail>(`/v1/quotes/${id(quoteId)}/submit`, { comment: comment ?? undefined }),
  send: (quoteId: string, request: SendQuoteRequest) =>
    api.post<QuoteDetail>(`/v1/quotes/${id(quoteId)}/send`, request),
  accept: (quoteId: string, request: AcceptQuoteRequest) =>
    api.post<QuoteDetail>(`/v1/quotes/${id(quoteId)}/accept`, request),
  link: (quoteId: string) => api.get<{ url: string }>(`/v1/quotes/${id(quoteId)}/link`),
  settings: () => api.get<QuoteSettings>('/v1/quote-settings'),
  updateSettings: (request: QuoteSettingsRequest, version?: string | null) =>
    api.patch<QuoteSettings>('/v1/quote-settings', request, { headers: ifMatch(version) }),
}

export function useQuotes(page: number, search: string, status: string) {
  return useQuery({
    queryKey: quotesKeys.list(page, search, status),
    queryFn: () => quotesApi.list(page, search, status),
    placeholderData: keepPreviousData,
  })
}

export function useDealQuotes(dealId: string) {
  return useQuery({ queryKey: quotesKeys.deal(dealId), queryFn: () => quotesApi.forDeal(dealId), enabled: Boolean(dealId) })
}

export function useQuote(quoteId: string) {
  return useQuery({ queryKey: quotesKeys.detail(quoteId), queryFn: () => quotesApi.get(quoteId), enabled: Boolean(quoteId) })
}

export function useQuoteSettings(enabled = true) {
  return useQuery({ queryKey: quotesKeys.settings(), queryFn: quotesApi.settings, enabled })
}

export function useQuoteMutations() {
  const client = useQueryClient()
  const refresh = (quote?: QuoteDetail) => {
    if (quote) client.setQueryData(quotesKeys.detail(quote.id), quote)
    void client.invalidateQueries({ queryKey: quotesKeys.all })
    void client.invalidateQueries({ queryKey: ['approvals'] })
  }
  return {
    create: useMutation({ mutationFn: quotesApi.create, onSuccess: refresh }),
    update: useMutation({
      mutationFn: ({ quoteId, request, version }: { quoteId: string; request: QuoteRequest; version?: string | null }) =>
        quotesApi.update(quoteId, request, version),
      onSuccess: refresh,
    }),
    remove: useMutation({ mutationFn: quotesApi.remove, onSuccess: () => refresh() }),
    createVersion: useMutation({ mutationFn: quotesApi.createVersion, onSuccess: refresh }),
    submit: useMutation({
      mutationFn: ({ quoteId, comment }: { quoteId: string; comment: string | null }) => quotesApi.submit(quoteId, comment),
      onSuccess: refresh,
    }),
    send: useMutation({
      mutationFn: ({ quoteId, request }: { quoteId: string; request: SendQuoteRequest }) => quotesApi.send(quoteId, request),
      onSuccess: refresh,
    }),
    accept: useMutation({
      mutationFn: ({ quoteId, request }: { quoteId: string; request: AcceptQuoteRequest }) => quotesApi.accept(quoteId, request),
      onSuccess: refresh,
    }),
    updateSettings: useMutation({
      mutationFn: ({ request, version }: { request: QuoteSettingsRequest; version?: string | null }) =>
        quotesApi.updateSettings(request, version),
      onSuccess: () => void client.invalidateQueries({ queryKey: quotesKeys.settings() }),
    }),
  }
}
