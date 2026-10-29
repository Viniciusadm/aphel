import type { RouteObject } from 'react-router'

export const navigationSettingsRoute: RouteObject = {
  path: 'navigation',
  lazy: async () => ({ Component: (await import('./NavigationSettingsPage')).NavigationSettingsPage }),
}
