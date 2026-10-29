import type { ReactNode } from 'react'

export type SettingsField =
  | { kind: 'integer'; key: string; label: string; min: number; max: number }
  | { kind: 'boolean'; key: string; label: string }

export interface WidgetRenderer {
  render: (data: unknown) => ReactNode
  isEmpty?: (data: unknown) => boolean
  settings?: SettingsField[]
}

export interface WidgetRecordRef {
  record_type: string
  record_id: string
  title: string | null
}
