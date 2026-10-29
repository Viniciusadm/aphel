import { Workflow } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { automationsRoutes } from './routes'

const automationsModule = defineModule({
  key: 'automations',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(automationsRoutes)
    app.nav({
      to: '/automations',
      label: 'nav.automations',
      icon: Workflow,
      permission: 'automations.manage',
      order: 200,
    })
  },
})

export default automationsModule
