import type { RouteObject } from 'react-router'

export const sequencesRoutes: RouteObject[] = [
  {
    path: 'sequences',
    children: [
      { index: true, lazy: async () => ({ Component: (await import('./SequencesPage')).SequencesPage }) },
      { path: 'new', lazy: async () => ({ Component: (await import('./SequenceEditorPage')).SequenceEditorPage }) },
      { path: ':id', lazy: async () => ({ Component: (await import('./SequenceDetailPage')).SequenceDetailPage }) },
      { path: ':id/edit', lazy: async () => ({ Component: (await import('./SequenceEditorPage')).SequenceEditorPage }) },
    ],
  },
]
