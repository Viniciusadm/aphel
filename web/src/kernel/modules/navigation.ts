import type { LucideIcon } from 'lucide-react'
import { useMemo, type ComponentType } from 'react'
import type { NavContribution, NavSectionContribution } from './define'
import { NO_FILTER_CONTEXT, useFilters } from './filters'
import { useLabelTranslator } from './i18n'
import { pluginMenus, type PluginMenu } from './wasm_runtime'
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
  return useMemo(
    () =>
      sortContributions(items)
        .filter(({ value }, index, all) => all.findIndex((other) => other.value.key === value.key) === index)
        .map(({ module, value }) => ({ key: value.key, label: translate(module, value.label) })),
    [items, translate],
  )
}

export const SETTINGS_ENTRY_KEY = 'settings'

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
  let result = entries.map((entry) => {
    const menu = menus.get(entry.module)
    return menu?.flat && entry.group ? { ...entry, group: undefined } : entry
  })
  let moved = false
  for (const [, menu] of [...menus].sort(([left], [right]) => left.localeCompare(right))) {
    for (const rule of menu.rules) {
      if (rule.target === SETTINGS_ENTRY_KEY) continue
      if (rule.action === 'hide') {
        result = result.filter((entry) => entry.key !== rule.target)
      } else if (rule.order != null) {
        const order = rule.order
        result = result.map((entry) => (entry.key === rule.target ? { ...entry, order } : entry))
        moved = true
      }
    }
  }
  return moved ? result.toSorted((left, right) => left.order - right.order) : result
}

export function useNavigationEntries(): NavigationEntry[] {
  const items = useActiveContributions<NavContribution>('nav')
  const allowed = usePermissionCheck()
  const translate = useLabelTranslator()
  const settingsAccess = useAllowedSettingsAreas(useAdminAreas()).length > 0
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
        section: value.section,
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
  }, [items, allowed, translate, settingsAccess])
  const arranged = useMemo(() => applyPluginMenus(entries, pluginMenus()), [entries])
  return useFilters('navigation.items', arranged, NO_FILTER_CONTEXT)
}
