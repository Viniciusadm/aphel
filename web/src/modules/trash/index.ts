import { Trash2 } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { trashSettingsRoute } from './routes'

const trashModule = defineModule({
  key: 'trash',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('data', {
      group: 'compliance',
      path: 'trash',
      label: 'settings:tabs.trash',
      description: 'settings:items.trash.description',
      icon: Trash2,
      permission: SETTINGS_MANAGE,
      order: 110,
      routes: [trashSettingsRoute],
    })
  },
})

export default trashModule
