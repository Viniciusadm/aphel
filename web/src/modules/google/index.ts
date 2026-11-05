import { CalendarDays } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { GOOGLE_CALENDAR_PERMISSION, GOOGLE_KEY } from './api'
import {SSO_SHARED_KEY} from '@/sdk'
import { GoogleSettingsPanel } from './SettingsPanel'

const googleModule = defineModule({
  key: GOOGLE_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  namespaces: { [SSO_SHARED_KEY]: { 'pt-BR': () => import('@/modules/sso/i18n/pt-BR.json'), en: () => import('@/modules/sso/i18n/en.json') } },
  settingsPanel: GoogleSettingsPanel,
  register(app) {
    app.routes([
      {
        path: 'google',
        children: [{ index: true, lazy: async () => ({ Component: (await import('./GooglePage')).GooglePage }) }],
      },
    ])
    app.nav({ section: 'administration',
      to: '/google',
      label: 'nav',
      icon: CalendarDays,
      permission: [GOOGLE_CALENDAR_PERMISSION, 'settings.manage'],
    })
  },
})

export default googleModule
