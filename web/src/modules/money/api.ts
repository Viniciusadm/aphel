import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type {
  CreateExchangeRateRequest,
  CurrencySettings,
  ExchangeRate,
  ExchangeRatesPage,
  ExchangeRatesSync,
  UpdateExchangeRateRequest,
  UpdateRatesProviderRequest,
} from './types'

export const RATES_PER_PAGE = 25

export const currenciesKeys = {
  all: ['currencies'] as const,
  settings: () => [...currenciesKeys.all, 'settings'] as const,
  rates: (currency: string, page: number) => [...currenciesKeys.all, 'rates', currency, page] as const,
}

export const currenciesApi = {
  settings: () => api.get<CurrencySettings>('/v1/money/currencies'),
  setCurrencies: (currencies: string[]) => api.put<CurrencySettings>('/v1/money/currencies', { currencies }),
  listRates: (currency: string, page: number) =>
    api.get<ExchangeRatesPage>('/v1/money/currencies/rates', {
      query: { currency: currency || undefined, page, per_page: RATES_PER_PAGE },
    }),
  createRate: (body: CreateExchangeRateRequest) => api.post<ExchangeRate>('/v1/money/currencies/rates', body),
  updateRate: (id: string, body: UpdateExchangeRateRequest, version: string) =>
    api.patch<ExchangeRate>(`/v1/money/currencies/rates/${encodeURIComponent(id)}`, body, {
      headers: { 'If-Match': version },
    }),
  deleteRate: (id: string) => api.delete<void>(`/v1/money/currencies/rates/${encodeURIComponent(id)}`),
  updateProvider: (body: UpdateRatesProviderRequest) => api.put<CurrencySettings>('/v1/money/currencies/provider', body),
  syncRates: () => api.post<ExchangeRatesSync>('/v1/money/currencies/rates/sync'),
}

export function useCurrencySettings(enabled = true) {
  return useQuery({
    queryKey: currenciesKeys.settings(),
    queryFn: currenciesApi.settings,
    staleTime: 60_000,
    enabled,
  })
}

export function useExchangeRates(currency: string, page: number) {
  return useQuery({
    queryKey: currenciesKeys.rates(currency, page),
    queryFn: () => currenciesApi.listRates(currency, page),
    placeholderData: keepPreviousData,
  })
}

export function useInvalidateCurrencies() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: currenciesKeys.all })
}

export { currencyErrorKey } from './errors'
