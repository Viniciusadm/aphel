import type { RouteObject } from 'react-router'

export const builtinReportRoutes: RouteObject[] = [
  { path: 'conversion', lazy: async () => ({ Component: (await import('./ConversionReportPage')).ConversionReportPage }) },
  { path: 'performance', lazy: async () => ({ Component: (await import('./PerformanceReportPage')).PerformanceReportPage }) },
]
