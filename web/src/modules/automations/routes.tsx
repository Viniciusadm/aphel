import type { RouteObject } from 'react-router'

export const automationsRoutes: RouteObject[] = [
  {
    path: 'automations',
    children: [
      { index: true, lazy: async () => ({ Component: (await import('./AutomationsPage')).AutomationsPage }) },
      { path: ':id', lazy: async () => ({ Component: (await import('./AutomationPage')).AutomationPage }) },
    ],
  },
]
