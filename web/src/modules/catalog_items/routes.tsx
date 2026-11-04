import type { RouteObject } from 'react-router'
import './resource'

export const productsRoutes: RouteObject[] = [
  {
    path: 'catalog-items',
    children: [
      { index: true, lazy: async () => ({ Component: (await import('./ProductsPage')).ProductsPage }) },
      { path: ':id', lazy: async () => ({ Component: (await import('./ProductDetailPage')).ProductDetailPage }) },
    ],
  },
]
