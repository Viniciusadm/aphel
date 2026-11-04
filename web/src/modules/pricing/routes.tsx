import type { RouteObject } from 'react-router'

export const priceListRoutes: RouteObject[] = [
  {
    path: 'pricing',
    children: [
      { index: true, lazy: async () => ({ Component: (await import('./PriceListsPage')).PriceListsPage }) },
      { path: ':id', lazy: async () => ({ Component: (await import('./PriceListDetailPage')).PriceListDetailPage }) },
    ],
  },
]
