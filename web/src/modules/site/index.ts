import { Gauge, Globe, Languages, ListTree, Palette, PanelsTopLeft, Settings2, Shuffle, UserRound } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { SITE_OVERVIEW_PATH, SITE_SCREENS, siteRoutes } from './routes'

const ICONS = {
  navigation: ListTree,
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
  namespaces: { 'site-blocks': { 'pt-BR': () => import('./i18n/blocks.pt-BR.json'), en: () => import('./i18n/blocks.en.json') } },
  register(app) {
    app.routes(siteRoutes)
    app.nav({
      to: SITE_OVERVIEW_PATH,
      label: 'nav.overview',
      icon: Gauge,
      permission: SETTINGS_MANAGE,
      section: 'operations',
      group: { key: 'site', label: 'nav.site', icon: Globe },
      order: 38,
    })
    SITE_SCREENS.forEach((screen, index) =>
      app.nav({
        to: screen.path,
        label: `nav.${screen.key}`,
        icon: ICONS[screen.key],
        permission: screen.key === 'appearance' || screen.key === 'frame' ? ['site.appearance.edit', 'site.presentation.edit'] : SETTINGS_MANAGE,
        section: 'operations',
        group: { key: 'site', label: 'nav.site', icon: Globe },
        order: 41 + index,
      }),
    )
  },
})

export default siteModule
