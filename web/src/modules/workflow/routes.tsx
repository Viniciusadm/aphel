import type { RouteObject } from 'react-router'

export const pipelinesSettingsRoute: RouteObject = {
  path: 'workflow',
  lazy: async () => ({ Component: (await import('./PipelinesSettingsPage')).PipelinesSettingsPage }),
}
