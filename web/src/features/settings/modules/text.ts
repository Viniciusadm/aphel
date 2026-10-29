import type { ModuleInfo, ModuleSettingField } from '@/kernel/modules/api'
import { useModuleTranslation } from '@/kernel/modules/i18n'

export function useModuleText(module: ModuleInfo) {
  const translate = useModuleTranslation(module.key)
  const lookup = (key: string, fallback: string): string => {
    const value = translate(key)
    return value && value !== key ? value : fallback
  }
  return {
    name: lookup('name', module.name),
    description: lookup('description', module.description),
    fieldLabel: (field: ModuleSettingField) => lookup(`settings.${field.key}.label`, field.label),
    fieldHelp: (field: ModuleSettingField) => lookup(`settings.${field.key}.help`, field.help ?? ''),
    optionLabel: (field: ModuleSettingField, value: string, fallback: string) =>
      lookup(`settings.${field.key}.options.${value}`, fallback),
  }
}
