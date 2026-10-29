import type { components } from '@/api'

type Schemas = components['schemas']

export type RottingStageFields = Pick<Schemas['DealStageResponse'], 'rotting_days'>

export type RottingDealFields = Partial<
  Pick<Schemas['DealResponse'], 'status' | 'stage_entered_at' | 'days_in_stage' | 'is_rotting' | 'rotting_at' | 'stage'>
>

export interface RottingInfo {
  isRotting: boolean
  daysInStage: number | null
  limit: number | null
}
