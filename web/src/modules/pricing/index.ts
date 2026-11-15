import { Boxes, Percent, Tags } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { BundleRecordTab } from './bundles/BundleRecordTab'
import { PriceListsLink } from './PriceListsLink'
import { SETTINGS_MANAGE } from '@/sdk'
import { priceListRoutes } from './routes'


const PRODUCT_RECORD_TYPE = 'catalog_items.item'
const BUNDLE_TAB_KEY = 'bundle'

const priceListsModule = defineModule({
  key: 'pricing',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(priceListRoutes)
    app.nav({ section: 'records', to: '/pricing', label: 'pricing:nav.priceLists', icon: Tags, permission: ['catalog_items.read'], order: 61 })
    app.settings('organization', {
      group: 'sales',
      path: 'discount-limits',
      label: 'pricing:nav.discountLimits',
      description: 'pricing:nav.discountLimitsDescription',
      icon: Percent,
      permission: SETTINGS_MANAGE,
      lazy: async () => ({ Component: (await import('./DiscountLimitsPage')).DiscountLimitsPage }),
    })
    app.slot('catalog_items.list.header', { key: 'pricing', permission: ['catalog_items.read'], render: PriceListsLink })
    app.recordSection(PRODUCT_RECORD_TYPE, {
      key: BUNDLE_TAB_KEY,
      label: 'bundles.tab',
      icon: Boxes,
      order: 10,
      render: BundleRecordTab,
    })
  },
})

export default priceListsModule
