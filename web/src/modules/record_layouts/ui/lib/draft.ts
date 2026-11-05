import { type FieldDefinition, type FilterNode, type FormLayout, isEditableField, type LayoutContext, type LayoutField, layoutKeys, type LayoutSection, MAX_LAYOUT_COLUMNS, READ_ONLY_SYSTEM_KEYS } from '@/sdk'
export interface FieldLocation {
  tabId: string
  sectionId: string
  column: number
  index: number
}

export interface DropTarget {
  sectionId: string
  column: number
  index?: number
}

export interface FieldRules {
  visible_when: FilterNode | null
  required_when: FilterNode | null
}

let sequence = 0

export function newLayoutId(prefix: string): string {
  sequence += 1
  return `${prefix}-${Date.now().toString(36)}${sequence.toString(36)}`
}

export function clone(layout: FormLayout): FormLayout {
  return structuredClone(layout)
}

export function layoutCandidates(fields: FieldDefinition[], context: LayoutContext): FieldDefinition[] {
  const active = fields.filter((field) => field.is_active).sort((left, right) => left.position - right.position)
  return context === 'view' ? active : active.filter((field) => isEditableField(field, READ_ONLY_SYSTEM_KEYS))
}

export function isLocked(field: FieldDefinition, context: LayoutContext): boolean {
  return context === 'create' && field.is_required
}

export function startingLayout(fields: FieldDefinition[], context: LayoutContext): FormLayout {
  const candidates = layoutCandidates(fields, context)
  const columns = context === 'view' ? 1 : 2
  const buckets = Array.from({ length: columns }, () => ({ fields: [] as LayoutField[] }))
  candidates.forEach((field, index) => buckets[index % columns].fields.push({ key: field.key }))
  return { tabs: [{ id: 'main', label: '', sections: [{ id: 'main', label: '', columns: buckets }] }] }
}

export function unplacedFields(layout: FormLayout, candidates: FieldDefinition[]): FieldDefinition[] {
  const placed = new Set(layoutKeys(layout))
  return candidates.filter((field) => !placed.has(field.key))
}

export function locate(layout: FormLayout, key: string): FieldLocation | null {
  for (const tab of layout.tabs) {
    for (const section of tab.sections) {
      for (const [column, entry] of section.columns.entries()) {
        const index = entry.fields.findIndex((field) => field.key === key)
        if (index >= 0) return { tabId: tab.id, sectionId: section.id, column, index }
      }
    }
  }
  return null
}

function findSection(layout: FormLayout, sectionId: string): LayoutSection | undefined {
  return layout.tabs.flatMap((tab) => tab.sections).find((section) => section.id === sectionId)
}

function detach(layout: FormLayout, key: string): LayoutField {
  const location = locate(layout, key)
  if (!location) return { key }
  const column = findSection(layout, location.sectionId)!.columns[location.column]
  const [field] = column.fields.splice(location.index, 1)
  return field
}

export function placeField(layout: FormLayout, key: string, target: DropTarget): FormLayout {
  const next = clone(layout)
  const field = detach(next, key)
  const column = findSection(next, target.sectionId)?.columns[target.column]
  if (!column) return layout
  const index = target.index ?? column.fields.length
  column.fields.splice(Math.max(0, Math.min(index, column.fields.length)), 0, field)
  return next
}

export function shiftField(layout: FormLayout, key: string, offset: -1 | 1): FormLayout {
  const location = locate(layout, key)
  if (!location) return layout
  const next = clone(layout)
  const fields = findSection(next, location.sectionId)!.columns[location.column].fields
  const target = location.index + offset
  if (target < 0 || target >= fields.length) return layout
  const [field] = fields.splice(location.index, 1)
  fields.splice(target, 0, field)
  return next
}

export function removeField(layout: FormLayout, key: string): FormLayout {
  const next = clone(layout)
  detach(next, key)
  return withoutDanglingRules(next)
}

export function setFieldRules(layout: FormLayout, key: string, rules: FieldRules): FormLayout {
  const next = clone(layout)
  const location = locate(next, key)
  if (!location) return layout
  const fields = findSection(next, location.sectionId)!.columns[location.column].fields
  const field: LayoutField = { key }
  if (rules.visible_when) field.visible_when = rules.visible_when
  if (rules.required_when) field.required_when = rules.required_when
  fields[location.index] = field
  return next
}

export function addTab(layout: FormLayout, label: string): FormLayout {
  const next = clone(layout)
  next.tabs.push({
    id: newLayoutId('tab'),
    label,
    sections: [{ id: newLayoutId('section'), label: '', columns: [{ fields: [] }] }],
  })
  return next
}

export function renameTab(layout: FormLayout, tabId: string, label: string): FormLayout {
  const next = clone(layout)
  const tab = next.tabs.find((item) => item.id === tabId)
  if (tab) tab.label = label
  return next
}

