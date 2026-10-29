export interface SourceRow {
  source: string | null
  count: number
  amount: string | null
}

export interface DealsBySourceData {
  status: 'ok' | 'source_hidden'
  currency: string
  only_won: boolean
  amount_visible: boolean
  rows: SourceRow[]
  other: { count: number; amount: string | null } | null
  total: { count: number; amount: string | null }
}

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

export interface BirthdayItem {
  id: string
  name: string
  day: number
  is_today: boolean
}

export interface BirthdaysData {
  status: 'ok' | 'not_configured' | 'field_unavailable'
  month: number
  today: number
  field_label: string | null
  items: BirthdayItem[]
  total: number
  truncated: boolean
}
