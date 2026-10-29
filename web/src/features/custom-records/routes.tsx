import type { RouteObject } from 'react-router'

export const customRecordsRoutes: RouteObject[] = [
  {
    path: 'objects/:key',
    children: [
      { index: true, lazy: async () => ({ Component: (await import('./CustomRecordsPage')).CustomRecordsPage }) },
      {
        path: ':id',
        lazy: async () => ({ Component: (await import('./CustomRecordDetailPage')).CustomRecordDetailPage }),
      },
    ],
  },
]
