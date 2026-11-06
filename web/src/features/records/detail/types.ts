import type { LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import type { PermissionRequirement } from '@/kernel/modules/define'

export interface RecordSectionSource {
  module: string
  key: string
}

export interface RecordTab {
  source?: RecordSectionSource
  permission?: PermissionRequirement
  key: string
  label: string
  summary?: ReactNode
  group?: string
  content: ReactNode
  icon?: LucideIcon
}

export type RecordTabsConfig = RecordTab[] | ((defaults: RecordTab[]) => RecordTab[])

export const DEFAULT_RECORD_TAB_KEYS = ['timeline', 'tasks', 'notes', 'files', 'related'] as const

export type DefaultRecordTabKey = (typeof DEFAULT_RECORD_TAB_KEYS)[number]

export const DEFAULT_READ_ONLY_FIELD_KEYS = ['id', 'created_at', 'updated_at', 'archived_at', 'deleted_at']

export function resolveRecordTabs(config: RecordTabsConfig | undefined, defaults: RecordTab[]): RecordTab[] {
  if (!config) return defaults
  return typeof config === 'function' ? config(defaults) : config
}
