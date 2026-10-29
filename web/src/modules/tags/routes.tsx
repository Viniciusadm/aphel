import type { RouteObject } from 'react-router'

export const tagsSettingsRoute: RouteObject = {
  path: 'tags',
  lazy: async () => ({ Component: (await import('./TagsSettingsPage')).TagsSettingsPage }),
}
