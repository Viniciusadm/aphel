import { CalendarDays } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { calendarRoutes } from './routes'
import './slots'

export default defineModule({
  key: 'calendar',
  translations: {
    en: () => import('./i18n/en.json'),
    'pt-BR': () => import('./i18n/pt-BR.json'),
  },
  register(app) {
    app.routes(calendarRoutes)
    app.nav({ section: 'work', to: '/calendar', label: 'navigation', icon: CalendarDays, permission: ['calendar.read'], order: 110 })
    app.keybinding({ id: 'calendar.go', label: 'navigation', to: '/calendar', keys: ['g a'], permission: ['calendar.read'] })
  },
})
