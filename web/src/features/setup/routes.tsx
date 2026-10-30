import { Navigate, type RouteObject } from 'react-router'
import { SetupPage } from './SetupPage'

export const setupRoutes: RouteObject[] = [
  { path: 'setup', element: <SetupPage /> },
  { path: 'install', element: <Navigate to="/setup" replace /> },
]
