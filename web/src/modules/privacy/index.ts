import { Lock } from 'lucide-react'
import { PRIVACY_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'

const privacyModule = defineModule({
  key: 'privacy',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('data', {
      group: 'compliance',
      path: 'privacy',
      label: 'settings:tabs.privacy',
      description: 'settings:items.privacy.description',
      icon: Lock,
      permission: PRIVACY_MANAGE,
      order: 120,
      lazy: async () => ({ Component: (await import('./PrivacyPage')).PrivacyPage }),
    })
  },
})

export default privacyModule
