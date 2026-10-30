import type { components } from '@/api'

export type InstallerModule = components['schemas']['InstallerModuleResponse']
export type InstallerCatalog = components['schemas']['InstallerStatusResponse']
export type ModuleSettings = Record<string, Record<string, unknown>>

export interface ModuleChoice {
  modules: string[]
  module_settings: ModuleSettings
}

export const EMPTY_CHOICE: ModuleChoice = { modules: [], module_settings: {} }

export function dependencyClosure(catalog: InstallerModule[], selected: string[]): Set<string> {
  const keys = new Set([...selected, ...catalog.filter((item) => item.required).map((item) => item.key)])
  let previous = -1
  while (previous !== keys.size) {
    previous = keys.size
    for (const item of catalog) if (keys.has(item.key)) for (const key of item.dependencies) keys.add(key)
  }
  return keys
}

export function needsConfiguration(item: InstallerModule, catalog: InstallerModule[]): boolean {
  const closure = dependencyClosure(catalog, [item.key])
  return catalog.some(
    (module) => closure.has(module.key) && !module.required && module.settings_schema.some((field) => field.required),
  )
}

export function recommendedModules(catalog: InstallerCatalog, businessType: string | null): Set<string> {
  return new Set(catalog.profiles.find((profile) => profile.key === businessType)?.modules ?? [])
}

export function preselect(catalog: InstallerCatalog, businessType: string | null): ModuleChoice {
  const recommended = recommendedModules(catalog, businessType)
  return {
    modules: catalog.modules
      .filter(
        (item) =>
          !item.required && item.compatible && recommended.has(item.key) && !needsConfiguration(item, catalog.modules),
      )
      .map((item) => item.key),
    module_settings: {},
  }
}

export function withoutSecrets(catalog: InstallerCatalog | undefined, choice: ModuleChoice): ModuleChoice {
  if (!catalog) return { ...choice, module_settings: {} }
  const settings = Object.fromEntries(
    Object.entries(choice.module_settings).map(([key, values]) => {
      const secrets = new Set(
        catalog.modules
          .find((item) => item.key === key)
          ?.settings_schema.filter((field) => field.type === 'secret')
          .map((field) => field.key) ?? [],
      )
      return [key, Object.fromEntries(Object.entries(values).filter(([field]) => !secrets.has(field)))]
    }),
  )
  return { ...choice, module_settings: settings }
}
