import type { RouteObject } from 'react-router'
import './resource'

export const productsRoutes: RouteObject[] = [
  {
    path: 'products',
    children: [
      { index: true, lazy: async () => ({ Component: (await import('./ProductsPage')).ProductsPage }) },
      { path: ':id', lazy: async () => ({ Component: (await import('./ProductDetailPage')).ProductDetailPage }) },
    ],
  },
]

export const productsSettingsRoute: RouteObject = {
  path: 'products',
  lazy: async () => ({ Component: (await import('./settings/DealItemSettingsPage')).DealItemSettingsPage }),
}
