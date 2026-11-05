import { TrendingUp } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { forecastRoutes, forecastSettingsRoute } from './routes'

const forecastModule = defineModule({
  key: 'forecast',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settingsGroup('organization', { key: 'sales', label: 'groups.sales', order: 20 })
    app.routes(forecastRoutes)
    app.nav({ section: 'analysis', to: '/forecast', label: 'nav.forecast', icon: TrendingUp, permission: ['deals.read'], order: 150 })
    app.settings('organization', {
      group: 'sales',
      path: 'forecast',
      label: 'settings:tabs.forecast',
      description: 'settings:items.forecast.description',
      icon: TrendingUp,
      permission: SETTINGS_MANAGE,
      routes: [forecastSettingsRoute],
    })
  },
})

export default forecastModule
