import { Package } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { defineModule } from '@/kernel/modules/define'
import { productsRoutes, productsSettingsRoute } from './routes'
import './slots'

export { productsApi, productsKeys, useProductPicker, type Product, type ProductRecurrence } from './api'
export { ProductPicker, type ProductPickerProps } from './ProductPicker'
export { PRODUCT_RECORD_TYPE, PRODUCTS_PATH, productPath } from './resource'

const productsModule = defineModule({
  key: 'products',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(productsRoutes)
    app.nav({ to: '/products', label: 'nav.products', icon: Package, permission: ['products.read'], order: 60 })
    app.settings('organization', {
      group: 'sales',
      path: 'products',
      label: 'settings:tabs.products',
      description: 'settings:items.products.description',
      icon: Package,
      permission: SETTINGS_MANAGE,
      routes: [productsSettingsRoute],
    })
  },
})

export default productsModule
