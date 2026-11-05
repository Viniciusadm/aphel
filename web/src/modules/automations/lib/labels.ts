import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useCustomObjects } from '@/sdk'
import type { ActionCatalogEntry, AutomationCatalog, TriggerCatalogEntry } from '../types'

export const CORE_RECORD_TYPES = ['people.person', 'organizations.organization', 'deals.deal', 'tasks.task'] as const

type Translate = (key: string, options?: Record<string, unknown>) => string

function keyOf(value: string): string {
  return value.replace(/[.:]/g, '_')
}

function humanize(value: string): string {
  const text = value.replace(/_id$/, '').replace(/[_.]/g, ' ').trim()
  return text ? text.charAt(0).toUpperCase() + text.slice(1) : value
}

export function useAutomationLabels(catalog?: AutomationCatalog) {
  const { t } = useTranslation('automations')
  const objects = useCustomObjects()
  const translate = t as unknown as Translate

  const recordTypeLabel = useCallback(
    (recordType: string) => {
      if ((CORE_RECORD_TYPES as readonly string[]).includes(recordType)) {
        return translate(`recordTypes.${recordType}`)
      }
      return objects.data?.find((object) => object.record_type === recordType)?.label_singular ?? recordType
    },
    [objects.data, translate],
  )

  const triggerLabel = useCallback(
    (key: string, entry?: TriggerCatalogEntry) => {
      const fallback = entry?.label ?? catalog?.triggers.find((item) => item.key === key)?.label ?? humanize(key)
      return translate(`triggers.${keyOf(key)}.label`, { defaultValue: fallback })
    },
    [catalog, translate],
  )

  const triggerDescription = useCallback(
    (key: string) =>
      translate(`triggers.${keyOf(key)}.description`, {
        defaultValue: catalog?.triggers.find((item) => item.key === key)?.description ?? '',
      }),
    [catalog, translate],
  )

  const actionLabel = useCallback(
    (key: string, entry?: ActionCatalogEntry) => {
      const fallback = entry?.label ?? catalog?.actions.find((item) => item.key === key)?.label ?? humanize(key)
      return translate(`actions.${keyOf(key)}.label`, { defaultValue: fallback })
    },
    [catalog, translate],
  )

  const actionDescription = useCallback(
    (key: string) =>
      translate(`actions.${keyOf(key)}.description`, {
        defaultValue: catalog?.actions.find((item) => item.key === key)?.description ?? '',
      }),
    [catalog, translate],
  )

  const propertyLabel = useCallback(
    (name: string, title?: string) =>
      translate(`properties.${keyOf(name)}`, { defaultValue: title ?? humanize(name) }),
    [translate],
  )

  const enumLabel = useCallback(
    (name: string, value: unknown) =>
      translate(`enums.${keyOf(name)}.${keyOf(String(value))}`, { defaultValue: String(value) }),
    [translate],
  )

  return { recordTypeLabel, triggerLabel, triggerDescription, actionLabel, actionDescription, propertyLabel, enumLabel }
}

export function automatableRecordTypes(catalog: AutomationCatalog | undefined, customTypes: string[]): string[] {
  const core = new Set<string>()
  let custom = false
  for (const trigger of catalog?.triggers ?? []) {
    for (const recordType of trigger.record_types) core.add(recordType)
    custom ||= trigger.custom_objects
  }
  const ordered = CORE_RECORD_TYPES.filter((recordType) => core.has(recordType)) as string[]
  for (const recordType of core) if (!ordered.includes(recordType)) ordered.push(recordType)
  return custom ? [...ordered, ...customTypes] : ordered
}

export function supportsRecordType(
  entry: Pick<TriggerCatalogEntry, 'record_types' | 'custom_objects'>,
  recordType: string,
): boolean {
  if (recordType.startsWith('custom.')) return entry.custom_objects
  return entry.record_types.includes(recordType)
}
