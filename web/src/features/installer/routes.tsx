import type { RouteObject } from 'react-router'
import { AuthLayout } from '@/features/auth/components/AuthLayout'
import { InstallPage } from './InstallPage'

export const installerRoutes: RouteObject[] = [
  {
    element: <AuthLayout />,
    children: [{ path: 'install', element: <InstallPage /> }],
  },
]
