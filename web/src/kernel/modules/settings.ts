import { Cable, Database, Settings, UserRound, type LucideIcon } from 'lucide-react'
import { useMemo } from 'react'
import type {
  SettingsAreaContribution,
  SettingsAreaKey,
  SettingsContribution,
  SettingsGroupContribution,
} from './define'
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

export interface SettingsGroupDefinition {
  key: string
  label: string
}

export interface SettingsGroupEntry {
  area: SettingsAreaKey
  group: SettingsGroupContribution
}

export interface SettingsArea {
  key: SettingsAreaKey
  basePath: string
  titleKey: string
  descriptionKey: string
  icon: LucideIcon
  scope: 'account' | 'admin'
  primary: boolean
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

function toArea(contribution: SettingsAreaContribution): SettingsArea {
  return {
    key: contribution.key,
    basePath: contribution.basePath,
    titleKey: contribution.titleKey,
    descriptionKey: contribution.descriptionKey,
    icon: contribution.icon,
    scope: contribution.scope ?? 'admin',
    primary: contribution.primary ?? false,
  }
}

export const ACCOUNT_AREA: SettingsAreaContribution = {
  key: 'account',
  basePath: '/account',
  titleKey: 'areas.account.title',
  descriptionKey: 'areas.account.description',
  icon: UserRound,
  scope: 'account',
  order: 10,
  groups: [
    { key: 'profile', label: 'groups.profile' },
    { key: 'communication', label: 'groups.communication' },
    { key: 'accountSecurity', label: 'groups.accountSecurity' },
  ],
}

export const ORGANIZATION_AREA: SettingsAreaContribution = {
  key: 'organization',
  basePath: '/settings',
  titleKey: 'areas.organization.title',
  descriptionKey: 'areas.organization.description',
  icon: Settings,
  primary: true,
  order: 20,
  groups: [
    { key: 'organization', label: 'groups.organization' },
    { key: 'records', label: 'groups.records' },
    { key: 'communication', label: 'groups.communication', order: 30 },
  ],
}

export const INTEGRATIONS_AREA: SettingsAreaContribution = {
  key: 'integrations',
  basePath: '/integrations',
  titleKey: 'areas.integrations.title',
  descriptionKey: 'areas.integrations.description',
  icon: Cable,
  order: 30,
  groups: [
    { key: 'connections', label: 'groups.connections' },
    { key: 'webhooks', label: 'groups.webhooks' },
  ],
}

export const DATA_AREA: SettingsAreaContribution = {
  key: 'data',
  basePath: '/data',
  titleKey: 'areas.data.title',
  descriptionKey: 'areas.data.description',
  icon: Database,
  order: 40,
  groups: [
    { key: 'transfer', label: 'groups.transfer' },
    { key: 'compliance', label: 'groups.compliance' },
  ],
}

export const PLATFORM_SETTINGS_AREAS: SettingsAreaContribution[] = [
  ACCOUNT_AREA,
  ORGANIZATION_AREA,
  INTEGRATIONS_AREA,
  DATA_AREA,
]

const SETTINGS_NAMESPACE = 'settings'

export function settingsAreas(): SettingsArea[] {
  return sortContributions(contributions<SettingsAreaContribution>('settingsAreas'))
    .map(({ value }) => toArea(value))
    .filter((area, index, all) => all.findIndex((other) => other.key === area.key) === index)
}

export function useSettingsAreas(scope?: SettingsArea['scope']): SettingsArea[] {
  const items = useActiveContributions<SettingsAreaContribution>('settingsAreas')
  return useMemo(
    () =>
      items
        .map(({ value }) => toArea(value))
        .filter((area, index, all) => all.findIndex((other) => other.key === area.key) === index)
        .filter((area) => scope === undefined || area.scope === scope),
    [items, scope],
  )
}

export function useAdminAreas(): SettingsArea[] {
  return useSettingsAreas('admin')
}

function groupDefinitions(areaKey: SettingsAreaKey, areas: Contribution<SettingsAreaContribution>[], extra: Contribution<SettingsGroupEntry>[]): SettingsGroupDefinition[] {
  const declared = areas
    .filter(({ value }) => value.key === areaKey)
    .flatMap(({ value }) => (value.groups ?? []).map((group, index) => ({ group, order: group.order ?? (index + 1) * 10 })))
  const added = extra
    .filter(({ value }) => value.area === areaKey)
    .map(({ value, order }) => ({ group: value.group, order: value.group.order ?? order }))
  return [...declared, ...added]
    .sort((left, right) => left.order - right.order)
    .filter(({ group }, index, all) => all.findIndex((other) => other.group.key === group.key) === index)
    .map(({ group }) => ({ key: group.key, label: group.label }))
}

export function settingsEntries(area: SettingsAreaKey): Contribution<SettingsEntry>[] {
  return sortContributions(contributions<SettingsEntry>('settings').filter((entry) => entry.value.area === area))
}

function menuGroups(
  area: SettingsArea,
  definitions: SettingsGroupDefinition[],
  entries: Contribution<SettingsEntry>[],
  allowed: (permission: string | undefined) => boolean,
): SettingsMenuGroup[] {
  const groups = new Map<string, SettingsMenuGroup>(
    definitions.map((group) => [group.key, { key: group.key, label: group.label, items: [] }]),
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
  const areaItems = useActiveContributions<SettingsAreaContribution>('settingsAreas')
  const groupItems = useActiveContributions<SettingsGroupEntry>('settingsGroups')
  const allowed = usePermissionCheck()
  const current = useModuleState()
  const translate = useLabelTranslator()
  return useMemo(
    () =>
      areas.map((area) =>
        applyFilters(
          'settings.sections',
          menuGroups(area, groupDefinitions(area.key, areaItems, groupItems), entries, allowed),
          { area: area.key },
          current,
        )
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
    [areas, areaItems, groupItems, entries, allowed, current, translate],
  )
}

export function useAllowedSettingsAreas(areas: SettingsArea[]): SettingsArea[] {
  const groups = useSettingsGroups(areas)
  return useMemo(() => areas.filter((_, index) => groups[index].length > 0), [areas, groups])
}
