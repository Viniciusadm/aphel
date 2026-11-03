import type { RouteObject } from 'react-router'
import { Overridable } from '@/kernel/modules/Overridable'
import './resource'

export const contactsRoutes: RouteObject[] = [
  {
    path: 'people',
    children: [
      {
        index: true,
        lazy: async () => {
          const { ContactsPage } = await import('./ContactsPage')
          return { element: <Overridable id="people.ListPage" component={ContactsPage} /> }
        },
      },
      {
        path: ':id',
        lazy: async () => {
          const { ContactDetailPage } = await import('./ContactDetailPage')
          return { element: <Overridable id="people.DetailPage" component={ContactDetailPage} /> }
        },
      },
    ],
  },
]
