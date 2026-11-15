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

