import { KeyRound } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { OIDC_KEY } from './api'
import { OidcSettingsPanel } from './SettingsPanel'

const oidcModule = defineModule({
  key: OIDC_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  settingsPanel: OidcSettingsPanel,
  register(app) {
    app.routes([
      {
        path: 'oidc',
        children: [{ index: true, lazy: async () => ({ Component: (await import('./OidcPage')).OidcPage }) }],
      },
    ])
    app.nav({ to: '/oidc', label: 'nav', icon: KeyRound, permission: ['settings.manage'] })
  },
})

export default oidcModule
