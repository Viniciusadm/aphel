export interface BirthdayItem {
  id: string
  name: string
  day: number
  is_today: boolean
}

export interface BirthdaysData {
  status: 'ok' | 'field_unavailable'
  month: number
  today: number
  field_label: string | null
  items: BirthdayItem[]
  total: number
  truncated: boolean
}
