import { activeFields, valueFieldType } from '../fields'
import {
  isFilterCondition,
  isFilterGroup,
  isFilterRelated,
  type FieldDefinition,
  type FieldSchema,
  type FieldType,
  type FilterCondition,
  type FilterGroup,
  type FilterNode,
  type FilterOperator,
  type FilterRelated,
  type LogicalOperator,
  type OperandKind,
} from '../types'

export const MAX_FILTER_DEPTH = 4
export const MAX_FILTER_CONDITIONS = 50
export const MAX_LIST_VALUES = 100
export const MAX_RELATIVE_DAYS = 3650
export const MAX_EDITOR_DEPTH = 2
export const CURRENT_USER_VALUE = 'me'

export interface DraftCondition {
  kind: 'condition'
  id: string
  field: string
  operator: FilterOperator | null
  value: unknown
  original?: FilterCondition
}

export interface DraftGroup {
  kind: 'group'
  id: string
  op: LogicalOperator
  children: DraftNode[]
}

export interface DraftRelated {
  kind: 'related'
  id: string
  relationship: string
  none: boolean
  filter: DraftGroup
  schema: FieldSchema | null
  original: FilterRelated | null
}

export type DraftNode = DraftCondition | DraftGroup | DraftRelated

let draftSequence = 0

export function nextDraftId(): string {
  draftSequence += 1
  return `filter-node-${draftSequence}`
}

const TEXT_TYPES: ReadonlySet<FieldType> = new Set(['text', 'long_text', 'url', 'email', 'phone'])
const NUMERIC_TYPES: ReadonlySet<FieldType> = new Set(['number', 'currency'])

export function filterableFields(schema: FieldSchema): FieldDefinition[] {
  return activeFields(schema).filter((field) => field.is_filterable && field.operators.length > 0)
}

export function findFilterableField(schema: FieldSchema, key: string): FieldDefinition | undefined {
  return filterableFields(schema).find((field) => field.key === key)
}

export function operandOf(schema: FieldSchema, operator: FilterOperator): OperandKind {
  return schema.operators.find((definition) => definition.operator === operator)?.operand ?? 'single'
}

export function defaultOperator(field: FieldDefinition): FilterOperator | null {
  if (TEXT_TYPES.has(valueFieldType(field)) && field.operators.includes('contains')) return 'contains'
  return field.operators[0] ?? null
}

export function initialValue(field: FieldDefinition | undefined, operand: OperandKind | null): unknown {
  switch (operand) {
    case 'list':
      return []
    case 'pair':
      return [null, null]
    case 'single':
      return field && valueFieldType(field) === 'checkbox' ? true : null
    default:
      return null
  }
}

export function emptyCondition(): DraftCondition {
  return { kind: 'condition', id: nextDraftId(), field: '', operator: null, value: null }
}

export function emptyGroup(op: LogicalOperator = 'and', children: DraftNode[] = []): DraftGroup {
  return { kind: 'group', id: nextDraftId(), op, children }
}

export function emptyRelated(relationship = ''): DraftRelated {
  return {
    kind: 'related',
    id: nextDraftId(),
    relationship,
    none: false,
    filter: emptyGroup('and', [emptyCondition()]),
    schema: null,
    original: null,
  }
}

export function withField(condition: DraftCondition, field: FieldDefinition, schema: FieldSchema): DraftCondition {
  const operator = defaultOperator(field)
  return {
    ...condition,
    field: field.key,
    operator,
    value: initialValue(field, operator ? operandOf(schema, operator) : null),
  }
}

function firstValue(value: unknown): unknown {
  if (!Array.isArray(value)) return value
  return value.find((item) => !isBlank(item)) ?? null
}

export function adaptValue(
  value: unknown,
  from: OperandKind | null,
  to: OperandKind,
  field: FieldDefinition | undefined,
): unknown {
  if (from === to) return value
  switch (to) {
    case 'none':
      return null
    case 'days':
      return null
    case 'single': {
      if (from === 'list' || from === 'pair') {
        const first = firstValue(value)
        return isBlank(first) ? initialValue(field, to) : first
      }
      return initialValue(field, to)
    }
    case 'list': {
      if (from === 'single') return isBlank(value) ? [] : [value]
      if (from === 'pair' && Array.isArray(value)) return value.filter((item) => !isBlank(item))
      return []
    }
    case 'pair': {
      if (from === 'single' && !isBlank(value)) return [value, null]
      if (from === 'list') return [firstValue(value), null]
      return [null, null]
    }
  }
}

