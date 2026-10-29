import type { RouteObject } from 'react-router'

export const segmentsRoutes: RouteObject[] = [
  { path: 'segments', lazy: async () => ({ Component: (await import('./SegmentsPage')).SegmentsPage }) },
  { path: 'segments/:id', lazy: async () => ({ Component: (await import('./SegmentDetailPage')).SegmentDetailPage }) },
]
