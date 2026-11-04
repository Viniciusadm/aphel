import type { RouteObject } from 'react-router'

export const bookingRoutes: RouteObject[] = [
  {
    path: 'booking',
    lazy: async () => ({ Component: (await import('./BookingPage')).BookingPage }),
    children: [
      { index: true, lazy: async () => ({ Component: (await import('./bookings/BookingsPanel')).BookingsPanel }) },
      { path: 'availability', lazy: async () => ({ Component: (await import('./availability/AvailabilityPanel')).AvailabilityPanel }) },
      { path: 'meeting-types', lazy: async () => ({ Component: (await import('./meeting-types/MeetingTypesPanel')).MeetingTypesPanel }) },
    ],
  },
]
