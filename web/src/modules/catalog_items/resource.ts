import { Package } from 'lucide-react'
import { registerRecordResource } from '@/sdk'
export const PRODUCT_RECORD_TYPE = 'catalog_items.item'
export const PRODUCTS_PATH = '/catalog-items'

export function productPath(id: string) {
  return `${PRODUCTS_PATH}/${encodeURIComponent(id)}`
}

export const productResource = registerRecordResource({
  relatable: true,
  icon: Package,
  recordType: PRODUCT_RECORD_TYPE,
  createPermission: 'catalog_items.create',
  updatePermission: 'catalog_items.update',
  basePath: '/v1/catalog-items',
  summaryFields: ['code', 'price', 'currency', 'unit', 'recurrence', 'is_active'],
  titleField: 'name',
  subtitleField: 'code',
  detailPath: productPath,
  createDefaults: ({ currency }) => ({ currency, recurrence: 'none', is_active: true }),
})

export const PRODUCT_CREATE_LAYOUT = ['name', 'code', 'price', 'currency', 'unit', 'recurrence', 'term_months', 'is_active']

export const PRODUCT_READ_ONLY_KEYS = ['id', 'created_at', 'updated_at', 'deleted_at', 'created_by']
