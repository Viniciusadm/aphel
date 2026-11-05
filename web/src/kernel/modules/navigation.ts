import type { LucideIcon } from 'lucide-react'
import { useMemo, type ComponentType } from 'react'
import type { NavContribution, NavSectionContribution } from './define'
import { NO_FILTER_CONTEXT, useFilters } from './filters'
import { useLabelTranslator } from './i18n'
import { pluginMenus, type PluginMenu } from './wasm_runtime'
import { useTranslation } from 'react-i18next'
import { blueprintLabel, useInstalledBlueprint, type BlueprintSection } from './blueprint'
import { sortContributions, useActiveContributions, usePermissionCheck } from './registry'
import { ORGANIZATION_AREA, useAdminAreas, useAllowedSettingsAreas } from './settings'

export interface NavigationEntry {
  key: string
  module: string
  to: string
  label: string
  icon: LucideIcon
  order: number
  end?: boolean
  locked?: boolean
  badge?: ComponentType<{ collapsed?: boolean }>
  group?: { key: string; label: string; icon: LucideIcon }
  section?: string
  mobile?: boolean
}

export const DEFAULT_NAV_SECTION = 'operations'
export const RECORDS_NAV_SECTION = 'records'
export const ADMINISTRATION_NAV_SECTION = 'administration'

export interface ResolvedNavSection {
  key: string
  label: string
}

export function useNavSections(): ResolvedNavSection[] {
  const items = useActiveContributions<NavSectionContribution>('navSections')
  const translate = useLabelTranslator()
  const blueprint = useInstalledBlueprint()
  const { i18n } = useTranslation()
  return useMemo(() => {
    const declared = sortContributions(items)
      .filter(({ value }, index, all) => all.findIndex((other) => other.value.key === value.key) === index)
      .map(({ module, value }) => ({ key: value.key, label: translate(module, value.label) }))
    const chosen = (blueprint.data?.sections ?? []).map(({ key, label }) => ({
      key,
      label: blueprintLabel(label, i18n.language),
    }))
    return [...chosen, ...declared.filter((section) => !chosen.some((other) => other.key === section.key))]
  }, [items, translate, blueprint.data, i18n.language])
}

export const SETTINGS_ENTRY_KEY = 'settings'

function sectionFor(module: string, declared: string | undefined, sections: BlueprintSection[] | undefined) {
  return sections?.find((section) => section.items.includes(module))?.key ?? declared
}

export function navigationKey(to: string): string {
  return to === '/' ? 'home' : to.replace(/^\/+/, '')
}

interface LabeledEntry {
  key: string
  label: string
  group?: { label: string }
}

export function qualifiedLabel(entry: LabeledEntry, entries: LabeledEntry[]): string {
  const ambiguous = entries.some((other) => other.key !== entry.key && other.label === entry.label)
  return ambiguous && entry.group ? `${entry.group.label} › ${entry.label}` : entry.label
}

export function applyPluginMenus(
  entries: NavigationEntry[],
  menus: ReadonlyMap<string, PluginMenu>,
): NavigationEntry[] {
  if (menus.size === 0) return entries
  return entries.map((entry) => {
    const menu = menus.get(entry.module)
    return menu?.flat && entry.group ? { ...entry, group: undefined } : entry
  })
}

export function useNavigationEntries(): NavigationEntry[] {
  const items = useActiveContributions<NavContribution>('nav')
  const allowed = usePermissionCheck()
  const translate = useLabelTranslator()
  const settingsAccess = useAllowedSettingsAreas(useAdminAreas()).length > 0
  const blueprint = useInstalledBlueprint()
  const entries = useMemo<NavigationEntry[]>(() => {
    const modules = items
      .filter(({ value }) => allowed(value.permission))
      .map(({ module, value, order }) => ({
        key: value.key ?? navigationKey(value.to),
        module,
        to: value.to,
        label: translate(module, value.label),
        icon: value.icon,
        order,
        end: value.end,
        badge: value.badge,
        section: sectionFor(module, value.section, blueprint.data?.sections),
        mobile: value.mobile,
        group: value.group && { ...value.group, label: translate(module, value.group.label) },
      }))
    if (!settingsAccess) return modules
    return [
      ...modules,
      {
        key: SETTINGS_ENTRY_KEY,
        module: 'kernel',
        to: ORGANIZATION_AREA.basePath,
        label: translate('layout', 'nav.settings'),
        icon: ORGANIZATION_AREA.icon,
        order: Number.MAX_SAFE_INTEGER,
        locked: true,
        section: ADMINISTRATION_NAV_SECTION,
      },
    ]
  }, [items, allowed, translate, settingsAccess, blueprint.data])
  const arranged = useMemo(() => applyPluginMenus(entries, pluginMenus()), [entries])
  return useFilters('navigation.items', arranged, NO_FILTER_CONTEXT)
}
