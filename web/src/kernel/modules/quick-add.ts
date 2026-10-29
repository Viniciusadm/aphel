import { useMemo } from 'react'
import type { QuickAddContribution } from './define'
import { useLabelTranslator } from './i18n'
import { useActiveContributions, usePermissionCheck } from './registry'

export interface ModuleQuickAdd extends Omit<QuickAddContribution, 'title' | 'recordShortcut'> {
  id: string
  module: string
  title: string
  linksRecord: boolean
  recordShortcut?: string
}

export function useModuleQuickAdds(): ModuleQuickAdd[] {
  const items = useActiveContributions<QuickAddContribution>('quickAdd')
  const allowed = usePermissionCheck()
  const translate = useLabelTranslator()
  return useMemo(
    () =>
      items
        .filter(({ value }) => allowed(value.permission))
        .map(({ module, value }) => {
          const label = translate(module, value.label)
          return {
            ...value,
            id: `${module}.${value.key}`,
            module,
            label,
            title: value.title ? translate(module, value.title) : label,
            linksRecord: value.linksRecord ?? true,
            recordShortcut: value.recordShortcut ? translate(module, value.recordShortcut) : undefined,
          }
        }),
    [items, allowed, translate],
  )
}
