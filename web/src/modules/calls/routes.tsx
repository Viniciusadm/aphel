import { Navigate, type RouteObject } from 'react-router'

export const callsRoutes: RouteObject[] = [
  { path: 'calls/report', lazy: async () => ({ Component: (await import('./CallsReportPage')).CallsReportPage }) },
  { path: 'calls/outcomes', element: <Navigate to="/settings/call-outcomes" replace /> },
  {
    path: 'calls',
    lazy: async () => ({ Component: (await import('./CallsLayout')).CallsLayout }),
    children: [
      { index: true, lazy: async () => ({ Component: (await import('./SessionsPage')).SessionsPage }) },


    ],
  },
  {
    path: 'calls/sessions/:id',
    lazy: async () => ({ Component: (await import('./SessionPage')).SessionPage }),
  },
]
