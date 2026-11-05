import { House } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { PeriodSelector } from './PeriodSelector'
import { CompareToggle } from './SubjectSelector'
import { DASHBOARD_WIDGETS } from './widgets/registry'

const dashboardModule = defineModule({
  key: 'dashboard',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes([
      {
        index: true,
        lazy: async () => ({ Component: (await import('./DashboardPage')).DashboardPage }),
      },
    ])
    app.provideComponent('dashboard.PeriodSelector', PeriodSelector)
    app.provideComponent('dashboard.CompareToggle', CompareToggle)
    app.nav({ section: 'work', to: '/', label: 'nav.home', icon: House, end: true, order: 10 })
    for (const [key, renderer] of Object.entries(DASHBOARD_WIDGETS)) app.widget(key, renderer)
  },
})

export default dashboardModule
