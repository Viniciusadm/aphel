import { Repeat } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { RevenueTab } from './record-tabs'
import { RecurringRevenueWidget } from './RecurringRevenueWidget'
import type { RecurringRevenueData } from './types'
const dealsRevenueModule = defineModule({
  key: 'deals_revenue',
  translations: { en: () => import('./i18n/en.json'), 'pt-BR': () => import('./i18n/pt-BR.json') },
  register(app) {
    app.recordSection('deals.deal', { key: 'revenue', label: 'panel.tab', icon: Repeat, order: 40, render: RevenueTab })
    app.widget('recurring_revenue', { render: (data) => <RecurringRevenueWidget data={data as RecurringRevenueData} /> })
  },
})

export default dealsRevenueModule
