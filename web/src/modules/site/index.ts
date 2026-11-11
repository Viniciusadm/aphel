import { Globe, Languages, Palette, PanelsTopLeft, Settings2, Shuffle, UserRound } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { SITE_SCREENS, siteRoutes } from './routes'

const ICONS = {
  frame: PanelsTopLeft,
  appearance: Palette,
  languages: Languages,
  customers: UserRound,
  redirects: Shuffle,
  general: Settings2,
} as const

const siteModule = defineModule({
  key: 'site',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(siteRoutes)
    SITE_SCREENS.forEach((screen, index) =>
      app.nav({
        to: screen.path,
        label: `nav.${screen.key}`,
        icon: ICONS[screen.key],
        permission: SETTINGS_MANAGE,
        section: 'operations',
        group: { key: 'site', label: 'nav.site', icon: Globe },
        order: 41 + index,
      }),
    )
  },
})

export default siteModule
