import { isApiError } from '@/api'
import { fieldViolations } from '@/features/records/fields'
import { REQUIRABLE_SYSTEM_FIELDS, requiredStageFields } from '@/modules/pipelines/requirements'
import type { PipelineStage } from '@/modules/pipelines/types'
import type { Deal } from './types'

export const STAGE_REQUIREMENTS_MISSING = 'stage_requirements_missing'
export const STAGE_REQUIREMENTS_INACCESSIBLE = 'stage_requirements_inaccessible'

const SYSTEM_KEYS = new Set<string>(REQUIRABLE_SYSTEM_FIELDS)

export function isFilled(value: unknown): boolean {
  if (value === null || value === undefined) return false
  if (typeof value === 'string') return value.trim() !== ''
  if (Array.isArray(value)) return value.length > 0
  if (typeof value === 'object') return Object.keys(value).length > 0
  return true
}

export function dealFieldValue(deal: Deal, key: string): unknown {
  if (SYSTEM_KEYS.has(key)) return (deal as unknown as Record<string, unknown>)[key]
  return (deal.custom_fields as Record<string, unknown> | undefined)?.[key]
}

export function missingStageFields(deal: Deal, from: PipelineStage | undefined, to: PipelineStage): string[] {
  if (from?.id === to.id) return []
  return requiredStageFields(from, to).filter((key) => !isFilled(dealFieldValue(deal, key)))
}

export function isStageRequirementsInaccessible(error: unknown): boolean {
  return isApiError(error) && error.code === STAGE_REQUIREMENTS_INACCESSIBLE
}

export function requirementFieldsFromError(error: unknown): string[] | null {
  if (!isApiError(error)) return null
  const fields = fieldViolations(error.details).map((violation) => violation.field)
  if (error.code === STAGE_REQUIREMENTS_INACCESSIBLE) return fields
  if (error.code !== STAGE_REQUIREMENTS_MISSING) return null
  return fields.length > 0 ? fields : null
}