export function removeTab(layout: FormLayout, tabId: string): FormLayout {
  if (layout.tabs.length <= 1) return layout
  const next = clone(layout)
  next.tabs = next.tabs.filter((tab) => tab.id !== tabId)
  return withoutDanglingRules(next)
}

export function moveTab(layout: FormLayout, tabId: string, offset: -1 | 1): FormLayout {
  const index = layout.tabs.findIndex((tab) => tab.id === tabId)
  const target = index + offset
  if (index < 0 || target < 0 || target >= layout.tabs.length) return layout
  const next = clone(layout)
  const [tab] = next.tabs.splice(index, 1)
  next.tabs.splice(target, 0, tab)
  return next
}

export function addSection(layout: FormLayout, tabId: string, label: string): FormLayout {
  const next = clone(layout)
  next.tabs
    .find((tab) => tab.id === tabId)
    ?.sections.push({ id: newLayoutId('section'), label, columns: [{ fields: [] }] })
  return next
}

export function renameSection(layout: FormLayout, sectionId: string, label: string): FormLayout {
  const next = clone(layout)
  const section = findSection(next, sectionId)
  if (section) section.label = label
  return next
}

export function removeSection(layout: FormLayout, sectionId: string): FormLayout {
  const next = clone(layout)
  const tab = next.tabs.find((item) => item.sections.some((section) => section.id === sectionId))
  if (!tab || tab.sections.length <= 1) return layout
  tab.sections = tab.sections.filter((section) => section.id !== sectionId)
  return withoutDanglingRules(next)
}

export function moveSection(layout: FormLayout, sectionId: string, offset: -1 | 1): FormLayout {
  const next = clone(layout)
  const tab = next.tabs.find((item) => item.sections.some((section) => section.id === sectionId))
  if (!tab) return layout
  const index = tab.sections.findIndex((section) => section.id === sectionId)
  const target = index + offset
  if (target < 0 || target >= tab.sections.length) return layout
  const [section] = tab.sections.splice(index, 1)
  tab.sections.splice(target, 0, section)
  return next
}

export function setSectionColumns(layout: FormLayout, sectionId: string, count: number): FormLayout {
  const total = Math.min(Math.max(count, 1), MAX_LAYOUT_COLUMNS)
  const next = clone(layout)
  const section = findSection(next, sectionId)
  if (!section) return layout
  while (section.columns.length < total) section.columns.push({ fields: [] })
  if (section.columns.length > total) {
    const removed = section.columns.splice(total)
    section.columns[total - 1].fields.push(...removed.flatMap((column) => column.fields))
  }
  return next
}

function conditionKeys(node: FilterNode | null | undefined, keys: string[] = []): string[] {
  if (!node) return keys
  if ('op' in node && 'conditions' in node) node.conditions.forEach((child) => conditionKeys(child, keys))
  else if ('field' in node) keys.push(node.field)
  return keys
}

export function ruleReferences(rules: Pick<LayoutField, 'visible_when' | 'required_when'>): string[] {
  return [...conditionKeys(rules.visible_when), ...conditionKeys(rules.required_when)]
}

export function withoutDanglingRules(layout: FormLayout): FormLayout {
  const placed = new Set(layoutKeys(layout))
  for (const tab of layout.tabs) {
    for (const section of tab.sections) {
      for (const column of section.columns) {
        column.fields = column.fields.map((field) => {
          const next: LayoutField = { key: field.key }
          if (field.visible_when && conditionKeys(field.visible_when).every((key) => placed.has(key))) {
            next.visible_when = field.visible_when
          }
          if (field.required_when && conditionKeys(field.required_when).every((key) => placed.has(key))) {
            next.required_when = field.required_when
          }
          return next
        })
      }
    }
  }
  return layout
}

export function normalizeForSave(layout: FormLayout, context: LayoutContext): FormLayout {
  const next = clone(layout)
  for (const tab of next.tabs) {
    tab.label = tab.label?.trim() ?? ''
    for (const section of tab.sections) {
      section.label = section.label?.trim() ?? ''
      if (context === 'view') {
        for (const column of section.columns) {
          for (const field of column.fields) delete field.required_when
        }
      }
    }
  }
  return next
}

export function layoutsEqual(left: FormLayout | null, right: FormLayout | null): boolean {
  return JSON.stringify(left) === JSON.stringify(right)
}

export function fieldAtPath(layout: FormLayout, path: string): string | null {
  const match = /^layout\.tabs\.(\d+)\.sections\.(\d+)\.columns\.(\d+)\.fields\.(\d+)/.exec(path)
  if (!match) return null
  const [tab, section, column, field] = match.slice(1).map(Number)
  return layout.tabs[tab]?.sections[section]?.columns[column]?.fields[field]?.key ?? null
}
