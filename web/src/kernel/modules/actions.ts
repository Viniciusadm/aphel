import { useMemo } from 'react'
import type { CreateActionContribution } from './define'
import { useLabelTranslator } from './i18n'
import { useActiveContributions, usePermissionCheck } from './registry'

export interface ModuleCreateAction extends CreateActionContribution {
  key: string
  module: string
}

export function useModuleCreateActions(): ModuleCreateAction[] {
  const items = useActiveContributions<CreateActionContribution>('createActions')
  const allowed = usePermissionCheck()
  const translate = useLabelTranslator()
  return useMemo(
    () =>
      items
        .filter(({ value }) => allowed(value.permission))
        .map(({ module, value }) => ({
          ...value,
          key: `${module}.${value.id}`,
          module,
          label: translate(module, value.label),
          linkedLabel: value.linkedLabel ? translate(module, value.linkedLabel) : undefined,
        })),
    [items, allowed, translate],
  )
}
