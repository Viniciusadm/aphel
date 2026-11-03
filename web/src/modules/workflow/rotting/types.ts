export interface RottingStageFields { rotting_days?: number | null }
export interface RottingRecordFields {
  stage_entered_at?: string | null
  days_in_stage?: number | null
  is_rotting?: boolean | null
  rotting_at?: string | null
  stage?: RottingStageFields | null
}
export interface RottingInfo { isRotting: boolean; daysInStage: number | null; limit: number | null }
