import type { RouteObject } from 'react-router'

export const publicQuoteRoutes: RouteObject[] = [
  {
    path: 'quote/:token',
    lazy: async () => {
      const [{ PublicQuotePage }, { ensureQuoteTranslations }] = await Promise.all([
        import('./PublicQuotePage'),
        import('./translations'),
      ])
      await ensureQuoteTranslations()
      return { Component: PublicQuotePage }
    },
  },
]
