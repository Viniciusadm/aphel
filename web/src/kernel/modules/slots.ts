import { useMemo, type ComponentType } from 'react'
import type { RecordFormMode } from '@/features/records/form/RecordForm'
import type { BulkActionContext } from '@/features/records/table/RecordListView'
import type { FilterNode, RecordItem } from '@/features/records/types'
import type { SlotContribution } from './define'
import { matchesRecordType } from './record-types'
import { useActiveContributions, usePermissionCheck } from './registry'

export type ComposerContext = 'note' | 'email' | 'message'

export interface ComposerSubject {
  recordType: string
  recordId: string
}

export interface RecordSlotProps {
  recordType: string
  record: RecordItem
  readOnly: boolean
}

export interface SlotPropsMap {
  'record.detail.title': RecordSlotProps
  'record.detail.header': RecordSlotProps
  'record.detail.actions': RecordSlotProps
  'record.detail.sidebar': RecordSlotProps
  'record.fields.panel': RecordSlotProps
  'record.list.toolbar': {
    recordType: string
    filter: FilterNode | null
    applyFilter: (filter: FilterNode | null) => void
  }
  'record.list.actions': { recordType: string }
  'record.list.empty': { recordType: string }
  'record.list.viewActions': { recordType: string; viewId: string }
  'record.list.rowActions': { recordType: string; record: RecordItem }
  'record.list.bulkActions': BulkActionContext
  'record.form.sections': { recordType?: string; mode: RecordFormMode; record?: RecordItem }
  'composer.tools': {
    context: ComposerContext
    subject?: ComposerSubject
    value: string
    onChange: (value: string) => void
  }
  'user.menu': Record<string, never>
  'app.topbar': Record<string, never>
  'modules.settings': Record<string, never>
  'app.shell': Record<string, never>
  'app.banner': Record<string, never>
  'app.sidebar': { collapsed?: boolean; onNavigate?: () => void }
}

export type SlotName = keyof SlotPropsMap

export type SlotProps<N extends SlotName> = SlotPropsMap[N]

export interface SlotEntry<P> {
  key: string
  Component: ComponentType<P>
}

function recordTypeOf(props: object): string | undefined {
  const value = (props as { recordType?: unknown }).recordType
  return typeof value === 'string' ? value : undefined
}

export function useSlot<N extends SlotName>(name: N, props: SlotProps<N>): SlotEntry<SlotProps<N>>[] {
  return useSlotEntries(name, recordTypeOf(props))
}

export function useSlotEntries<N extends SlotName>(name: N, recordType?: string): SlotEntry<SlotProps<N>>[] {
  const items = useActiveContributions<SlotContribution<SlotProps<N>>>(`slot:${name}`)
  const allowed = usePermissionCheck()
  return useMemo(
    () =>
      items
        .filter(({ value }) => matchesRecordType(value.recordTypes, recordType))
        .filter(({ value }) => allowed(value.permission))
        .map(({ module, value, seq }) => ({ key: `${module}.${value.key ?? seq}`, Component: value.render })),
    [items, allowed, recordType],
  )
}
