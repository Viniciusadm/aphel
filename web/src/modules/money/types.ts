import type { components } from '@/api'

type Schemas = components['schemas']

export type ExchangeRateSource = Schemas['ExchangeRateSource']
export type ExchangeRate = Schemas['ExchangeRateResponse']
export type Currency = Schemas['CurrencyResponse']
export type CurrencySettings = Schemas['CurrencySettingsResponse']
export type ExchangeRatesPage = Schemas['Page_ExchangeRateResponse']
export type CreateExchangeRateRequest = Schemas['CreateExchangeRateRequest']
export type UpdateExchangeRateRequest = Schemas['UpdateExchangeRateRequest']
export type RatesProvider = Schemas['RatesProviderResponse']
export type RatesProviderKey = 'manual' | 'frankfurter'
export type UpdateRatesProviderRequest = Schemas['UpdateRatesProviderRequest']
export type ExchangeRatesSync = Schemas['ExchangeRatesSyncResponse']