export function withOperator(
  condition: DraftCondition,
  operator: FilterOperator,
  schema: FieldSchema,
): DraftCondition {
  const field = schema.fields.find((candidate) => candidate.key === condition.field)
  const from = condition.operator ? operandOf(schema, condition.operator) : null
  const to = operandOf(schema, operator)
  return { ...condition, operator, value: adaptValue(condition.value, from, to, field) }
}

export function toDraft(node: FilterNode | null | undefined): DraftGroup {
  if (!node) return emptyGroup()
  if (isFilterGroup(node)) return groupToDraft(node)
  if (isFilterCondition(node)) return emptyGroup('and', [conditionToDraft(node)])
  if (isFilterRelated(node)) return emptyGroup('and', [relatedToDraft(node)])
  return emptyGroup()
}

function groupToDraft(group: FilterGroup): DraftGroup {
  return emptyGroup(
    group.op,
    group.conditions.flatMap((child): DraftNode[] => {
      if (isFilterGroup(child)) return [groupToDraft(child)]
      if (isFilterCondition(child)) return [conditionToDraft(child)]
      if (isFilterRelated(child)) return [relatedToDraft(child)]
      return []
    }),
  )
}

function conditionToDraft(condition: FilterCondition): DraftCondition {
  return {
    kind: 'condition',
    id: nextDraftId(),
    field: condition.field,
    operator: condition.operator,
    value: condition.value ?? null,
    original: condition,
  }
}

function relatedToDraft(node: FilterRelated): DraftRelated {
  return {
    kind: 'related',
    id: nextDraftId(),
    relationship: node.related.relationship,
    none: node.related.none === true,
    filter: toDraft(node.related.filter),
    schema: null,
    original: node,
  }
}

function untouchedOriginal(condition: DraftCondition): FilterCondition | null {
  const original = condition.original
  if (!original) return null
  const same =
    original.field === condition.field &&
    original.operator === condition.operator &&
    (original.value ?? null) === condition.value
  return same ? original : null
}

function relatedNode(relationship: string, filter: FilterNode, none: boolean): FilterRelated {
  return { related: none ? { relationship, filter, none: true } : { relationship, filter } }
}

export function serializeRelated(node: DraftRelated): FilterRelated | null {
  if (!node.relationship) return null
  if (node.schema) {
    const inner = serializeFilter(node.filter, node.schema)
    return inner ? relatedNode(node.relationship, inner, node.none) : null
  }
  const original = node.original?.related
  if (!original || original.relationship !== node.relationship) return null
  return relatedNode(original.relationship, original.filter, node.none)
}

function isBlank(value: unknown): boolean {
  if (value === null || value === undefined) return true
  if (typeof value === 'string') return value.trim() === ''
  if (typeof value === 'number') return !Number.isFinite(value)
  return false
}

function normalizeScalar(field: FieldDefinition, value: unknown): unknown {
  if (isBlank(value)) return undefined
  const fieldType = valueFieldType(field)
  if (fieldType === 'checkbox') return typeof value === 'boolean' ? value : undefined
  if (NUMERIC_TYPES.has(fieldType)) {
    const number = typeof value === 'number' ? value : Number(String(value).trim())
    return Number.isFinite(number) ? number : undefined
  }
  if (typeof value === 'string') return value.trim()
  if (typeof value === 'number') return value
  return undefined
}

function normalizeDays(value: unknown): number | undefined {
  if (isBlank(value)) return undefined
  const number = typeof value === 'number' ? value : Number(String(value).trim())
  if (!Number.isInteger(number) || number < 0 || number > MAX_RELATIVE_DAYS) return undefined
  return number
}

