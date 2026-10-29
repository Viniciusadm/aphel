import { fieldViolations } from '@/features/records/fields'
import { emptyGroup, serializeFilter, toDraft, type DraftGroup } from '@/features/records/filters'
import type { FieldDefinition, FieldSchema } from '@/features/records/types'
import { identifierFrom } from './identifier'
import { MAX_DUE_HOURS, MIN_DUE_HOURS, QUOTE_RECORD_TYPE, STAGE_RECORD_TYPES } from '../constants'
import type {
  ApprovalMoment,
  ApprovalPolicy,
  ApprovalPolicyRequest,
  ApprovalRule,
  ApproverKind,
  ConditionKind,
} from '../types'

export const CUSTOM_FIELD_PREFIX = 'custom_fields.'

let stepSequence = 0

export interface StepDraft {
  uid: string
  name: string
  approverKind: ApproverKind
  approverUserId: string
  approverRoleId: string
  rule: ApprovalRule
}

export interface PolicyDraft {
  key: string
  keyTouched: boolean
  name: string
  description: string
  recordType: string
  moment: ApprovalMoment
  pipelineId: string
  targetId: string
  conditionKind: ConditionKind
  filter: DraftGroup
  formula: string
  lockedFields: string[]
  dueHours: string
  isActive: boolean
  steps: StepDraft[]
}

export type FormErrors = Record<string, string>

export function newStep(): StepDraft {
  stepSequence += 1
  return {
    uid: `approval-step-${stepSequence}`,
    name: '',
    approverKind: 'owner_team_manager',
    approverUserId: '',
    approverRoleId: '',
    rule: 'any',
  }
}

export function keyFromName(name: string): string {
  return identifierFrom(name)
}

export function emptyDraft(recordType: string): PolicyDraft {
  return {
    key: '',
    keyTouched: false,
    name: '',
    description: '',
    recordType,
    moment: 'on_demand',
    pipelineId: '',
    targetId: '',
    conditionKind: 'always',
    filter: emptyGroup(),
    formula: '',
    lockedFields: [],
    dueHours: '',
    isActive: true,
    steps: [newStep()],
  }
}

export function draftFromPolicy(policy: ApprovalPolicy, pipelineOfStage: (stageId: string) => string): PolicyDraft {
  return {
    key: policy.key,
    keyTouched: true,
    name: policy.name,
    description: policy.description ?? '',
    recordType: policy.record_type,
    moment: policy.moment,
    pipelineId: policy.target_id ? pipelineOfStage(policy.target_id) : '',
    targetId: policy.target_id ?? '',
    conditionKind: policy.condition_kind,
    filter: toDraft(policy.condition_filter),
    formula: policy.condition_formula ?? '',
    lockedFields: [...policy.locked_fields],
    dueHours: policy.due_hours == null ? '' : String(policy.due_hours),
    isActive: policy.is_active,
    steps: [...policy.steps]
      .sort((left, right) => left.position - right.position)
      .map((step) => ({
        ...newStep(),
        name: step.name ?? '',
        approverKind: step.approver_kind,
        approverUserId: step.approver_user_id ?? '',
        approverRoleId: step.approver_role_id ?? '',
        rule: step.rule,
      })),
  }
}

export function availableMoments(recordType: string): ApprovalMoment[] {
  const moments: ApprovalMoment[] = []
  if (STAGE_RECORD_TYPES.includes(recordType)) moments.push('stage_change')
  if (recordType === QUOTE_RECORD_TYPE) moments.push('quote_send')
  moments.push('on_demand')
  return moments
}

export function lockableKey(field: FieldDefinition): string {
  return field.is_system ? field.key : `${CUSTOM_FIELD_PREFIX}${field.key}`
}

export function validateDraft(draft: PolicyDraft, creating: boolean): FormErrors {
  const errors: FormErrors = {}
  if (!draft.name.trim()) errors.name = 'required'
  if (creating && !/^[a-z](?:_?[a-z0-9])*$/.test(draft.key)) errors.key = 'invalidKey'
  if (!draft.recordType) errors.record_type = 'required'
  if (draft.moment === 'stage_change' && !draft.targetId) errors.target_id = 'required'
  if (draft.conditionKind === 'formula' && !draft.formula.trim()) errors.condition_formula = 'required'
  const due = draft.dueHours.trim()
  if (due) {
    const hours = Number(due)
    if (!Number.isInteger(hours) || hours < MIN_DUE_HOURS || hours > MAX_DUE_HOURS) errors.due_hours = 'dueRange'
  }
  draft.steps.forEach((step, index) => {
    if (step.approverKind === 'user' && !step.approverUserId) errors[`steps.${index}.approver_user_id`] = 'required'
    if (step.approverKind === 'role' && !step.approverRoleId) errors[`steps.${index}.approver_role_id`] = 'required'
  })
  return errors
}

export function requestFromDraft(
  draft: PolicyDraft,
  schema: FieldSchema | undefined,
  creating: boolean,
): ApprovalPolicyRequest {
  const due = draft.dueHours.trim()
  const filter = draft.conditionKind === 'filter' && schema ? serializeFilter(draft.filter, schema) : null
  return {
    ...(creating ? { key: draft.key } : {}),
    name: draft.name.trim(),
    description: draft.description.trim() || null,
    record_type: draft.recordType,
    moment: draft.moment,
    target_id: draft.moment === 'stage_change' ? draft.targetId || null : null,
    condition_kind: draft.conditionKind,
    condition_filter: draft.conditionKind === 'filter' ? filter : null,
    condition_formula: draft.conditionKind === 'formula' ? draft.formula.trim() : null,
    locked_fields: draft.lockedFields,
    due_hours: due ? Number(due) : null,
    is_active: draft.isActive,
    steps: draft.steps.map((step) => ({
      name: step.name.trim() || null,
      approver_kind: step.approverKind,
      approver_user_id: step.approverKind === 'user' ? step.approverUserId || null : null,
      approver_role_id: step.approverKind === 'role' ? step.approverRoleId || null : null,
      rule: step.rule,
    })),
  }
}

export function serverErrors(details: unknown): FormErrors {
  const errors: FormErrors = {}
  for (const violation of fieldViolations(details)) {
    errors[violation.field] ??= violation.message
  }
  return errors
}
