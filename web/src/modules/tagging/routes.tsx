import type { RouteObject } from 'react-router'

export const tagsSettingsRoute: RouteObject = {
  path: 'tagging',
  lazy: async () => ({ Component: (await import('./TagsSettingsPage')).TagsSettingsPage }),
}
