import type { components } from '@/api'
import { type FilterNode } from '@/sdk'
type Schemas = components['schemas']

export type { RecordsPage as Page, PageMeta } from '@/sdk'

export type ApprovalMoment = Schemas['ApprovalMoment']
export type ApprovalMomentInfo = Schemas['ApprovalMomentResponse']
export type ApprovalStatus = Schemas['ApprovalStatus']
export type ApproverKind = Schemas['ApproverKind']
export type ApprovalRule = Schemas['ApprovalRule']
export type ConditionKind = Schemas['ApprovalConditionKind']
export type ApprovalStepState = Schemas['ApprovalStepState']
export type ApprovalDecision = 'approved' | 'rejected'
export type ApprovalBox = 'pending_for_me' | 'requested_by_me' | 'all'
export type SubmissionOutcome = 'not_required' | 'pending' | 'approved'

export type ApprovalPolicyStep = Schemas['ApprovalPolicyStepResponse']
export type ApprovalPolicy = Omit<Schemas['ApprovalPolicyResponse'], 'condition_filter'> & {
  condition_filter?: FilterNode | null
}
export type ApprovalPolicyStepRequest = Schemas['ApprovalPolicyStepRequest']
export type ApprovalPolicyRequest = Omit<Schemas['ApprovalPolicyRequest'], 'condition_filter'> & {
  condition_filter?: FilterNode | null
}
export type ApprovalPersonRef = Schemas['ApprovalUserResponse']
export type ApprovalApprover = Omit<Schemas['ApprovalApproverResponse'], 'decision'> & {
  decision?: ApprovalDecision | null
}
export type ApprovalRequestStep = Omit<Schemas['ApprovalStepResponse'], 'approvers'> & { approvers: ApprovalApprover[] }
export type ApprovalRequest = Omit<Schemas['ApprovalRequestResponse'], 'steps'> & { steps: ApprovalRequestStep[] }
export type ApprovalSubmissionRequest = Schemas['ApprovalSubmitRequest']
export type ApprovalSubmission = Omit<Schemas['ApprovalSubmissionResponse'], 'outcome' | 'requests'> & {
  outcome: SubmissionOutcome
  requests: ApprovalRequest[]
}
export type ApprovalPendingCount = Schemas['ApprovalPendingCountResponse']

export interface ApprovalPolicyRef {
  id: string
  key: string
  name: string
}

export interface ApprovalRequiredDetails {
  record_type: string
  record_id: string
  moment: ApprovalMoment
  target_id: string | null
  policies: ApprovalPolicyRef[]
}

export interface ApprovalPendingDetails {
  request_id: string
  record_type?: string
  record_id?: string
  moment?: ApprovalMoment
  target_id?: string | null
  fields?: string[]
}
