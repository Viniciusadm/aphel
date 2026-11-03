import type { RottingRecordFields, RottingInfo, RottingStageFields } from './types'

export const MAX_ROTTING_DAYS = 3650
export const IS_ROTTING_FIELD = 'is_rotting'
export const DAYS_IN_STAGE_FIELD = 'days_in_stage'
export const ROTTING_AT_FIELD = 'rotting_at'
export const ROTTING_READ_ONLY_KEYS = [DAYS_IN_STAGE_FIELD, IS_ROTTING_FIELD, ROTTING_AT_FIELD]

export function stageRottingDays(stage: object | null | undefined): number | null {
  const days = (stage as RottingStageFields | null | undefined)?.rotting_days
  return typeof days === 'number' ? days : null
}

function stillInSameEntry(record: RottingRecordFields): boolean {
  if (!record.rotting_at || !record.stage_entered_at) return false
  return Date.parse(record.rotting_at) > Date.parse(record.stage_entered_at)
}

export function rottingOf(value: object): RottingInfo {
  const record = value as RottingRecordFields
  const isRotting = record.is_rotting === true && stillInSameEntry(record)
  return {
    isRotting,
    daysInStage: typeof record.days_in_stage === 'number' ? record.days_in_stage : null,
    limit: stageRottingDays(record.stage),
  }
}

export function parseRottingDays(text: string): number | null | undefined {
  const trimmed = text.trim()
  if (!trimmed) return null
  const value = Number(trimmed)
  if (!Number.isInteger(value) || value < 1 || value > MAX_ROTTING_DAYS) return undefined
  return value
}
