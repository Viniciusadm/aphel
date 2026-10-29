import type { RouteObject } from 'react-router'

export const forecastRoutes: RouteObject[] = [
  { path: 'forecast', lazy: async () => ({ Component: (await import('./ForecastPage')).ForecastPage }) },
]

export const forecastSettingsRoute: RouteObject = {
  path: 'forecast',
  lazy: async () => ({ Component: (await import('./settings/ForecastSettingsPage')).ForecastSettingsPage }),
}
