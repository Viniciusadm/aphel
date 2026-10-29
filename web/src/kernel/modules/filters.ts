import { useMemo } from 'react'
import type { PaletteCommand } from '@/features/command-palette/types'
import type { RecordTab } from '@/features/records/detail/types'
import type { FormLayout } from '@/features/records/form/lib/layout'
import type { RecordFormMode } from '@/features/records/form/RecordForm'
import type { FieldDefinition, RecordItem } from '@/features/records/types'
import type { FilterFunction, SettingsAreaKey } from './define'
import type { NavigationEntry } from './navigation'
import { activeContributions, moduleState, useModuleState, type ModuleState } from './registry'
import type { SettingsMenuGroup } from './settings'

export interface FilterMap {
  'records.columns': { value: FieldDefinition[]; context: { recordType: string } }
  'records.formLayout': { value: FormLayout | null; context: { recordType?: string; mode: RecordFormMode } }
  'records.detailTabs': { value: RecordTab[]; context: { recordType: string; record: RecordItem; readOnly: boolean } }
  'navigation.items': { value: NavigationEntry[]; context: Record<string, never> }
  commands: { value: PaletteCommand[]; context: Record<string, never> }
  'settings.sections': { value: SettingsMenuGroup[]; context: { area: SettingsAreaKey } }
}

export const NO_FILTER_CONTEXT: Record<string, never> = {}

export type FilterName = keyof FilterMap

export type FilterValue<N extends FilterName> = FilterMap[N]['value']

export type FilterContext<N extends FilterName> = FilterMap[N]['context']

export interface FilterEntry<N extends FilterName> {
  fn: FilterFunction<N>
}

export function applyFilters<N extends FilterName>(
  name: N,
  value: FilterValue<N>,
  context: FilterContext<N>,
  current: ModuleState = moduleState(),
): FilterValue<N> {
  return activeContributions<FilterEntry<N>>(`filter:${name}`, current).reduce(
    (result, { value: entry }) => entry.fn(result, context),
    value,
  )
}

export function useFilters<N extends FilterName>(name: N, value: FilterValue<N>, context: FilterContext<N>): FilterValue<N> {
  const current = useModuleState()
  return useMemo(() => applyFilters(name, value, context, current), [name, value, context, current])
}
