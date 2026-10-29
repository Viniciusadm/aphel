import type { FilterNode } from '@/features/records/types'
import { parseDuration } from '../lib/durations'
import { uniqueId } from '../lib/flow'
import type { ActionCatalogEntry, JsonObject, JsonSchema, StepSpec, StepType } from '../types'

export type NewStepKind = Exclude<StepType, 'action'> | { action: string }

export const EMPTY_GROUP: FilterNode = { op: 'and', conditions: [] }

function defaultsOf(schema: JsonSchema | undefined): JsonObject {
  const config: JsonObject = {}
  for (const [name, property] of Object.entries(schema?.properties ?? {})) {
    if (property.readOnly) continue
    if (property.default !== undefined) config[name] = property.default
    const required = schema?.required?.includes(name)
    const userSpec = property.oneOf?.some((option) => option.properties?.type?.const === 'record_owner')
    if (required && userSpec) config[name] = { type: 'record_owner' }
    if (required && property.type === 'array' && property.items?.oneOf?.some((option) => option.properties?.type?.const === 'record_owner')) {
      config[name] = [{ type: 'record_owner' }]
    }
  }
  return config
}

export function actionDefaults(entry: ActionCatalogEntry | undefined, recordHasOwner: boolean): JsonObject {
  const config = defaultsOf(entry?.config_schema)
  if (!recordHasOwner) {
    for (const [name, value] of Object.entries(config)) {
      if (typeof value === 'object' && value !== null && (value as JsonObject).type === 'record_owner') delete config[name]
      if (Array.isArray(value)) delete config[name]
    }
  }
  if (entry?.key === 'create_task' && config.due_in === undefined) config.due_in = '1d'
  return config
}

export function createStep(
  kind: NewStepKind,
  taken: Set<string>,
  options: { actions: ActionCatalogEntry[]; recordHasOwner: boolean; caseLabel: string },
): StepSpec {
  if (typeof kind === 'object') {
    const entry = options.actions.find((action) => action.key === kind.action)
    return {
      id: uniqueId(kind.action, taken),
      type: 'action',
      action: kind.action,
      config: actionDefaults(entry, options.recordHasOwner),
    }
  }
  switch (kind) {
    case 'condition':
      return { id: uniqueId('condicao', taken), type: 'condition', filter: EMPTY_GROUP }
    case 'branch':
      return {
        id: uniqueId('ramificacao', taken),
        type: 'branch',
        cases: [{ key: 'caso_1', label: options.caseLabel, filter: EMPTY_GROUP }],
      }
    case 'wait':
      return { id: uniqueId('esperar', taken), type: 'wait', duration: '1d' }
    case 'wait_until':
      return { id: uniqueId('esperar_ate', taken), type: 'wait_until', filter: EMPTY_GROUP, timeout: '7d' }
  }
}

export function nextCaseKey(step: StepSpec): string {
  const keys = new Set((step.cases ?? []).map((item) => item.key))
  for (let index = keys.size + 1; ; index += 1) {
    const key = `caso_${index}`
    if (!keys.has(key)) return key
  }
}

export function configHeadline(config: JsonObject | null | undefined): string | null {
  if (!config) return null
  for (const key of ['title', 'subject', 'body', 'url']) {
    const value = config[key]
    if (typeof value === 'string' && value.trim()) return value.trim()
  }
  return null
}

export function durationParts(value: string | null | undefined) {
  return parseDuration(value)
}
