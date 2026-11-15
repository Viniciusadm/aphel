import { type DraftGroup, emptyGroup, type FieldSchema, fieldViolations, serializeFilter, toDraft } from '@/sdk'
import {
  type CreateScoringModelRequest,
  type ScoringCriterion,
  type ScoringCriterionKind,
  type ScoringCriterionRequest,
  type ScoringModel,
  type ScoringSignal,
  type UpdateScoringModelRequest,
} from './types'

export const MAX_POINTS = 1000
export const MAX_CAP = 100000
export const MAX_DECAY_DAYS = 3650
export const MAX_CRITERIA = 50
export const NAME_MAX_LENGTH = 120
export const DESCRIPTION_MAX_LENGTH = 500
export const LABEL_MAX_LENGTH = 120

const KEY_PATTERN = /^[a-z](?:_?[a-z0-9])*$/

let criterionSequence = 0

export type FormError = 'required' | 'invalidKey' | 'points' | 'cap' | 'decay' | 'bands' | 'filter' | 'integer'

export type FormErrors = Record<string, FormError | string>

export interface CriterionDraft {
  uid: string
  key: string
  label: string
  kind: ScoringCriterionKind
  filter: DraftGroup
  signal: ScoringSignal
  points: string
  maxPoints: string
  decayDays: string
}

export interface ModelDraft {
  key: string
  name: string
  description: string
  recordType: string
  isPrimary: boolean
  isActive: boolean
  warmFrom: string
  hotFrom: string
  criteria: CriterionDraft[]
}

function nextUid(): string {
  criterionSequence += 1
  return `scoring-criterion-${criterionSequence}`
}

export function newCriterion(kind: ScoringCriterionKind): CriterionDraft {
  return {
    uid: nextUid(),
    key: '',
    label: '',
    kind,
    filter: emptyGroup(),
    signal: 'email_opened',
    points: kind === 'profile' ? '10' : '5',
    maxPoints: '',
    decayDays: kind === 'behavior' ? '30' : '',
  }
}

function criterionDraft(criterion: ScoringCriterion): CriterionDraft {
  return {
    uid: nextUid(),
    key: criterion.key,
    label: criterion.label,
    kind: criterion.kind,
    filter: toDraft(criterion.filter),
    signal: criterion.signal ?? 'email_opened',
    points: String(criterion.points),
    maxPoints: criterion.max_points == null ? '' : String(criterion.max_points),
    decayDays: criterion.decay_days == null ? '' : String(criterion.decay_days),
  }
}

export function draftFromModel(model: ScoringModel): ModelDraft {
  return {
    key: model.key,
    name: model.name,
    description: model.description ?? '',
    recordType: model.record_type,
    isPrimary: model.is_primary,
    isActive: model.is_active,
    warmFrom: String(model.warm_from),
    hotFrom: String(model.hot_from),
    criteria: [...model.criteria].sort((left, right) => left.position - right.position).map(criterionDraft),
  }
}

function parseInteger(value: string): number | null {
  const trimmed = value.trim()
  if (!trimmed) return null
  const number = Number(trimmed)
  return Number.isInteger(number) ? number : Number.NaN
}

export function validateDraft(draft: ModelDraft, creating: boolean): FormErrors {
  const errors: FormErrors = {}
  if (!draft.name.trim()) errors.name = 'required'
  if (creating && draft.key.trim() && !KEY_PATTERN.test(draft.key.trim())) errors.key = 'invalidKey'
  const warm = parseInteger(draft.warmFrom)
  const hot = parseInteger(draft.hotFrom)
  if (warm === null || Number.isNaN(warm)) errors.warm_from = 'integer'
  if (hot === null || Number.isNaN(hot)) errors.hot_from = 'integer'
  else if (warm !== null && !Number.isNaN(warm) && hot <= warm) errors.hot_from = 'bands'
  draft.criteria.forEach((criterion, index) => {
    const path = (field: string) => `criteria.${index}.${field}`
    if (!criterion.label.trim()) errors[path('label')] = 'required'
    if (criterion.key.trim() && !KEY_PATTERN.test(criterion.key.trim())) errors[path('key')] = 'invalidKey'
    const points = parseInteger(criterion.points)
    if (points === null || Number.isNaN(points) || points === 0 || Math.abs(points) > MAX_POINTS)
      errors[path('points')] = 'points'
    if (criterion.kind === 'behavior') {
      const cap = parseInteger(criterion.maxPoints)
      if (cap !== null && (Number.isNaN(cap) || cap < 1 || cap > MAX_CAP)) errors[path('max_points')] = 'cap'
      const decay = parseInteger(criterion.decayDays)
      if (decay !== null && (Number.isNaN(decay) || decay < 1 || decay > MAX_DECAY_DAYS))
        errors[path('decay_days')] = 'decay'
    }
  })
  return errors
}

function criterionRequest(criterion: CriterionDraft, schema: FieldSchema | undefined): ScoringCriterionRequest {
  const base = {
    ...(criterion.key.trim() ? { key: criterion.key.trim() } : {}),
    label: criterion.label.trim(),
    kind: criterion.kind,
    points: Number(criterion.points.trim()),
  }
  if (criterion.kind === 'profile') {
    return { ...base, filter: schema ? serializeFilter(criterion.filter, schema) : null }
  }
  return {
    ...base,
    signal: criterion.signal,
    max_points: parseInteger(criterion.maxPoints),
    decay_days: parseInteger(criterion.decayDays),
  }
}

export function createRequest(draft: ModelDraft, schema: FieldSchema | undefined): CreateScoringModelRequest {
  return {
    record_type: draft.recordType,
    ...(draft.key.trim() ? { key: draft.key.trim() } : {}),
    name: draft.name.trim(),
    description: draft.description.trim() || null,
    is_primary: draft.isPrimary,
    is_active: draft.isActive,
    warm_from: Number(draft.warmFrom.trim()),
    hot_from: Number(draft.hotFrom.trim()),
    criteria: draft.criteria.map((criterion) => criterionRequest(criterion, schema)),
  }
}

export function updateRequest(draft: ModelDraft, schema: FieldSchema | undefined): UpdateScoringModelRequest {
  const request = createRequest(draft, schema)
  return {
    name: request.name,
    description: request.description,
    is_primary: request.is_primary,
    is_active: request.is_active,
    warm_from: request.warm_from,
    hot_from: request.hot_from,
    criteria: request.criteria,
  }
}

export function serverErrors(details: unknown): FormErrors {
  const errors: FormErrors = {}
  for (const violation of fieldViolations(details)) {
    const match = /^criteria\.(\d+)\.filter/.exec(violation.field)
    const field = match ? `criteria.${match[1]}.filter` : violation.field
    errors[field] ??= violation.message
  }
  return errors
}
