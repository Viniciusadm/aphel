import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useOrganizationUsers } from '@/features/records/api'
import { countConditions, describeNode } from '@/features/records/filters'
import { useRecordFormatting } from '@/features/records/formatting'
import { isFilterGroup, type FieldSchema, type FilterNode } from '@/features/records/types'
import { parseDuration } from '../lib/durations'
import { useAutomationLabels } from '../lib/labels'
import type { AutomationCatalog, StepSpec } from '../types'
import { configHeadline } from './steps'

export function useDurationText() {
  const { t } = useTranslation('automations')
  return useCallback(
    (value: string | null | undefined, signed = false) => {
      const parsed = parseDuration(value)
      if (!parsed) return value ?? ''
      const amount = t(`durationText.${parsed.unit}`, { count: parsed.amount })
      if (!signed) return amount
      return parsed.sign === '-' ? t('durationText.before', { amount }) : t('durationText.after', { amount })
    },
    [t],
  )
}

export function useFilterText(schema: FieldSchema | undefined) {
  const { t } = useTranslation('automations')
  const { t: tableT } = useTranslation('records-table')
  const formatting = useRecordFormatting()
  const users = useOrganizationUsers()
  return useCallback(
    (filter: FilterNode | null | undefined) => {
      if (!filter || countConditions(filter) === 0) return t('summary.noConditions')
      if (!schema) return t('summary.conditions', { count: countConditions(filter) })
      const nodes = isFilterGroup(filter) ? filter.conditions : [filter]
      const joiner = isFilterGroup(filter) && filter.op === 'or' ? t('summary.or') : t('summary.and')
      const parts = nodes.map((node) => {
        if ('related' in node) {
          const relation =
            node.related.relationship === 'activities' ? t('conditions.activitiesRelation').toLowerCase() : node.related.relationship
          return node.related.none ? t('summary.relatedNone', { relation }) : t('summary.relatedExists', { relation })
        }
        return describeNode(node, { schema, t: tableT, formatting, users: users.data ?? [] })
      })
      return parts.join(` ${joiner} `)
    },
    [formatting, schema, t, tableT, users.data],
  )
}

export function useStepSummary(schema: FieldSchema | undefined, catalog: AutomationCatalog | undefined) {
  const { t } = useTranslation('automations')
  const labels = useAutomationLabels(catalog)
  const durationText = useDurationText()
  const filterText = useFilterText(schema)
  const fieldLabel = useCallback(
    (key: string | null | undefined) => schema?.fields.find((field) => field.key === key)?.label ?? key ?? '',
    [schema],
  )

  return useCallback(
    (step: StepSpec): { title: string; detail: string | null } => {
      const custom = step.label?.trim() || null
      switch (step.type) {
        case 'condition':
          return { title: custom ?? t('steps.condition'), detail: filterText(step.filter) }
        case 'branch':
          return {
            title: custom ?? t('steps.branch'),
            detail: t('summary.paths', { count: (step.cases?.length ?? 0) + 1 }),
          }
        case 'action': {
          const action = step.action ? labels.actionLabel(step.action) : t('steps.action')
          return { title: custom ?? action, detail: custom ? action : configHeadline(step.config) }
        }
        case 'wait': {
          const base = step.until_field
            ? t('summary.waitUntilField', {
                field: fieldLabel(step.until_field),
                offset: step.offset ? durationText(step.offset, true) : t('summary.onTheDate'),
              })
            : t('summary.waitFor', { duration: durationText(step.duration) })
          return {
            title: custom ?? t('steps.wait'),
            detail: step.recheck && countConditions(step.recheck) > 0 ? `${base} · ${t('summary.recheck')}` : base,
          }
        }
        case 'wait_until':
          return {
            title: custom ?? t('steps.wait_until'),
            detail: `${filterText(step.filter)} · ${t('summary.timeout', { duration: durationText(step.timeout) })}`,
          }
      }
    },
    [durationText, fieldLabel, filterText, labels, t],
  )
}
