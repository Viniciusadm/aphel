import type { RouteObject } from 'react-router'

export const duplicatesSettingsRoutes: RouteObject[] = [
  { path: 'duplicates', lazy: async () => ({ Component: (await import('./DuplicatesPage')).DuplicatesPage }) },
  { path: 'duplicates/merge', lazy: async () => ({ Component: (await import('./MergePage')).MergePage }) },
]
