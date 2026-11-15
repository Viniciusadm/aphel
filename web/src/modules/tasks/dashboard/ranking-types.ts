export interface ActivityTypeCount {
  id: string | null
  name: string | null
  count: number
}

export interface RankingRow {
  position: number
  user_id: string | null
  name: string | null
  count: number
  types: ActivityTypeCount[]
}

export interface TeamActivityRankingData {
  by_type: boolean
  rows: RankingRow[]
  total: number
}

