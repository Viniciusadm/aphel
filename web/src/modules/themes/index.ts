import { Palette } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { CatalogThemes } from './CatalogThemes'
import { THEMES_MANAGE } from './permissions'
import { themesSettingsRoute } from './routes'

const themesModule = defineModule({
  key: 'themes',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('organization', {
      group: 'organization',
      path: 'themes',
      label: 'settings:tabs.themes',
      description: 'settings:items.themes.description',
      icon: Palette,
      permission: THEMES_MANAGE,
      order: 800,
      routes: [themesSettingsRoute],
    })
    app.slot('catalog.sections', { key: 'themes', permission: THEMES_MANAGE, render: CatalogThemes })
  },
})

export default themesModule
