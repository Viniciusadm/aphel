import { createElement, useMemo } from 'react'
import type { WidgetRenderer } from '@/kernel/widgets/types'
import type { RecordTab } from '@/features/records/detail/types'
import type { TimelineRenderer } from '@/features/records/timeline/registry'
import type { RecordTabEntry } from './app'
import type { CommandContribution } from './define'
import { useLabelTranslator } from './i18n'
import { matchesRecordType } from './record-types'
import {
  activeContributions,
  contributions,
  isModuleActive,
  moduleState,
  sortContributions,
  useActiveContributions,
  useModuleState,
  usePermissionCheck,
  type ModuleState,
} from './registry'
import { useStableList } from './stable-list'

function last<T>(bucket: string, current: ModuleState): T | undefined {
  return activeContributions<T>(bucket, current).at(-1)?.value
}

export function moduleTimelineRenderer(kind: string, current: ModuleState = moduleState()): TimelineRenderer | undefined {
  return last<TimelineRenderer>(`timeline:${kind}`, current)
}

export function moduleEventRenderer(eventType: string, current: ModuleState = moduleState()): TimelineRenderer | undefined {
  return last<TimelineRenderer>(`event:${eventType}`, current)
}

export function moduleWidgetRenderer(widgetKey: string, current: ModuleState = moduleState()): WidgetRenderer | undefined {
  return last<WidgetRenderer>(`widget:${widgetKey}`, current)
}

export function moduleWidgetTitleKey(widgetKey: string, current: ModuleState = moduleState()): string | undefined {
  const module = activeContributions<WidgetRenderer>(`widget:${widgetKey}`, current).at(-1)?.module
  if (!module) return undefined
  const localKey = widgetKey.startsWith(`${module}.`) ? widgetKey.slice(module.length + 1) : widgetKey
  return `${module}:widgets.${localKey}.title`
}

export interface ModuleCommand extends CommandContribution {
  module: string
}

export function useModuleCommands(): ModuleCommand[] {
  const items = useActiveContributions<CommandContribution>('commands')
  const allowed = usePermissionCheck()
  const translate = useLabelTranslator()
  return useMemo(
    () =>
      items
        .filter(({ value }) => allowed(value.permission))
        .map(({ module, value }) => ({ ...value, module, label: translate(module, value.label) })),
    [items, allowed, translate],
  )
}

export function useModuleRecordTabs(recordType: string, recordId: string, readOnly: boolean): RecordTab[] {
  const current = useModuleState()
  const allowed = usePermissionCheck()
  const translate = useLabelTranslator()
  const entries = sortContributions(contributions<RecordTabEntry>('recordTabs'))
  const visible = useStableList(
    entries.map(({ module, value: { recordTypes, tab } }) => {
      const enabled = isModuleActive(module, current) && matchesRecordType(recordTypes, recordType) && allowed(tab.permission)
      const shown = tab.useVisible?.({ recordType, recordId, readOnly, enabled }) ?? true
      return enabled && shown
    }),
  )
  return useMemo(
    () =>
      sortContributions(contributions<RecordTabEntry>('recordTabs'))
        .filter((_, index) => visible[index])
        .map(({ module, value: { tab } }) => ({
          key: tab.key,
          source: { module, key: tab.key },
          permission: tab.permission,
          label: translate(module, tab.label),
          icon: tab.icon,
          group: tab.group,
          summary: tab.summary ? createElement(tab.summary, { recordType, recordId, readOnly }) : undefined,
          content: createElement(tab.render, { recordType, recordId, readOnly }),
        })),
    [visible, translate, recordType, recordId, readOnly],
  )
}

export const useModuleRecordSections = useModuleRecordTabs
