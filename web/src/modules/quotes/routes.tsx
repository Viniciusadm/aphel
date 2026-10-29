import type { RouteObject } from 'react-router'
import './resource'

export const quotesRoutes: RouteObject[] = [
  {
    path: 'quotes',
    children: [
      { index: true, lazy: async () => ({ Component: (await import('./QuotesPage')).QuotesPage }) },
      { path: ':id', lazy: async () => ({ Component: (await import('./QuoteEditorPage')).QuoteEditorPage }) },
    ],
  },
]

export const quotesSettingsRoute: RouteObject = {
  path: 'quotes',
  lazy: async () => ({ Component: (await import('./QuoteSettingsPage')).QuoteSettingsPage }),
}
