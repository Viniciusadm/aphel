import type { RouteObject } from 'react-router'

export const pipelinesSettingsRoute: RouteObject = {
  path: 'pipelines',
  lazy: async () => ({ Component: (await import('./PipelinesSettingsPage')).PipelinesSettingsPage }),
}
