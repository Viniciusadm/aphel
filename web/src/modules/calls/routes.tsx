import type { RouteObject } from 'react-router'

export const callsRoutes: RouteObject[] = [
  {
    path: 'calls',
    lazy: async () => ({ Component: (await import('./CallsLayout')).CallsLayout }),
    children: [
      { index: true, lazy: async () => ({ Component: (await import('./SessionsPage')).SessionsPage }) },
      { path: 'report', lazy: async () => ({ Component: (await import('./ReportPage')).ReportPage }) },
      { path: 'outcomes', lazy: async () => ({ Component: (await import('./OutcomesPage')).OutcomesPage }) },
    ],
  },
  {
    path: 'calls/sessions/:id',
    lazy: async () => ({ Component: (await import('./SessionPage')).SessionPage }),
  },
]
