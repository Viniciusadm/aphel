export type FilterOperator =
  | 'eq'
  | 'neq'
  | 'gt'
  | 'gte'
  | 'lt'
  | 'lte'
  | 'in'
  | 'not_in'
  | 'contains'
  | 'not_contains'
  | 'starts_with'
  | 'between'
  | 'is_empty'
  | 'is_not_empty'
  | 'today'
  | 'yesterday'
  | 'tomorrow'
  | 'this_week'
  | 'last_week'
  | 'next_week'
  | 'this_month'
  | 'last_month'
  | 'next_month'
  | 'this_year'
  | 'within_last_days'
  | 'within_next_days'
  | 'older_than_days'
  | 'in_saved_filter'
  | (string & {})

export type FilterScalar = string | number | boolean | Date | null

export type FilterValue = FilterScalar | readonly FilterScalar[]

export type FilterConditions = Record<string, FilterValue | Partial<Record<FilterOperator, FilterValue>>>

export interface FilterGroup {
  op: 'and' | 'or'
  conditions: FilterNode[]
}

export interface FilterCondition {
  field: string
  op: FilterOperator
  value?: unknown
}

export type FilterNode = FilterGroup | FilterCondition

const VALUELESS_OPERATORS: ReadonlySet<string> = new Set([
  'is_empty',
  'is_not_empty',
  'today',
  'yesterday',
  'tomorrow',
  'this_week',
  'last_week',
  'next_week',
  'this_month',
  'last_month',
  'next_month',
  'this_year',
])

function scalar(value: FilterScalar): string {
  if (value === null) return ''
  if (value instanceof Date) return value.toISOString()
  return String(value)
}

function encodeValue(value: FilterValue): string {
  return Array.isArray(value) ? value.map((item: FilterScalar) => scalar(item)).join(',') : scalar(value as FilterScalar)
}

function isOperatorMap(value: unknown): value is Partial<Record<FilterOperator, FilterValue>> {
  return typeof value === 'object' && value !== null && !Array.isArray(value) && !(value instanceof Date)
}

export function buildFilterQuery(conditions: FilterConditions): Array<[string, string]> {
  const entries: Array<[string, string]> = []
  for (const [field, condition] of Object.entries(conditions)) {
    if (isOperatorMap(condition)) {
      for (const [operator, value] of Object.entries(condition)) {
        if (value === undefined) continue
        const encoded = VALUELESS_OPERATORS.has(operator) ? '' : encodeValue(value)
        entries.push([`filter[${field}][${operator}]`, encoded])
      }
    } else {
      entries.push([`filter[${field}][eq]`, encodeValue(condition)])
    }
  }
  return entries
}

export function serializeFilterTree(filter: FilterNode): string {
  return JSON.stringify(filter)
}
