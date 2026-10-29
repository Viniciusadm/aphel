import { BarChart3 } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { lazyComponent } from '@/lib/lazy-component'
import type { SavedReportWidgetData } from './builder/SavedReportWidget'
import { reportsRoutes } from './routes'

const SavedReportWidget = lazyComponent(async () => (await import('./builder/SavedReportWidget')).SavedReportWidget)

const reportsModule = defineModule({
  key: 'reports',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(reportsRoutes)
    app.nav({
      to: '/reports',
      label: 'nav.reports',
      icon: BarChart3,
      permission: ['contacts.read', 'companies.read', 'deals.read', 'activities.read'],
      order: 130,
    })
    app.widget('saved_report', {
      render: (data) => <SavedReportWidget data={data as SavedReportWidgetData} />,
      settings: [
        {
          kind: 'boolean',
          key: 'use_dashboard_period',
          label: 'reports:builder.widget.useDashboardPeriod',
        },
      ],
    })
  },
})

export default reportsModule
