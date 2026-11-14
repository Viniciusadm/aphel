import { KeyRound } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { SSO_KEY } from './api'
import { SsoCard } from './IntegrationCard'
import { SsoSettingsPanel } from './SettingsPanel'

const ssoModule = defineModule({
  key: SSO_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  settingsPanel: SsoSettingsPanel,
  register(app) {
    app.slot('integrations.cards', { key: 'login', order: 400, permission: ['settings.manage'], render: SsoCard })
    app.settings('integrations', {
      group: 'connections',
      path: 'sso',
      label: 'nav',
      description: 'page.menuDescription',
      icon: KeyRound,
      permission: 'settings.manage',
      lazy: async () => ({ Component: (await import('./SsoPage')).SsoPage }),
    })
  },
})

export default ssoModule
