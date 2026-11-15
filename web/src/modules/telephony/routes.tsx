import { Navigate, type RouteObject } from 'react-router'

export const callsRoutes: RouteObject[] = [
  { path: 'telephony/report', lazy: async () => ({ Component: (await import('./CallsReportPage')).CallsReportPage }) },
  { path: 'telephony/outcomes', element: <Navigate to="/settings/call-outcomes" replace /> },
  { path: 'telephony', lazy: async () => ({ Component: (await import('./SessionsPage')).SessionsPage }) },
  {
    path: 'telephony/sessions/:id',
    lazy: async () => ({ Component: (await import('./SessionPage')).SessionPage }),
  },
]
