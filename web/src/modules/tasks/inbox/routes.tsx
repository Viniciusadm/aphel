import type { RouteObject } from 'react-router'

export const inboxRoutes: RouteObject[] = [
  {
    path: 'inbox',
    lazy: async () => ({ Component: (await import('./InboxPage')).InboxPage }),
  },
]
