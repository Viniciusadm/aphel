export type RenewalOwnerMode = 'original' | 'none'

export interface RenewalSettings {
  pipeline_id: string
  configured: boolean
  is_enabled: boolean
  days_before: number
  target_pipeline_id: string
  target_stage_id: string
  title_prefix: string
  owner_mode: RenewalOwnerMode
  updated_by: string | null
  updated_at: string | null
}

export interface RenewalSettingsRequest {
  is_enabled?: boolean
  days_before?: number
  target_pipeline_id?: string
  target_stage_id?: string
  title_prefix?: string
  owner_mode?: RenewalOwnerMode
}

export interface RenewalsDueItem {
  id: string
  title: string
  amount: string | null
  currency: string
  stage_name: string
  owner: { id: string; display_name: string } | null
  contract_end?: string | null
}

export interface RenewalsDueData {
  currency: string
  days: number
  count: number
  mrr: string | null
  arr: string | null
  partial?: boolean
  items: RenewalsDueItem[]
}
