import { CalendarClock } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { bookingRoutes } from './public/routes'
import { schedulingRoutes } from './routes'

const schedulingModule = defineModule({
  key: 'scheduling',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.publicRoutes(bookingRoutes)
    app.routes(schedulingRoutes)
    app.nav({
      to: '/scheduling',
      label: 'nav.scheduling',
      icon: CalendarClock,
      permission: ['scheduling.manage'],
      order: 170,
    })
  },
})

export default schedulingModule
