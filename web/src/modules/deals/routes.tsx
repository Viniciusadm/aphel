import type { RouteObject } from 'react-router'
import './resource'

export const dealsRoutes: RouteObject[] = [
  {
    path: 'deals',
    children: [
      { index: true, lazy: async () => ({ Component: (await import('./DealsPage')).DealsPage }) },
      { path: ':id', lazy: async () => ({ Component: (await import('./detail/DealDetailPage')).DealDetailPage }) },
    ],
  },
]
