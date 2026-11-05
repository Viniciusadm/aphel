import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { activeFields, type FieldSchema } from '@/sdk'
import type { AutomationCatalog } from '../types'

export type VariableGroup = 'record' | 'trigger' | 'automation' | 'run' | 'dates'

export const VARIABLE_GROUPS: VariableGroup[] = ['record', 'trigger', 'dates', 'automation', 'run']

export interface TemplateVariable {
  path: string
  label: string
  group: VariableGroup
}

const ROOT_GROUPS: Record<string, VariableGroup> = {
  record: 'record',
  trigger: 'trigger',
  automation: 'automation',
  run: 'run',
  now: 'dates',
  today: 'dates',
}

const RELATED_PATHS: Record<string, { path: string; field: string }[]> = {
  deal: [
    { path: 'record.stage.name', field: 'stage_id' },
    { path: 'record.contact.name', field: 'contact_id' },
    { path: 'record.company.name', field: 'company_id' },
  ],
  contact: [{ path: 'record.company.name', field: 'company_id' }],
}

export function useTemplateVariables(
  recordType: string,
  schema: FieldSchema | undefined,
  catalog: AutomationCatalog | undefined,
): TemplateVariable[] {
  const { t } = useTranslation('automations')
  return useMemo(() => {
    const roots = new Set(catalog?.template_roots ?? Object.keys(ROOT_GROUPS))
    const triggerPaths = new Set(catalog?.trigger_paths ?? [])
    const variables: TemplateVariable[] = []
    const fields = schema ? activeFields(schema).filter((field) => field.access !== 'hidden') : []
    const labelOf = (key: string) => fields.find((field) => field.key === key)?.label ?? key

    if (roots.has('record')) {
      variables.push({ path: 'record.id', label: t('variables.recordId'), group: 'record' })
      variables.push({ path: 'record.title', label: t('variables.recordTitle'), group: 'record' })
      for (const field of fields) {
        variables.push({
          path: field.is_system ? `record.${field.key}` : `record.custom.${field.key}`,
          label: field.label,
          group: 'record',
        })
      }
      if (fields.some((field) => field.key === 'owner_id')) {
        variables.push({ path: 'record.owner.display_name', label: t('variables.ownerName'), group: 'record' })
      }
      for (const related of RELATED_PATHS[recordType] ?? []) {
        if (fields.some((field) => field.key === related.field)) {
          variables.push({
            path: related.path,
            label: t('variables.relatedName', { field: labelOf(related.field) }),
            group: 'record',
          })
        }
      }
    }

    if (roots.has('trigger')) {
      const triggerVariables: [string, string][] = [
        ['trigger.type', t('variables.triggerType')],
        ['trigger.event_type', t('variables.triggerEventType')],
        ['trigger.occurred_at', t('variables.triggerOccurredAt')],
        ['trigger.actor.id', t('variables.triggerActorId')],
        ['trigger.actor.type', t('variables.triggerActorType')],
        ['trigger.changed_fields', t('variables.triggerChangedFields')],
        ['trigger.payload', t('variables.triggerPayload')],
      ]
      for (const [path, label] of triggerVariables) {
        const head = path.split('.')[1] ?? ''
        if (triggerPaths.size === 0 || triggerPaths.has(head)) variables.push({ path, label, group: 'trigger' })
      }
      if (triggerPaths.size === 0 || triggerPaths.has('previous')) {
        for (const field of fields.filter((candidate) => candidate.is_system)) {
          variables.push({
            path: `trigger.previous.${field.key}`,
            label: t('variables.previous', { field: field.label }),
            group: 'trigger',
          })
        }
      }
    }

    if (roots.has('now')) {
      variables.push({ path: 'now', label: t('variables.now'), group: 'dates' })
      variables.push({ path: 'now + 1h', label: t('variables.nowPlusHour'), group: 'dates' })
      variables.push({ path: 'now + 2d', label: t('variables.nowPlusDays', { count: 2 }), group: 'dates' })
    }
    if (roots.has('today')) {
      variables.push({ path: 'today', label: t('variables.today'), group: 'dates' })
      variables.push({ path: 'today + 1d', label: t('variables.tomorrow'), group: 'dates' })
      variables.push({ path: 'today + 1w', label: t('variables.nextWeek'), group: 'dates' })
    }
    if (roots.has('automation')) {
      variables.push({ path: 'automation.name', label: t('variables.automationName'), group: 'automation' })
      variables.push({ path: 'automation.key', label: t('variables.automationKey'), group: 'automation' })
      variables.push({ path: 'automation.id', label: t('variables.automationId'), group: 'automation' })
    }
    if (roots.has('run')) {
      variables.push({ path: 'run.id', label: t('variables.runId'), group: 'run' })
      variables.push({ path: 'run.depth', label: t('variables.runDepth'), group: 'run' })
    }
    const seen = new Set<string>()
    return variables.filter((variable) => !seen.has(variable.path) && Boolean(seen.add(variable.path)))
  }, [catalog, recordType, schema, t])
}

function normalize(text: string): string {
  return text
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase()
}

export function filterVariables(variables: TemplateVariable[], query: string): TemplateVariable[] {
  const needle = normalize(query.trim())
  if (!needle) return variables
  return variables.filter(
    (variable) => normalize(variable.path).includes(needle) || normalize(variable.label).includes(needle),
  )
}

export const TOKEN_BEFORE_CARET = /\{\{\s*([\w.+\- ]*)$/
