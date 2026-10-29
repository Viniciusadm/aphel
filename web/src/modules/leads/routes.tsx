import type { RouteObject } from 'react-router'
import { Overridable } from '@/kernel/modules/Overridable'
import { LIFECYCLE_SETTINGS_PATH } from './constants'

export const leadsRoutes: RouteObject[] = [
  {
    path: 'leads',
    children: [
      {
        index: true,
        lazy: async () => {
          const { LeadsPage } = await import('./LeadsPage')
          return { element: <Overridable id="leads.Page" component={LeadsPage} /> }
        },
      },
      { path: 'report', lazy: async () => ({ Component: (await import('./LifecycleReportPage')).LifecycleReportPage }) },
    ],
  },
]

export const lifecycleSettingsRoute: RouteObject = {
  path: LIFECYCLE_SETTINGS_PATH,
  lazy: async () => ({ Component: (await import('./settings/LifecycleSettingsPage')).LifecycleSettingsPage }),
}
