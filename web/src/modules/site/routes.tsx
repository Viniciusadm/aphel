import type { RouteObject } from 'react-router'

export const siteSettingsRoute: RouteObject = {
  path: 'site',
  lazy: async () => ({ Component: (await import('./SiteSettingsPage')).SiteSettingsPage }),
}
