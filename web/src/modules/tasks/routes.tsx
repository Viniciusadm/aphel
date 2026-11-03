import type { RouteObject } from 'react-router'
import './resource'

export const activitiesRoutes: RouteObject[] = [
  {
    path: 'tasks',
    lazy: async () => ({ Component: (await import('./TasksLayout')).TasksLayout }),
    children: [
      { index: true, lazy: async () => ({ Component: (await import('./MyTasksPage')).MyTasksPage }) },
      { path: 'all', lazy: async () => ({ Component: (await import('./AllActivitiesPage')).AllActivitiesPage }) },
    ],
  },
  {
    path: 'today',
    lazy: async () => ({ Component: (await import('./mobile/TodayPage')).TodayPage }),
  },
  {
    path: 'tasks/:id',
    lazy: async () => ({ Component: (await import('./TaskDetailPage')).TaskDetailPage }),
  },
]

export const activityTypesSettingsRoute: RouteObject = {
  path: 'activity-types',
  lazy: async () => ({ Component: (await import('./settings/ActivityTypesPage')).ActivityTypesPage }),
}
