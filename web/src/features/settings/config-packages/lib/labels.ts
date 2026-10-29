import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useModuleConfigPackageLabels } from '@/kernel/modules/config-packages'

const SECTIONS = [
  'fields',
  'pipelines',
  'lost_reasons',
  'activity_types',
  'deal_member_roles',
  'currencies',
  'tags',
  'layouts',
  'views',
  'dashboard',
] as const
const KINDS = [
  'field',
  'pipeline',
  'stage',
  'lost_reason',
  'activity_type',
  'deal_member_role',
  'currency',
  'tag',
  'layout',
  'view',
  'dashboard',
  'dashboard_layout',
] as const
const PROPERTIES = [
  'label',
  'type',
  'required',
  'unique',
  'searchable',
  'active',
  'enabled',
  'config',
  'name',
  'stages',
  'stage_order',
  'kind',
  'probability',
  'rotting_days',
  'entry_required_fields',
  'exit_required_fields',
  'icon',
  'color',
  'record_types',
  'layout',
  'view_type',
  'filter',
  'sort',
  'columns',
  'settings',
  'default',
  'widgets',
  'period',
] as const

type Section = (typeof SECTIONS)[number]
type Kind = (typeof KINDS)[number]
type Property = (typeof PROPERTIES)[number]

const OPTION_PREFIX = 'options.'

function includes<T extends string>(list: readonly T[], value: string): value is T {
  return (list as readonly string[]).includes(value)
}

export function humanizeKey(key: string): string {
  const words = key.replace(/[._-]+/g, ' ').trim()
  return words ? words.charAt(0).toUpperCase() + words.slice(1) : key
}

export function usePackageLabels() {
  const { t } = useTranslation('config-packages')
  const contributed = useModuleConfigPackageLabels()
  return useMemo(
    () => ({
      section: (key: string) =>
        includes<Section>(SECTIONS, key) ? t(`sections.${key}`) : (contributed.sections[key] ?? humanizeKey(key)),
      kind: (kind: string) =>
        includes<Kind>(KINDS, kind) ? t(`kinds.${kind}`) : (contributed.kinds[kind] ?? humanizeKey(kind)),
      property: (property: string) => {
        if (property.startsWith(OPTION_PREFIX)) return t('properties.option', { value: property.slice(OPTION_PREFIX.length) })
        if (includes<Property>(PROPERTIES, property)) return t(`properties.${property}`)
        return contributed.properties[property] ?? humanizeKey(property)
      },
    }),
    [t, contributed],
  )
}
