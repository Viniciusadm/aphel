import React from 'react'
import { ProductPicker } from './ProductPicker'
import { Package } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { productsRoutes } from './routes'
import './slots'

export { productsApi, productsKeys, useProductPicker, type Product, type Recurrence } from './api'
export { ProductPicker, type ProductPickerProps } from './ProductPicker'
export { PRODUCT_RECORD_TYPE, PRODUCTS_PATH, productPath } from './resource'

const productsModule = defineModule({
  key: 'catalog_items',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.slot('line_items.catalogPicker', { key: 'catalog', render: ({ currency, disabled, onSelect }) => React.createElement(ProductPicker, { currency, disabled, onSelect: (product) => onSelect(product.id) }) })
    app.routes(productsRoutes)
    app.nav({ to: '/catalog-items', label: 'nav.products', icon: Package, permission: ['catalog_items.read'], order: 60 })
  },
})

export default productsModule
