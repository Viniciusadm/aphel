import type { RouteObject } from 'react-router'

export const reminderRoutes: RouteObject[] = [
  {
    path: 'reminders',
    lazy: async () => ({ Component: (await import('./RemindersPage')).RemindersPage }),
  },
]
