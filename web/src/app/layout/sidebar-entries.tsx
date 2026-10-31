import type { ComponentType, ReactNode } from 'react'
import { ObjectIcon, customObjectPath, objectColor, useCustomObjects } from '@/features/custom-records'
import type { NavigationPreferences } from '@/features/settings/navigation/types'
import { NAV_ORDER } from '@/kernel/modules/define'
import { NO_FILTER_CONTEXT, useFilters } from '@/kernel/modules/filters'
import { useNavigationEntries, type NavigationEntry } from '@/kernel/modules/navigation'

export interface SidebarEntry {
  key: string
  to: string
  label: string
  icon: ReactNode
  end?: boolean
  locked?: boolean
  badge?: ComponentType<{ collapsed?: boolean }>
  group?: { key: string; label: string; icon: ReactNode }
}

const ICON_CLASS = 'size-4 shrink-0'

function sidebarEntry(entry: NavigationEntry): SidebarEntry {
  return {
    key: entry.key,
    to: entry.to,
    label: entry.label,
    icon: <entry.icon className={ICON_CLASS} aria-hidden />,
    end: entry.end,
    locked: entry.locked,
    badge: entry.badge,
    group: entry.group && { key: entry.group.key, label: entry.group.label, icon: <entry.group.icon className={ICON_CLASS} aria-hidden /> },
  }
}

export function useAvailableSidebarEntries(): SidebarEntry[] {
  const entries = useNavigationEntries()
  const objects = useCustomObjects()
  const visibleObjects = useFilters('navigation.customObjects', objects.data ?? [], NO_FILTER_CONTEXT)

  const regular = entries.filter((entry) => !entry.locked)
  const leading = regular.filter((entry) => entry.order < NAV_ORDER.customObjects).map(sidebarEntry)
  const trailing = regular.filter((entry) => entry.order >= NAV_ORDER.customObjects).map(sidebarEntry)
  const locked = entries.filter((entry) => entry.locked).map(sidebarEntry)

  const customObjects = visibleObjects.map(
    (object): SidebarEntry => ({
      key: `object:${object.key}`,
      to: customObjectPath(object.key),
      label: object.label_plural,
      icon: (
        <ObjectIcon icon={object.icon} className={ICON_CLASS} style={{ color: objectColor(object.color) }} aria-hidden />
      ),
    }),
  )

  return [...leading, ...customObjects, ...trailing, ...locked]
}

export function arrangeSidebarEntries(
  entries: SidebarEntry[],
  preferences: NavigationPreferences,
  includeHidden = false,
): SidebarEntry[] {
  const position = new Map(preferences.order.map((key, index) => [key, index]))
  const hidden = new Set(preferences.hidden)
  const ordered = entries
    .map((entry, index) => ({ entry, index }))
    .sort((left, right) => {
      const leftLocked = left.entry.locked === true
      const rightLocked = right.entry.locked === true
      if (leftLocked !== rightLocked) return leftLocked ? 1 : -1
      const leftPosition = position.get(left.entry.key)
      const rightPosition = position.get(right.entry.key)
      if (leftPosition !== undefined && rightPosition !== undefined) return leftPosition - rightPosition
      if (leftPosition !== undefined) return -1
      if (rightPosition !== undefined) return 1
      return left.index - right.index
    })
    .map(({ entry }) => entry)
  if (includeHidden) return ordered
  return ordered.filter((entry) => entry.locked || !hidden.has(entry.key))
}
