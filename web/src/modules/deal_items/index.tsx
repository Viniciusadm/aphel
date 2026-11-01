import { Repeat, ShoppingCart } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { defineModule } from '@/kernel/modules/define'
import { productsSettingsRoute } from '@/modules/products/routes'
import { AmountFieldLock } from './AmountFieldLock'
import { DealItemsRecordTab, DealRevenueRecordTab } from './record-tabs'
import { RecurringRevenueWidget } from './recurring/RecurringRevenueWidget'
import type { RecurringRevenueData } from './recurring/types'
import { DEAL_ITEM_TIMELINE_RENDERERS } from './timeline'

const DEAL_RECORD_TYPE = 'deal'

const dealItemsModule = defineModule({
  key: 'deal_items',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('organization', {
      group: 'sales',
      path: 'products',
      label: 'settings:tabs.products',
      description: 'settings:items.products.description',
      icon: ShoppingCart,
      permission: SETTINGS_MANAGE,
      routes: [productsSettingsRoute],
    })
    app.recordSection(DEAL_RECORD_TYPE, {
      key: 'items',
      label: 'items.tab',
      icon: ShoppingCart,
      order: 20,
      render: DealItemsRecordTab,
    })
    app.recordSection(DEAL_RECORD_TYPE, {
      key: 'revenue',
      label: 'panel.tab',
      icon: Repeat,
      order: 40,
      render: DealRevenueRecordTab,
    })
    app.slot('deals.detail.fieldLocks', { key: 'amount', render: AmountFieldLock })
    app.widget('recurring_revenue', {
      render: (data) => <RecurringRevenueWidget data={data as RecurringRevenueData} />,
    })
    for (const [eventType, renderer] of Object.entries(DEAL_ITEM_TIMELINE_RENDERERS))
      app.eventRenderer(eventType, renderer)
  },
})

export default dealItemsModule
