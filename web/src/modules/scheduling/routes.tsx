import type { RouteObject } from 'react-router'

export const schedulingRoutes: RouteObject[] = [
  {
    path: 'scheduling',
    lazy: async () => ({ Component: (await import('./SchedulingPage')).SchedulingPage }),
  },
]
