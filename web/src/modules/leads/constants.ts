import type { FilterNode } from '@/features/records/types'
import type { TransitionTrigger } from './types'

export const LEADS_PATH = '/leads'
export const LEADS_REPORT_PATH = '/leads/report'
export const LIFECYCLE_SETTINGS_PATH = 'lifecycle'
export const LIFECYCLE_MANAGE_PERMISSION = 'lifecycle.manage'
export const CONTACT_RECORD_TYPE = 'contact'
export const LIFECYCLE_STAGE_FIELD = 'lifecycle_stage'
export const LEAD_STAGE = 'lead'
export const QUALIFIED_STAGE = 'qualified'
export const LIFECYCLE_TAB_KEY = 'lifecycle'
export const MAX_NOTE_LENGTH = 2000
export const MAX_REASON_NAME_LENGTH = 140
export const DEFAULT_REPORT_DAYS = 90

export const TRANSITION_TRIGGERS: TransitionTrigger[] = ['deal_created', 'deal_won', 'all_deals_lost']

export const LEAD_INBOX_FILTER: FilterNode = {
  field: LIFECYCLE_STAGE_FIELD,
  operator: 'eq',
  value: LEAD_STAGE,
}

export function isOpenLead(stage: unknown): boolean {
  return stage === LEAD_STAGE || stage === QUALIFIED_STAGE
}
