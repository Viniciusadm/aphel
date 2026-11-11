import type { RouteObject } from 'react-router'
import { ensureBookingTranslations } from './translations'

export const bookingRoutes: RouteObject[] = [
  {
    path: 'book',
    children: [
      {
        path: 'manage/:token',
        lazy: async () => {
          const [{ ManageBookingPage }] = await Promise.all([import('./ManageBookingPage'), ensureBookingTranslations()])
          return { Component: ManageBookingPage }
        },
      },
      {
        path: ':slug',
        lazy: async () => {
          const [{ BookingPage }] = await Promise.all([import('./BookingPage'), ensureBookingTranslations()])
          return { Component: BookingPage }
        },
      },
    ],
  },
]
