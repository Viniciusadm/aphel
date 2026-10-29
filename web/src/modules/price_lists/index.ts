import { Boxes } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { BundleRecordTab } from './bundles/BundleRecordTab'
import { PriceListsLink } from './PriceListsLink'
import { priceListRoutes } from './routes'

export { PRICE_LISTS_PATH, priceListPath, priceListsApi, priceListsKeys, useActivePriceLists } from './api'
export type { DiscountLimit, PriceList, PriceListAppliesTo, PriceListItem } from './types'

const PRODUCT_RECORD_TYPE = 'product'
const BUNDLE_TAB_KEY = 'bundle'

const priceListsModule = defineModule({
  key: 'price_lists',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(priceListRoutes)
    app.slot('products.list.header', { key: 'price-lists', permission: ['products.read'], render: PriceListsLink })
    app.recordTab(PRODUCT_RECORD_TYPE, {
      key: BUNDLE_TAB_KEY,
      label: 'bundles.tab',
      icon: Boxes,
      order: 10,
      render: BundleRecordTab,
    })
  },
})

export default priceListsModule
