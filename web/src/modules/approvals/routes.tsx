import type { RouteObject } from 'react-router'

export const approvalsRoutes: RouteObject[] = [
  {
    path: 'approvals',
    children: [
      {
        index: true,
        lazy: async () => ({
          Component: (await import('./ApprovalsPage')).ApprovalsPage,
        }),
      },
      {
        path: ':id',
        lazy: async () => ({
          Component: (await import('./ApprovalDetailPage')).ApprovalDetailPage,
        }),
      },
    ],
  },
]
