import type { RottingDealFields, RottingInfo, RottingStageFields } from './types'

export const MAX_ROTTING_DAYS = 3650
export const IS_ROTTING_FIELD = 'is_rotting'
export const DAYS_IN_STAGE_FIELD = 'days_in_stage'
export const ROTTING_AT_FIELD = 'rotting_at'
export const ROTTING_READ_ONLY_KEYS = [DAYS_IN_STAGE_FIELD, IS_ROTTING_FIELD, ROTTING_AT_FIELD]

export function stageRottingDays(stage: object | null | undefined): number | null {
  const days = (stage as RottingStageFields | null | undefined)?.rotting_days
  return typeof days === 'number' ? days : null
}

function stillInSameEntry(deal: RottingDealFields): boolean {
  if (!deal.rotting_at || !deal.stage_entered_at) return false
  return Date.parse(deal.rotting_at) > Date.parse(deal.stage_entered_at)
}

export function rottingOf(value: object): RottingInfo {
  const deal = value as RottingDealFields
  const isRotting = deal.is_rotting === true && deal.status === 'open' && stillInSameEntry(deal)
  return {
    isRotting,
    daysInStage: typeof deal.days_in_stage === 'number' ? deal.days_in_stage : null,
    limit: stageRottingDays(deal.stage),
  }
}

export function parseRottingDays(text: string): number | null | undefined {
  const trimmed = text.trim()
  if (!trimmed) return null
  const value = Number(trimmed)
  if (!Number.isInteger(value) || value < 1 || value > MAX_ROTTING_DAYS) return undefined
  return value
}
