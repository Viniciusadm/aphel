import type { RouteObject } from 'react-router'
import { Overridable } from '@/kernel/modules/Overridable'
import './resource'

export const contactsRoutes: RouteObject[] = [
  {
    path: 'contacts',
    children: [
      {
        index: true,
        lazy: async () => {
          const { ContactsPage } = await import('./ContactsPage')
          return { element: <Overridable id="contacts.ListPage" component={ContactsPage} /> }
        },
      },
      {
        path: ':id',
        lazy: async () => {
          const { ContactDetailPage } = await import('./ContactDetailPage')
          return { element: <Overridable id="contacts.DetailPage" component={ContactDetailPage} /> }
        },
      },
    ],
  },
]