export function normalizeConditionValue(
  field: FieldDefinition,
  operand: OperandKind,
  value: unknown,
): { complete: boolean; value?: unknown } {
  switch (operand) {
    case 'none':
      return { complete: true }
    case 'days': {
      const days = normalizeDays(value)
      return days === undefined ? { complete: false } : { complete: true, value: days }
    }
    case 'single': {
      const normalized = normalizeScalar(field, value)
      return normalized === undefined ? { complete: false } : { complete: true, value: normalized }
    }
    case 'pair': {
      if (!Array.isArray(value) || value.length !== 2) return { complete: false }
      const low = normalizeScalar(field, value[0])
      const high = normalizeScalar(field, value[1])
      return low === undefined || high === undefined ? { complete: false } : { complete: true, value: [low, high] }
    }
    case 'list': {
      if (!Array.isArray(value)) return { complete: false }
      const items: unknown[] = []
      for (const item of value) {
        const normalized = normalizeScalar(field, item)
        if (normalized !== undefined && !items.includes(normalized)) items.push(normalized)
      }
      return items.length === 0 ? { complete: false } : { complete: true, value: items.slice(0, MAX_LIST_VALUES) }
    }
  }
}

export function serializeCondition(condition: DraftCondition, schema: FieldSchema): FilterCondition | null {
  if (!condition.operator) return null
  const field = findFilterableField(schema, condition.field)
  if (!field) return untouchedOriginal(condition)
  if (!field.operators.includes(condition.operator)) return null
  const result = normalizeConditionValue(field, operandOf(schema, condition.operator), condition.value)
  if (!result.complete) return null
  return result.value === undefined
    ? { field: field.key, operator: condition.operator }
    : { field: field.key, operator: condition.operator, value: result.value }
}

export function serializeFilter(root: DraftGroup, schema: FieldSchema): FilterNode | null {
  let remaining = MAX_FILTER_CONDITIONS

  const visit = (node: DraftNode, depth: number): FilterNode | null => {
    if (node.kind === 'condition') {
      if (remaining <= 0) return null
      const condition = serializeCondition(node, schema)
      if (condition) remaining -= 1
      return condition
    }
    if (node.kind === 'related') {
      if (remaining <= 0) return null
      const related = serializeRelated(node)
      if (related) remaining -= countConditions(related)
      return related
    }
    if (depth > MAX_FILTER_DEPTH) return null
    const conditions = node.children
      .map((child) => visit(child, depth + 1))
      .filter((child): child is FilterNode => child !== null)
    return conditions.length === 0 ? null : { op: node.op, conditions }
  }

  return visit(root, 1)
}

export function countConditions(node: FilterNode | DraftNode | null | undefined): number {
  if (!node) return 0
  if ('kind' in node) {
    if (node.kind === 'condition') return 1
    if (node.kind === 'related') return countConditions(node.filter)
    return node.children.reduce((total, child) => total + countConditions(child), 0)
  }
  if (isFilterCondition(node)) return 1
  if (isFilterRelated(node)) return countConditions(node.related.filter)
  return node.conditions.reduce((total, child) => total + countConditions(child), 0)
}

export function topLevelNodes(filter: FilterNode | null | undefined): FilterNode[] {
  if (!filter) return []
  return isFilterGroup(filter) ? filter.conditions : [filter]
}

export function removeTopLevelNode(filter: FilterNode | null | undefined, index: number): FilterNode | null {
  if (!filter || !isFilterGroup(filter)) return null
  const conditions = filter.conditions.filter((_, position) => position !== index)
  return conditions.length === 0 ? null : { ...filter, conditions }
}

export function updateDraftNode(root: DraftGroup, id: string, update: (node: DraftNode) => DraftNode): DraftGroup {
  const visit = (node: DraftNode): DraftNode => {
    if (node.id === id) return update(node)
    if (node.kind !== 'group') return node
    return { ...node, children: node.children.map(visit) }
  }
  return visit(root) as DraftGroup
}

export function removeDraftNode(root: DraftGroup, id: string): DraftGroup {
  const visit = (group: DraftGroup): DraftGroup => ({
    ...group,
    children: group.children
      .filter((child) => child.id !== id)
      .map((child) => (child.kind === 'group' ? visit(child) : child)),
  })
  return visit(root)
}

export function appendDraftNode(root: DraftGroup, groupId: string, child: DraftNode): DraftGroup {
  return updateDraftNode(root, groupId, (node) =>
    node.kind === 'group' ? { ...node, children: [...node.children, child] } : node,
  )
}

export function filterKey(filter: FilterNode | null | undefined): string {
  return filter ? JSON.stringify(filter) : ''
}
