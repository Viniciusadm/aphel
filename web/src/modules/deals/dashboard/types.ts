interface UserRef { id: string; display_name: string }
export interface ClosedDealsTotals {
  won_count: number
  won_amount?: string
  lost_count: number
  conversion_rate: number | null
}

export interface SalesIndicatorsData extends ClosedDealsTotals {
  currency: string
  open_count: number
  pipeline_amount?: string
  forecast_amount?: string
  previous: ClosedDealsTotals | null
}

export interface StageBar {
  id: string
  name: string
  kind: 'open' | 'won' | 'lost'
  probability: number
  count: number
  amount?: string
}

export interface DealsByStageData {
  pipeline: { id: string; name: string } | null
  currency: string
  stages: StageBar[]
}
export interface StaleLineItem {
  id: string
  title: string | null
  amount: string | null
  currency: string
  stage_name: string | null
  owner: UserRef | null
  last_touched_at: string
}

