import type { RouteObject } from 'react-router'

export const objectsSettingsRoutes: RouteObject[] = [
  { path: 'objects', lazy: async () => ({ Component: (await import('./ObjectsSettingsPage')).ObjectsSettingsPage }) },
  {
    path: 'objects/:key',
    lazy: async () => ({ Component: (await import('./ObjectSettingsDetailPage')).ObjectSettingsDetailPage }),
  },
]
