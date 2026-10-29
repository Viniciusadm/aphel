import type { RouteObject } from 'react-router'

export const followingRoutes: RouteObject[] = [
  {
    path: 'following',
    lazy: async () => ({ Component: (await import('./FollowingPage')).FollowingPage }),
  },
]
