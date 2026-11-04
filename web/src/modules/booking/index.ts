import { CalendarClock } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { bookingRoutes as publicBookingRoutes } from './public/routes'
import { bookingRoutes } from './routes'

const bookingModule = defineModule({
  key: 'booking',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.publicRoutes(publicBookingRoutes)
    app.routes(bookingRoutes)
    app.nav({
      to: '/booking',
      label: 'nav.booking',
      icon: CalendarClock,
      permission: ['booking.manage'],
      order: 170,
    })
  },
})

export default bookingModule
