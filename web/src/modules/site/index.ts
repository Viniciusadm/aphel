import { Globe } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { siteSettingsRoute } from './routes'

const siteModule = defineModule({
  key: 'site',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('organization', {
      group: 'organization',
      path: 'site',
      label: 'site:settings.label',
      description: 'site:settings.description',
      icon: Globe,
      permission: SETTINGS_MANAGE,
      routes: [siteSettingsRoute],
    })
  },
})

export default siteModule
