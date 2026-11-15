import type { RouteObject } from 'react-router'

export const trashSettingsRoute: RouteObject = {
  path: 'trash',
  lazy: async () => ({ Component: (await import('./TrashPage')).TrashPage }),
}

export const trashRoutes: RouteObject[] = [{ ...trashSettingsRoute }]
