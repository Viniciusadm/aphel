import type { RouteObject } from 'react-router'

export const searchRoutes: RouteObject[] = [
  { path: 'search', lazy: async () => ({ Component: (await import('@/modules/search/ui/SearchPage')).SearchPage }) },
]
