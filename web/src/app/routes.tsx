import type { RouteObject } from 'react-router'
import { RouteErrorBoundary } from '@/components/feedback'
import { NotFoundPage } from '@/features/not-found/NotFoundPage'
import { buildSettingsRoutes } from '@/features/settings/routes'
import { HomeFallback } from '@/kernel/modules/HomeFallback'
import { modulePublicRoutes, moduleRoutes } from '@/kernel/modules/routes'
import { AppLayout } from './layout/AppLayout'
import { KeybindingOutlet } from './layout/KeybindingOutlet'
import './modules'
import { RequireAuth } from './session/RequireAuth'
import { UnknownRouteShell } from './UnknownRouteShell'

export function createRoutes(): RouteObject[] {
  return [
    ...modulePublicRoutes(),
    {
      element: <RequireAuth />,
      children: [
        {
          element: <KeybindingOutlet />,
          children: [
            ...moduleRoutes('bare'),
            {
              element: <AppLayout />,
              children: [
                {
                  errorElement: <RouteErrorBoundary />,
                  children: [...moduleRoutes('app'), ...buildSettingsRoutes(), { path: 'records/:recordType/new', lazy: async () => ({ Component: (await import('@/features/records/form/RecordCreatePage')).RecordCreatePage }) }, { path: 'records/:recordType/:recordId/edit', lazy: async () => ({ Component: (await import('@/features/records/form/RecordEditPage')).RecordEditPage }) }, { index: true, element: <HomeFallback /> }],
                },
              ],
            },
          ],
        },
      ],
    },
    {
      element: <UnknownRouteShell />,
      errorElement: <RouteErrorBoundary />,
      children: [{ path: '*', element: <NotFoundPage /> }],
    },
  ]
}
