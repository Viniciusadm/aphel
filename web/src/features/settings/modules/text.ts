import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import type { ModuleInfo, ModuleSettingField } from '@/kernel/modules/api'

type Translate = (key: string, options: Record<string, unknown>) => string

function useTranslate(): Translate {
  const { t } = useTranslation()
  return t as unknown as Translate
}

function lookupIn(translate: Translate, module: { key: string }, path: string, fallback: string, catalogPath?: string): string {
  if (catalogPath) {
    const shared = translate(catalogPath, { ns: 'modules', defaultValue: '' })
    if (shared) return shared
  }
  const own = translate(path, { ns: module.key, defaultValue: '' })
  return own || fallback
}

export function useModuleName(): (module: Pick<ModuleInfo, 'key' | 'name'>) => string {
  const translate = useTranslate()
  return useCallback(
    (module) => lookupIn(translate, module, 'name', module.name, `catalog.${module.key}.name`),
    [translate],
  )
}

export function useModuleText(module: ModuleInfo) {
  const translate = useTranslate()
  const lookup = (path: string, fallback: string, catalogPath?: string) =>
    lookupIn(translate, module, path, fallback, catalogPath)
  return {
    name: lookup('name', module.name, `catalog.${module.key}.name`),
    description: lookup('description', module.description, `catalog.${module.key}.description`),
    permissionLabel: (permission: string, fallback: string) => lookup(`permissions.${permission}`, fallback),
    eventDescription: (eventType: string, fallback: string) => lookup(`events.${eventType}`, fallback),
    fieldLabel: (field: ModuleSettingField) => lookup(`settings.${field.key}.label`, field.label),
    fieldHelp: (field: ModuleSettingField) => lookup(`settings.${field.key}.help`, field.help ?? ''),
    optionLabel: (field: ModuleSettingField, value: string, fallback: string) =>
      lookup(`settings.${field.key}.options.${value}`, fallback),
  }
}
