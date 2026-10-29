import { Navigate, type RouteObject } from 'react-router'

export const importsRoutes: RouteObject[] = [
  {
    path: 'imports',
    children: [
      { index: true, element: <Navigate to="new" replace /> },
      { path: 'new', lazy: async () => ({ Component: (await import('./ImportWizardPage')).ImportWizardPage }) },
      { path: ':id', lazy: async () => ({ Component: (await import('./ImportWizardPage')).ImportWizardPage }) },
    ],
  },
]
