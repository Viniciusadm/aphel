import type { RouteObject } from 'react-router'

export const bookingRoutes: RouteObject[] = [
  {
    path: 'book',
    children: [
      {
        path: 'manage/:token',
        lazy: async () => ({ Component: (await import('./ManageBookingPage')).ManageBookingPage }),
      },
      { path: ':slug', lazy: async () => ({ Component: (await import('./BookingPage')).BookingPage }) },
    ],
  },
]
