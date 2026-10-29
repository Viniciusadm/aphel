import type { RouteObject } from 'react-router'

export const systemRoutes: RouteObject[] = [
  { path: 'system', lazy: async () => ({ Component: (await import('./SystemPage')).SystemPage }) },
]
