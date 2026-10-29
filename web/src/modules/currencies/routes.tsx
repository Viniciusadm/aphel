import type { RouteObject } from 'react-router'

export const currenciesSettingsRoute: RouteObject = {
  path: 'currencies',
  lazy: async () => ({ Component: (await import('./CurrenciesSettingsPage')).CurrenciesSettingsPage }),
}
