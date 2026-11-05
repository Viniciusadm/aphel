import { Coins } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { currenciesSettingsRoute } from './routes'

const currenciesModule = defineModule({
  key: 'money',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('organization', {
      group: 'sales',
      path: 'currencies',
      label: 'settings:tabs.currencies',
      description: 'settings:items.currencies.description',
      icon: Coins,
      permission: SETTINGS_MANAGE,
      routes: [currenciesSettingsRoute],
    })
  },
})

export default currenciesModule
