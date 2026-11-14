import { Plug } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'

const integrationsModule = defineModule({
  key: 'integrations',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('integrations', {
      group: 'connections',
      path: 'connections',
      label: 'settings:tabs.integrations',
      description: 'settings:items.integrations.description',
      icon: Plug,
      permission: SETTINGS_MANAGE,
      order: 900,
      lazy: async () => ({ Component: (await import('./IntegrationsPage')).IntegrationsPage }),
    })
  },
})

export default integrationsModule
