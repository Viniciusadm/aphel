import { registerRecordResource } from '@/sdk'
import { quotePath } from './api'

export const QUOTE_RECORD_TYPE = 'quotes.quote'

export const quoteResource = registerRecordResource({
  recordType: QUOTE_RECORD_TYPE,
  basePath: '/v1/quotes',
  titleField: 'title',
  detailPath: quotePath,
})
