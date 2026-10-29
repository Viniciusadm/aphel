import type { RouteObject } from 'react-router'

export const customReportRoutes: RouteObject[] = [
  { path: 'saved', lazy: async () => ({ Component: (await import('./SavedReportsPage')).SavedReportsPage }) },
  { path: 'saved/new', lazy: async () => ({ Component: (await import('./ReportBuilderPage')).ReportBuilderPage }) },
  { path: 'saved/:reportId', lazy: async () => ({ Component: (await import('./ReportBuilderPage')).ReportBuilderPage }) },
]
