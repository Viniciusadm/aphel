import type { RouteObject } from 'react-router'
import { customReportRoutes } from './builder/routes'
import { ReportsIndex, ReportsLayout } from './ReportsLayout'

export const reportsRoutes: RouteObject[] = [
  {
    path: 'reports',
    element: <ReportsLayout />,
    children: [{ index: true, element: <ReportsIndex /> }, ...customReportRoutes],
  },
]
