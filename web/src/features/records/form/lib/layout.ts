import type { components } from '@/api'
import { valueFieldType } from '../../fields'
import type { FieldDefinition, FieldType } from '../../types'
import type { ConditionContext } from './conditions'

export type LayoutContext = components['schemas']['LayoutContext']
export type LayoutField = components['schemas']['LayoutField']
export type LayoutColumn = components['schemas']['LayoutColumn']
export type LayoutSection = components['schemas']['LayoutSection']
export type LayoutTab = components['schemas']['LayoutTab']
export type FormLayout = components['schemas']['FormLayout']
export type FormLayoutResponse = components['schemas']['FormLayoutResponse']

export const LAYOUT_CONTEXTS: LayoutContext[] = ['create', 'edit', 'view']
export const MAX_LAYOUT_COLUMNS = 3
export const MAX_LAYOUT_TABS = 10
export const MAX_LAYOUT_SECTIONS = 20
export const MAX_LAYOUT_LABEL_LENGTH = 80

export interface ResolvedLayoutField {
  field: FieldDefinition
  rule: LayoutField
}

export interface ResolvedLayoutColumn {
  fields: ResolvedLayoutField[]
}

export interface ResolvedLayoutSection {
  id: string
  label: string
  columns: ResolvedLayoutColumn[]
}

export interface ResolvedLayoutTab {
  id: string
  label: string
  sections: ResolvedLayoutSection[]
}

export interface ResolvedLayout {
  tabs: ResolvedLayoutTab[]
}

export function layoutFields(layout: FormLayout): LayoutField[] {
  return layout.tabs.flatMap((tab) => tab.sections.flatMap((section) => section.columns.flatMap((column) => column.fields)))
}

export function layoutKeys(layout: FormLayout): string[] {
  return layoutFields(layout).map((field) => field.key)
}

export function defaultFormLayout(fields: FieldDefinition[], columns = 1): FormLayout {
  const count = Math.min(Math.max(columns, 1), MAX_LAYOUT_COLUMNS)
  const buckets: LayoutColumn[] = Array.from({ length: count }, () => ({ fields: [] }))
  fields.forEach((field, index) => buckets[index % count].fields.push({ key: field.key }))
  return { tabs: [{ id: 'main', label: '', sections: [{ id: 'main', label: '', columns: buckets }] }] }
}

export function resolveLayout(
  layout: FormLayout,
  candidates: FieldDefinition[],
  { appendRequired = false }: { appendRequired?: boolean } = {},
): ResolvedLayout {
  const byKey = new Map(candidates.map((field) => [field.key, field]))
  const seen = new Set<string>()
  const tabs: ResolvedLayoutTab[] = layout.tabs.map((tab) => ({
    id: tab.id,
    label: tab.label ?? '',
    sections: tab.sections.map((section) => ({
      id: section.id,
      label: section.label ?? '',
      columns: section.columns.map((column) => ({
        fields: column.fields.flatMap((rule) => {
          const field = byKey.get(rule.key)
          if (!field || seen.has(rule.key)) return []
          seen.add(rule.key)
          return [{ field, rule: field.is_required ? { ...rule, visible_when: null } : rule }]
        }),
      })),
    })),
  }))
  if (appendRequired) {
    const missing = candidates.filter((field) => field.is_required && !seen.has(field.key))
    const target = tabs[0]?.sections[0]?.columns[0]
    if (target) target.fields.push(...missing.map((field) => ({ field, rule: { key: field.key } })))
    else if (missing.length > 0) {
      tabs.push({
        id: 'main',
        label: '',
        sections: [{ id: 'main', label: '', columns: [{ fields: missing.map((field) => ({ field, rule: { key: field.key } })) }] }],
      })
    }
  }
  return { tabs }
}

export function resolvedFields(layout: ResolvedLayout): ResolvedLayoutField[] {
  return layout.tabs.flatMap((tab) => tab.sections.flatMap((section) => section.columns.flatMap((column) => column.fields)))
}

export function fieldTypes(fields: FieldDefinition[]): Record<string, FieldType> {
  return Object.fromEntries(fields.filter((field) => field.is_active).map((field) => [field.key, valueFieldType(field)]))
}

export function buildConditionContext(
  fields: FieldDefinition[],
  { timeZone, userId, now = new Date() }: { timeZone: string; userId: string; now?: Date },
): ConditionContext {
  return { now, timeZone, userId, fieldTypes: fieldTypes(fields) }
}

export function hasRules(layout: FormLayout): boolean {
  return layoutFields(layout).some((field) => field.visible_when || field.required_when)
}
