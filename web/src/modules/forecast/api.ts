import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type ForecastReport = Schemas['ForecastReportResponse']
export type ForecastMonth = Schemas['ForecastMonthResponse']
export type ForecastSummary = Schemas['ForecastSummaryResponse']
export type ForecastSettings = Schemas['ForecastSettingsResponse']
export type ForecastMethod = Schemas['ForecastMethod']
export type ForecastCategory = Schemas['ForecastCategory']
export type ForecastDeal = Schemas['ForecastDealResponse']
export type ForecastDealsPage = Schemas['Page_ForecastDealResponse']
export type UpdateForecastSettings = Schemas['UpdateForecastSettingsRequest']

export const FORECAST_METHODS: ForecastMethod[] = ['deal_probability', 'stage_probability', 'commit_only']
export const FORECAST_CATEGORIES: ForecastCategory[] = ['pipeline', 'best_case', 'commit']
export const HORIZON_OPTIONS = [3, 6, 12, 18]
export const MAX_HORIZON_MONTHS = 18
export const FORECAST_DEALS_PER_PAGE = 20

export interface ForecastDealsParams {
  month: string
  category?: ForecastCategory
  page: number
}

export const forecastKeys = {
  all: ['forecast'] as const,
  report: (months: number | null) => [...forecastKeys.all, 'report', months] as const,
  deals: (params: ForecastDealsParams) => [...forecastKeys.all, 'deals', params] as const,
  settings: () => [...forecastKeys.all, 'settings'] as const,
}

export const forecastApi = {
  report: (months: number | null) =>
    api.get<ForecastReport>('/v1/forecast', { query: { months: months ?? undefined } }),
  deals: (params: ForecastDealsParams) =>
    api.get<ForecastDealsPage>('/v1/forecast/deals', {
      query: {
        month: params.month,
        category: params.category,
        page: params.page,
        per_page: FORECAST_DEALS_PER_PAGE,
      },
    }),
  settings: () => api.get<ForecastSettings>('/v1/forecast/settings'),
  updateSettings: (body: UpdateForecastSettings, version?: string | null) =>
    api.patch<ForecastSettings>('/v1/forecast/settings', body, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
}

export function useForecast(months: number | null) {
  return useQuery({
    queryKey: forecastKeys.report(months),
    queryFn: () => forecastApi.report(months),
    placeholderData: keepPreviousData,
  })
}

export function useForecastDeals(params: ForecastDealsParams | null) {
  return useQuery({
    queryKey: forecastKeys.deals(params ?? { month: '', page: 1 }),
    queryFn: () => forecastApi.deals(params as ForecastDealsParams),
    enabled: params !== null,
    placeholderData: keepPreviousData,
  })
}

export function useForecastSettings() {
  return useQuery({ queryKey: forecastKeys.settings(), queryFn: forecastApi.settings })
}

export function useInvalidateForecast() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: forecastKeys.all })
}

export function categoryAmount(summary: ForecastSummary, category: ForecastCategory) {
  return summary.categories?.find((item) => item.category === category) ?? null
}
