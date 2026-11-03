import type { RouteObject } from 'react-router'

export const dealReportRoutes: RouteObject[] = [
  { path: 'deals/reports/conversion', lazy: async () => ({ Component: (await import('./ConversionReportPage')).ConversionReportPage }) },
  { path: 'deals/reports/performance', lazy: async () => ({ Component: (await import('./PerformanceReportPage')).PerformanceReportPage }) },
]
