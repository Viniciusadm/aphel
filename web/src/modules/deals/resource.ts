import { dealsKeys } from './api'
import { registerRecordResource } from '@/features/records/resources'

export const DEAL_RECORD_TYPE = 'deal'
export const DEALS_PATH = '/deals'

export function dealPath(id: string) {
  return `${DEALS_PATH}/${encodeURIComponent(id)}`
}

export const dealResource = registerRecordResource({
  recordType: DEAL_RECORD_TYPE,
  createPermission: 'deals.create',
  updatePermission: 'deals.update',
  collectionQueryKeys: [dealsKeys.boards()],
  basePath: '/v1/deals',
  summaryFields: ['stage_id', 'amount', 'currency', 'expected_close_date', 'contact_id', 'company_id', 'owner_id'],
  titleField: 'title',
  subtitleField: 'stage_id',
  detailPath: dealPath,
})

export const DEAL_CREATE_LAYOUT = ['title', 'amount', 'expected_close_date', 'contact_id', 'company_id', 'owner_id']

export const DEAL_READ_ONLY_KEYS = [
  'id',
  'created_at',
  'updated_at',
  'archived_at',
  'deleted_at',
  'created_by',
  'status',
  'pipeline_id',
  'closed_at',
  'stage_entered_at',
  'last_activity_at',
  'lost_reason_id',
  'currency',
  'amount_base',
  'exchange_rate',
  'mrr',
  'arr',
  'tcv',
  'renewal_of_deal_id',
  'days_in_stage',
  'is_rotting',
  'rotting_at',
]
