import { KeyRound } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { SSO_KEY } from './api'
import { SsoSettingsPanel } from './SettingsPanel'

const ssoModule = defineModule({
  key: SSO_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  settingsPanel: SsoSettingsPanel,
  register(app) {
    app.routes([
      {
        path: 'sso',
        children: [{ index: true, lazy: async () => ({ Component: (await import('./SsoPage')).SsoPage }) }],
      },
    ])
    app.nav({ to: '/sso', label: 'nav', icon: KeyRound, permission: ['settings.manage'] })
  },
})

export default ssoModule
