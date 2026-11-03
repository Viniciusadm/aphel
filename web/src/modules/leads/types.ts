import type { components } from '@/api'

type Schemas = components['schemas']

export type TransitionTrigger = 'deal_created' | 'deal_won' | 'all_deals_lost'
export type LifecycleRecordType = 'people.person' | 'organizations.organization'

export type LifecycleStageOption = Schemas['LifecycleStageOptionResponse']
export type LifecycleTransitionRule = Omit<Schemas['LifecycleTransitionRuleResponse'], 'trigger'> & {
  trigger: TransitionTrigger
}
export type LifecycleSettings = Omit<Schemas['LifecycleSettingsResponse'], 'rules'> & { rules: LifecycleTransitionRule[] }
export type LifecycleSettingsRequest = Schemas['LifecycleSettingsRequest']
export type DisqualificationReason = Schemas['DisqualificationReasonResponse']
export type CreateDisqualificationReasonRequest = Schemas['CreateDisqualificationReasonRequest']
export type UpdateDisqualificationReasonRequest = Schemas['UpdateDisqualificationReasonRequest']
export type LifecycleHistoryEntry = Schemas['LifecycleHistoryEntryResponse']
export type LifecycleStageReport = Schemas['LifecycleStageReportResponse']
export type LifecycleDisqualificationCount = Schemas['LifecycleDisqualificationCountResponse']
export type LifecycleReport = Omit<Schemas['LifecycleReportResponse'], 'record_type'> & {
  record_type: LifecycleRecordType
}
export type DisqualifyLeadRequest = Schemas['DisqualifyLeadRequest']
export type ConvertLeadRequest = Schemas['ConvertLeadRequest']
export type LeadActionResult = Schemas['LeadActionResponse']

export interface LifecycleReportQuery {
  record_type: LifecycleRecordType
  from?: string
  to?: string
}
