import type { LucideIcon } from 'lucide-react'
import { useMemo, type ComponentType } from 'react'
import type { NavContribution } from './define'
import { NO_FILTER_CONTEXT, useFilters } from './filters'
import { useLabelTranslator } from './i18n'
import { useActiveContributions, usePermissionCheck } from './registry'
import { ADMIN_AREAS, ORGANIZATION_AREA, useAllowedSettingsAreas } from './settings'

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
}

export const SETTINGS_ENTRY_KEY = 'settings'

export function navigationKey(to: string): string {
  return to === '/' ? 'home' : to.replace(/^\/+/, '')
}

export function useNavigationEntries(): NavigationEntry[] {
  const items = useActiveContributions<NavContribution>('nav')
  const allowed = usePermissionCheck()
  const translate = useLabelTranslator()
  const settingsAccess = useAllowedSettingsAreas(ADMIN_AREAS).length > 0
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
      },
    ]
  }, [items, allowed, translate, settingsAccess])
  return useFilters('navigation.items', entries, NO_FILTER_CONTEXT)
}
