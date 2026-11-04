import { Boxes } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { BundleRecordTab } from './bundles/BundleRecordTab'
import { PriceListsLink } from './PriceListsLink'
import { priceListRoutes } from './routes'

export { PRICE_LISTS_PATH, priceListPath, priceListsApi, priceListsKeys, useActivePriceLists } from './api'
export type { DiscountLimit, PriceList, PriceListAppliesTo, PriceListItem } from './types'

const PRODUCT_RECORD_TYPE = 'catalog_items.item'
const BUNDLE_TAB_KEY = 'bundle'

const priceListsModule = defineModule({
  key: 'pricing',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(priceListRoutes)
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
