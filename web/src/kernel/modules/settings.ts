import type { ParseKeys } from 'i18next'
import { Cable, Database, Settings, UserRound, type LucideIcon } from 'lucide-react'
import { useMemo } from 'react'
import type { SettingsAreaKey, SettingsContribution } from './define'
import { applyFilters } from './filters'
import { useLabelTranslator } from './i18n'
import {
  contributions,
  sortContributions,
  useActiveContributions,
  useModuleState,
  usePermissionCheck,
  type Contribution,
} from './registry'

type SettingsKey = ParseKeys<'settings'>

export interface SettingsGroupDefinition {
  key: string
  labelKey: SettingsKey
}

export interface SettingsArea {
  key: SettingsAreaKey
  basePath: string
  titleKey: SettingsKey
  descriptionKey: SettingsKey
  icon: LucideIcon
  groups: SettingsGroupDefinition[]
}

export interface SettingsEntry {
  area: SettingsAreaKey
  item: SettingsContribution
}

export interface SettingsMenuItem {
  module: string
  path: string
  to: string
  label: string
  description: string
  icon: LucideIcon
  permission?: string
}

export interface SettingsMenuGroup {
  key: string
  label: string
  items: SettingsMenuItem[]
}

export interface ResolvedSettingsItem {
  to: string
  label: string
  description: string
  icon: LucideIcon
}

export interface ResolvedSettingsGroup {
  key: string
  label: string
  items: ResolvedSettingsItem[]
}

export const ACCOUNT_AREA: SettingsArea = {
  key: 'account',
  basePath: '/account',
  titleKey: 'areas.account.title',
  descriptionKey: 'areas.account.description',
  icon: UserRound,
  groups: [
    { key: 'profile', labelKey: 'groups.profile' },
    { key: 'communication', labelKey: 'groups.communication' },
    { key: 'accountSecurity', labelKey: 'groups.accountSecurity' },
  ],
}

export const ORGANIZATION_AREA: SettingsArea = {
  key: 'organization',
  basePath: '/settings',
  titleKey: 'areas.organization.title',
  descriptionKey: 'areas.organization.description',
  icon: Settings,
  groups: [
    { key: 'organization', labelKey: 'groups.organization' },
    { key: 'records', labelKey: 'groups.records' },
    { key: 'sales', labelKey: 'groups.sales' },
    { key: 'communication', labelKey: 'groups.communication' },
  ],
}

export const INTEGRATIONS_AREA: SettingsArea = {
  key: 'integrations',
  basePath: '/integrations',
  titleKey: 'areas.integrations.title',
  descriptionKey: 'areas.integrations.description',
  icon: Cable,
  groups: [
    { key: 'connections', labelKey: 'groups.connections' },
    { key: 'webhooks', labelKey: 'groups.webhooks' },
  ],
}

export const DATA_AREA: SettingsArea = {
  key: 'data',
  basePath: '/data',
  titleKey: 'areas.data.title',
  descriptionKey: 'areas.data.description',
  icon: Database,
  groups: [
    { key: 'transfer', labelKey: 'groups.transfer' },
    { key: 'compliance', labelKey: 'groups.compliance' },
  ],
}

export const SETTINGS_AREAS: SettingsArea[] = [ACCOUNT_AREA, ORGANIZATION_AREA, INTEGRATIONS_AREA, DATA_AREA]

export const ADMIN_AREAS: SettingsArea[] = [ORGANIZATION_AREA, INTEGRATIONS_AREA, DATA_AREA]

const SETTINGS_NAMESPACE = 'settings'

export function settingsArea(key: SettingsAreaKey): SettingsArea {
  return SETTINGS_AREAS.find((area) => area.key === key) ?? ORGANIZATION_AREA
}

export function settingsEntries(area: SettingsAreaKey): Contribution<SettingsEntry>[] {
  return sortContributions(contributions<SettingsEntry>('settings').filter((entry) => entry.value.area === area))
}

function menuGroups(
  area: SettingsArea,
  entries: Contribution<SettingsEntry>[],
  allowed: (permission: string | undefined) => boolean,
): SettingsMenuGroup[] {
  const groups = new Map<string, SettingsMenuGroup>(
    area.groups.map((group) => [group.key, { key: group.key, label: `${SETTINGS_NAMESPACE}:${group.labelKey}`, items: [] }]),
  )
  for (const { module, value } of entries) {
    if (value.area !== area.key || !allowed(value.item.permission)) continue
    const group = groups.get(value.item.group) ?? { key: value.item.group, label: `${module}:${value.item.group}`, items: [] }
    group.items.push({
      module,
      path: value.item.path,
      to: `${area.basePath}/${value.item.path}`,
      label: value.item.label,
      description: value.item.description,
      icon: value.item.icon,
      permission: value.item.permission,
    })
    groups.set(group.key, group)
  }
  return [...groups.values()]
}

export function useSettingsGroups(areas: SettingsArea[]): ResolvedSettingsGroup[][] {
  const entries = useActiveContributions<SettingsEntry>('settings')
  const allowed = usePermissionCheck()
  const current = useModuleState()
  const translate = useLabelTranslator()
  return useMemo(
    () =>
      areas.map((area) =>
        applyFilters('settings.sections', menuGroups(area, entries, allowed), { area: area.key }, current)
          .filter((group) => group.items.length > 0)
          .map((group) => ({
            key: `${area.key}.${group.key}`,
            label: translate(SETTINGS_NAMESPACE, group.label),
            items: group.items.map((item) => ({
              to: item.to,
              label: translate(item.module, item.label),
              description: translate(item.module, item.description),
              icon: item.icon,
            })),
          })),
      ),
    [areas, entries, allowed, current, translate],
  )
}

export function useAllowedSettingsAreas(areas: SettingsArea[]): SettingsArea[] {
  const groups = useSettingsGroups(areas)
  return useMemo(() => areas.filter((_, index) => groups[index].length > 0), [areas, groups])
}
