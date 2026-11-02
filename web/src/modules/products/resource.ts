import { registerRecordResource } from '@/features/records/resources'

export const PRODUCT_RECORD_TYPE = 'product'
export const PRODUCTS_PATH = '/products'

export function productPath(id: string) {
  return `${PRODUCTS_PATH}/${encodeURIComponent(id)}`
}

export const productResource = registerRecordResource({
  recordType: PRODUCT_RECORD_TYPE,
  createPermission: 'products.create',
  updatePermission: 'products.update',
  basePath: '/v1/products',
  summaryFields: ['code', 'price', 'currency', 'unit', 'recurrence', 'is_active'],
  titleField: 'name',
  subtitleField: 'code',
  detailPath: productPath,
})

export const PRODUCT_CREATE_LAYOUT = ['name', 'code', 'price', 'currency', 'unit', 'recurrence', 'term_months', 'is_active']

export const PRODUCT_READ_ONLY_KEYS = ['id', 'created_at', 'updated_at', 'deleted_at', 'created_by']
