import { CalendarDays } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { MICROSOFT_CALENDAR_PERMISSION, MICROSOFT_KEY } from './api'
import { SSO_SHARED_KEY } from '@/modules/sso/shared/api'
import { MicrosoftSettingsPanel } from './SettingsPanel'

const microsoftModule = defineModule({
  key: MICROSOFT_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  namespaces: { [SSO_SHARED_KEY]: { 'pt-BR': () => import('@/modules/sso/i18n/pt-BR.json'), en: () => import('@/modules/sso/i18n/en.json') } },
  settingsPanel: MicrosoftSettingsPanel,
  register(app) {
    app.routes([
      {
        path: 'microsoft',
        children: [{ index: true, lazy: async () => ({ Component: (await import('./MicrosoftPage')).MicrosoftPage }) }],
      },
    ])
    app.nav({ section: 'administration',
      to: '/microsoft',
      label: 'nav',
      icon: CalendarDays,
      permission: [MICROSOFT_CALENDAR_PERMISSION, 'settings.manage'],
    })
  },
})

export default microsoftModule
