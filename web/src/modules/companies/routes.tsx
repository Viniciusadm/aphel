import type { RouteObject } from 'react-router'
import { Overridable } from '@/kernel/modules/Overridable'
import './resource'

export const companiesRoutes: RouteObject[] = [
  {
    path: 'companies',
    children: [
      {
        index: true,
        lazy: async () => {
          const { CompaniesPage } = await import('./CompaniesPage')
          return { element: <Overridable id="companies.ListPage" component={CompaniesPage} /> }
        },
      },
      {
        path: ':id',
        lazy: async () => {
          const { CompanyDetailPage } = await import('./CompanyDetailPage')
          return { element: <Overridable id="companies.DetailPage" component={CompanyDetailPage} /> }
        },
      },
    ],
  },
]
