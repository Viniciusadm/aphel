import type { RouteObject } from 'react-router'

export const goalsRoutes: RouteObject[] = [
  { path: 'goals', lazy: async () => ({ Component: (await import('./GoalsPage')).GoalsPage }) },
]
