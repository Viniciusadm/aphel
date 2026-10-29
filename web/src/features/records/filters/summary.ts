import type { TFunction } from 'i18next'
import { formatFieldText } from '../display/values'
import { valueFieldType } from '../fields'
import { formatDate, formatDateTime, type RecordFormatting } from '../formatting'
import {
  isFilterGroup,
  isFilterRelated,
  type FieldDefinition,
  type FieldSchema,
  type FieldType,
  type FilterCondition,
  type FilterNode,
  type FilterOperator,
  type UserReference,
} from '../types'
import { CURRENT_USER_VALUE, countConditions, operandOf } from './model'

type TableT = TFunction<'records-table'>

const DATE_LABELED: ReadonlySet<FilterOperator> = new Set(['eq', 'neq', 'gt', 'gte', 'lt', 'lte'])

export function operatorLabel(t: TableT, operator: FilterOperator, fieldType?: FieldType): string {
  if ((fieldType === 'date' || fieldType === 'datetime') && DATE_LABELED.has(operator)) {
    return t(`dateOperators.${operator as 'eq' | 'neq' | 'gt' | 'gte' | 'lt' | 'lte'}`)
  }
  return t(`operators.${operator}`)
}

export interface SummaryContext {
  schema: FieldSchema
  t: TableT
  formatting: RecordFormatting
  users: UserReference[]
}

export function describeValue(field: FieldDefinition | undefined, value: unknown, context: SummaryContext): string {
  const { t, formatting, users } = context
  if (value === null || value === undefined) return ''
  if (!field) return String(value)
  switch (valueFieldType(field)) {
    case 'checkbox':
      return value === true ? t('filters.yes') : t('filters.no')
    case 'user':
      if (value === CURRENT_USER_VALUE) return t('filters.meValue')
      return users.find((user) => user.id === value)?.display_name ?? String(value)
    case 'date':
      return formatDate(value, formatting) || String(value)
    case 'datetime':
      return (formatDate(value, formatting) || formatDateTime(value, formatting)) || String(value)
    default:
      return formatFieldText(field, value, formatting) || String(value)
  }
}

export function describeCondition(condition: FilterCondition, context: SummaryContext): string {
  const { schema, t } = context
  const field = schema.fields.find((candidate) => candidate.key === condition.field)
  const label = field?.label ?? condition.field
  const operator = operatorLabel(t, condition.operator, field && valueFieldType(field))
  const value = condition.value
  switch (operandOf(schema, condition.operator)) {
    case 'none':
      return `${label} ${operator}`
    case 'days':
      return `${label} ${operator} ${t('filters.daysValue', { count: Number(value) || 0 })}`
    case 'pair': {
      const [from, to] = Array.isArray(value) ? value : [value, null]
      return `${label} ${operator} ${t('filters.between', {
        from: describeValue(field, from, context),
        to: describeValue(field, to, context),
      })}`
    }
    case 'list': {
      const items = Array.isArray(value) ? value : [value]
      return `${label} ${operator} ${items.map((item) => describeValue(field, item, context)).join(', ')}`
    }
    default:
      return `${label} ${operator} ${describeValue(field, value, context)}`
  }
}

export function describeNode(node: FilterNode, context: SummaryContext): string {
  if (isFilterGroup(node)) {
    if (node.conditions.length === 1) return describeNode(node.conditions[0], context)
    return context.t('filters.groupSummary', { count: countConditions(node) })
  }
  if (isFilterRelated(node)) return `${node.related.relationship} › ${describeNode(node.related.filter, context)}`
  return describeCondition(node, context)
}
