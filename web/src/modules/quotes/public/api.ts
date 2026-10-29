import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { PublicQuote } from './types'

const id = encodeURIComponent

export const publicQuoteKeys = {
  quote: (token: string) => ['public-quote', token] as const,
}

export const publicQuoteApi = {
  get: (token: string) => api.get<PublicQuote>(`/v1/public/quotes/${id(token)}`),
  accept: (token: string, name: string, email: string) =>
    api.post<PublicQuote>(`/v1/public/quotes/${id(token)}/accept`, { name, email }),
  reject: (token: string, reason: string | null) =>
    api.post<PublicQuote>(`/v1/public/quotes/${id(token)}/reject`, { reason: reason ?? undefined }),
}

export function usePublicQuote(token: string) {
  return useQuery({ queryKey: publicQuoteKeys.quote(token), queryFn: () => publicQuoteApi.get(token), retry: false })
}

export function usePublicQuoteAnswer(token: string) {
  const client = useQueryClient()
  const update = (quote: PublicQuote) => client.setQueryData(publicQuoteKeys.quote(token), quote)
  return {
    accept: useMutation({
      mutationFn: ({ name, email }: { name: string; email: string }) => publicQuoteApi.accept(token, name, email),
      onSuccess: update,
    }),
    reject: useMutation({
      mutationFn: (reason: string | null) => publicQuoteApi.reject(token, reason),
      onSuccess: update,
    }),
  }
}
